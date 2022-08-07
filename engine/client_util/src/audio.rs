// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::visibility::VisibilityEvent;
use js_sys::ArrayBuffer;
use sprite_sheet::AudioSprite;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, Event, GainNode, Response,
};

/// A macro-generated enum representing all audio sprites.
pub trait Audio: Eq + Hash + Copy + Debug + Sized + 'static {
    /// Returns path to the audio file containing all the audio.
    fn path() -> Cow<'static, str>;
    /// Gets the audio sprite corresponding to this audio id.
    fn sprite(self) -> AudioSprite;
}

/// Renders (plays) audio.
pub struct AudioPlayer<A: Audio> {
    inner: Rc<RefCell<Option<Inner<A>>>>,
}

struct Inner<A: Audio> {
    context: AudioContext,
    sfx_gain: GainNode,
    #[allow(dead_code)]
    music_gain: GainNode,
    track: Option<AudioBuffer>,
    playing: HashMap<A, Vec<AudioBufferSourceNode>>,
    /// What volume is or is ramping up/down to.
    volume_target: f32,
    /// The game wants to mute all audio.
    muted_by_game: bool,
    /// Whether muted because the page is unfocused.
    muted_by_visibility: bool,
    /// Whether muted due to conflicting with an advertisement's audio.
    muted_by_ad: bool,
    /// Volume (kept up to date with the corresponding setting.
    volume_setting: f32,
}

impl<A: Audio> Default for AudioPlayer<A> {
    fn default() -> Self {
        if let Ok(context) = web_sys::AudioContext::new() {
            if let Some((sfx_gain, music_gain)) = web_sys::GainNode::new(&context)
                .ok()
                .zip(web_sys::GainNode::new(&context).ok())
            {
                let _ = sfx_gain.connect_with_audio_node(&context.destination());
                let _ = music_gain.connect_with_audio_node(&context.destination());

                let inner = Rc::new(RefCell::new(Some(Inner {
                    context,
                    sfx_gain,
                    music_gain,
                    track: None,
                    playing: HashMap::new(),
                    muted_by_game: false,
                    muted_by_visibility: false,
                    muted_by_ad: false,
                    volume_target: 0.0,
                    volume_setting: 0.0,
                })));

                let promise = web_sys::window().unwrap().fetch_with_str(&A::path());
                let inner_clone = inner.clone();

                let _ = future_to_promise(async move {
                    let response: Response =
                        JsFuture::from(promise).await.unwrap().dyn_into().unwrap();
                    let array_buffer: ArrayBuffer =
                        JsFuture::from(response.array_buffer().unwrap())
                            .await
                            .unwrap()
                            .dyn_into()
                            .unwrap();

                    #[allow(must_not_suspend)]
                    let borrow = inner_clone.borrow();
                    if let Some(inner) = borrow.as_ref() {
                        // Note: Cannot yield while borrowing; otherwise will be borrowed elsewhere. Use a scope
                        // to drop the first borrow.
                        let promise =
                            JsFuture::from(inner.context.decode_audio_data(&array_buffer).unwrap());
                        drop(borrow);

                        match promise.await {
                            Ok(res) => {
                                let track = res.dyn_into().unwrap();

                                inner_clone.borrow_mut().as_mut().unwrap().track = Some(track);
                            }
                            Err(_) => *inner_clone.borrow_mut() = None,
                        }
                    }

                    Ok(JsValue::from_str("ok"))
                });

                return Self { inner };
            }
        };

        Self {
            inner: Rc::new(RefCell::new(None)),
        }
    }
}

impl<A: Audio> AudioPlayer<A> {
    /// Plays a particular sound once.
    pub fn play(&self, audio: A) {
        self.play_with_volume(audio, 1.0);
    }

    /// Plays a particular sound once, with a specified volume.
    pub fn play_with_volume(&self, audio: A, volume: f32) {
        Inner::play(&self.inner, audio, volume, false);
    }

    /// Plays a particular sound once, with a specified volume and delay in seconds.
    pub fn play_with_volume_and_delay(&self, audio: A, volume: f32, _delay: f32) {
        Inner::play(&self.inner, audio, volume, false);
    }

    /// Plays a particular sound in a loop.
    pub fn play_looping(&self, audio: A) {
        Inner::play(&self.inner, audio, 1.0, true);
    }

    pub fn is_playing(&self, audio: A) -> bool {
        self.inner
            .borrow_mut()
            .as_mut()
            .map(|inner| inner.is_playing(audio))
            .unwrap_or(false)
    }

    pub fn stop_playing(&self, audio: A) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.stop_playing(audio)
        }
    }

    // Sets a multiplier for the volume of all sounds.
    pub(crate) fn set_volume_setting(&self, volume_setting: f32) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.volume_setting = volume_setting;
            inner.update_volume();
        }
    }

    /// For the game to mute/unmute all audio.
    pub fn set_muted_by_game(&self, muted_by_game: bool) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.muted_by_game = muted_by_game;
            inner.update_volume();
        }
    }

    pub(crate) fn peek_visibility(&self, event: &VisibilityEvent) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.muted_by_visibility = match event {
                VisibilityEvent::Visible(visible) => !visible,
            };
            inner.update_volume();
        }
    }

    #[allow(unused)]
    pub(crate) fn set_muted_by_ad(&self, muted_by_ad: bool) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.muted_by_ad = muted_by_ad;
            inner.update_volume();
        }
    }
}

impl<A: Audio> Inner<A> {
    fn recalculate_volume(&self) -> f32 {
        if self.muted_by_game || self.muted_by_visibility || self.muted_by_ad {
            0.0
        } else {
            self.volume_setting
        }
    }

    fn update_volume(&mut self) {
        let new_volume = self.recalculate_volume();
        if new_volume != self.volume_target {
            self.volume_target = new_volume;
            if let Err(e) = self
                .sfx_gain
                .gain()
                .linear_ramp_to_value_at_time(new_volume, self.context.current_time() + 1.5)
            {
                crate::console_log!("could not linear ramp audio: {:?}", e);
                self.sfx_gain.gain().set_value(new_volume);
            }
        }
    }

    /// Plays a particular sound, optionally in a loop. This is private, since looping is never
    /// determined at runtime.
    fn play(rc: &Rc<RefCell<Option<Self>>>, audio: A, volume: f32, looping: bool) {
        if let Some(inner) = rc.borrow_mut().as_mut() {
            if inner.recalculate_volume() == 0.0 {
                return;
            }

            if inner.context.state() == AudioContextState::Suspended {
                let _ = inner.context.resume();
            } else if inner.track.is_some() {
                let track = inner.track.as_ref().unwrap();

                let sprite = audio.sprite();
                //crate::console_log!("playing {:?} at {} ({:?})", audio, volume, sprite);
                let source: AudioBufferSourceNode = inner
                    .context
                    .create_buffer_source()
                    .unwrap()
                    .dyn_into()
                    .unwrap();

                source.set_buffer(Some(track));

                let gain = web_sys::GainNode::new(&inner.context).unwrap();
                gain.gain().set_value(volume);
                let _ = source.connect_with_audio_node(&gain);

                let _ = gain.connect_with_audio_node(&inner.sfx_gain);

                if looping {
                    source.set_loop(true);
                    source.set_loop_start(sprite.loop_start.unwrap_or(sprite.start) as f64);
                    source.set_loop_end((sprite.start + sprite.duration) as f64);
                    let _ = source.start_with_when_and_grain_offset(0.0, sprite.start as f64);
                } else {
                    let _ = source.start_with_when_and_grain_offset_and_grain_duration(
                        0.0,
                        sprite.start as f64,
                        sprite.duration as f64,
                    );
                }

                let cloned_rc = Rc::clone(rc);
                let stop = Closure::once_into_js(move |value: JsValue| {
                    let event: Event = value.dyn_into().unwrap();
                    if let Some(inner) = cloned_rc.borrow_mut().as_mut() {
                        if let Some(playing) = inner.playing.get_mut(&audio) {
                            for source in playing.drain_filter(|p| {
                                *p == event
                                    .target()
                                    .unwrap()
                                    .dyn_into::<AudioBufferSourceNode>()
                                    .unwrap()
                            }) {
                                // Ensure no double-invocation.
                                source.set_onended(None);
                            }
                        }
                    }
                });

                source.set_onended(Some(stop.as_ref().unchecked_ref()));

                inner
                    .playing
                    .entry(audio)
                    .or_insert_with(Vec::new)
                    .push(source);
            }
        }
    }

    fn is_playing(&self, audio: A) -> bool {
        //crate::console_log!("{:?}", self.playing.iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>());
        self.playing
            .get(&audio)
            .map(|playing| !playing.is_empty())
            .unwrap_or(false)
    }

    fn stop_playing(&mut self, audio: A) {
        if let Some(playing) = self.playing.get_mut(&audio) {
            for removed in playing.drain(..) {
                // WebAudio bug makes unsetting loop required?
                removed.set_loop(false);
                let _ = removed.stop();
            }
        }
    }
}
