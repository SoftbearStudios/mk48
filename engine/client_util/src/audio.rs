// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::visibility::VisibilityEvent;
use js_sys::ArrayBuffer;
use sprite_sheet::AudioSprite;
use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, Event, GainNode, Response,
};

/// A macro-generated enum representing all audio sprites.
/// They each have an index associated with them to use as a key into a [`Vec`].
pub trait Audio: Copy + Debug + 'static {
    /// Returns the [`Audio`]'s unique identifier.
    fn index(self) -> usize;

    /// Returns path to the audio file containing all the audio.
    fn path() -> &'static str;

    /// Returns a static slice of [`AudioSprite`]s indexed by [`Audio::index`].
    fn sprites() -> &'static [AudioSprite];
}

/// Renders (plays) audio.
pub struct AudioPlayer<A: Audio> {
    inner: Rc<RefCell<Option<Inner<A>>>>,
}

struct Inner<A: Audio> {
    context: AudioContext,
    sfx_gain: GainNode,
    _music_gain: GainNode,
    track: Option<AudioBuffer>,
    /// Audio indexed by [`Audio::index`].
    playing: Box<[Vec<AudioBufferSourceNode>]>,
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
    spooky: PhantomData<A>,
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
                    _music_gain: music_gain,
                    track: None,
                    playing: vec![Vec::new(); std::mem::variant_count::<A>()].into_boxed_slice(),
                    muted_by_game: false,
                    muted_by_visibility: false,
                    muted_by_ad: false,
                    volume_target: 0.0,
                    volume_setting: 0.0,
                    spooky: PhantomData,
                })));

                let promise = js_hooks::window().fetch_with_str(&A::path());
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

    pub fn set_muted_by_ad(&self, muted_by_ad: bool) {
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
            if let Err(_e) = self
                .sfx_gain
                .gain()
                .linear_ramp_to_value_at_time(new_volume, self.context.current_time() + 1.5)
            {
                #[cfg(debug_assertions)]
                js_hooks::console_log!("could not linear ramp audio: {:?}", _e);
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

                let sprite = &A::sprites()[audio.index()];
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
                        let playing = &mut inner.playing[audio.index()];
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
                });

                source.set_onended(Some(stop.as_ref().unchecked_ref()));

                inner.playing[audio.index()].push(source);
            }
        }
    }

    fn is_playing(&self, audio: A) -> bool {
        !self.playing[audio.index()].is_empty()
    }

    fn stop_playing(&mut self, audio: A) {
        let playing = &mut self.playing[audio.index()];
        for removed in playing.drain(..) {
            // WebAudio bug makes unsetting loop required?
            removed.set_loop(false);
            let _ = removed.stop();
        }
    }
}
