// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::console_log;
use crate::renderer::renderer::{Layer, Renderer};
use js_sys::ArrayBuffer;
use sprite_sheet::AudioSpriteSheet;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{future_to_promise, JsFuture};
use web_sys::{
    AudioBuffer, AudioBufferSourceNode, AudioContext, AudioContextState, Event, GainNode, Response,
};

/// Renders (plays) audio. Doesn't render anything visual.
pub struct AudioLayer {
    inner: Rc<RefCell<Option<Inner>>>,
}

impl Layer for AudioLayer {
    /// Audio doesn't render anything visual.
    fn render(&mut self, _renderer: &Renderer) {}
}

struct Inner {
    context: AudioContext,
    sfx_gain: GainNode,
    #[allow(dead_code)]
    music_gain: GainNode,
    track: Option<AudioBuffer>,
    sprite_sheet: AudioSpriteSheet,
    playing: HashMap<String, Vec<AudioBufferSourceNode>>,
    muted: bool,
}

impl AudioLayer {
    /// Allocates a new AudioPlayer.
    pub fn new(path: &str, sprite_sheet: AudioSpriteSheet) -> Self {
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
                    sprite_sheet,
                    playing: HashMap::new(),
                    muted: false,
                })));

                let promise = web_sys::window().unwrap().fetch_with_str(path);
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

    /// Plays a particular sound once.
    pub fn play(&self, name: &'static str) {
        self.play_with_volume(name, 1.0);
    }

    /// Plays a particular sound once, with a specified volume.
    pub fn play_with_volume(&self, name: &'static str, volume: f32) {
        Inner::play(&self.inner, name, volume, false);
    }

    /// Plays a particular sound once, with a specified volume and delay in seconds.
    pub fn play_with_volume_and_delay(&self, name: &'static str, volume: f32, _delay: f32) {
        Inner::play(&self.inner, name, volume, false);
    }

    /// Plays a particular sound in a loop.
    pub fn play_looping(&self, name: &'static str) {
        Inner::play(&self.inner, name, 1.0, true);
    }

    pub fn is_playing(&self, name: &'static str) -> bool {
        self.inner
            .borrow_mut()
            .as_mut()
            .map(|inner| inner.is_playing(name))
            .unwrap_or(false)
    }

    pub fn stop_playing(&self, name: &'static str) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.stop_playing(name)
        }
    }

    // Sets a multiplier for the volume of all sounds.
    pub fn set_volume(&self, volume: f32) {
        if let Some(inner) = self.inner.borrow_mut().as_mut() {
            inner.sfx_gain.gain().set_value(volume);
            inner.muted = volume == 0.0;
        }
    }
}

impl Inner {
    /// Plays a particular sound, optionally in a loop. This is private, since looping is never
    /// determined at runtime.
    fn play(rc: &Rc<RefCell<Option<Self>>>, name: &'static str, volume: f32, looping: bool) {
        if let Some(inner) = rc.borrow_mut().as_mut() {
            if inner.muted {
                return;
            }

            if inner.context.state() == AudioContextState::Suspended {
                let _ = inner.context.resume();
            } else if inner.track.is_some() {
                let track = inner.track.as_ref().unwrap();

                if let Some(sprite) = inner.sprite_sheet.sprites.get(name) {
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
                        source.set_loop_start(sprite.start as f64);
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
                            if let Some(playing) = inner.playing.get_mut(name) {
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
                        .entry(String::from(name))
                        .or_insert_with(Vec::new)
                        .push(source);
                } else {
                    console_log!("warning: missing audio sprite {}", name);
                }

                // Don't re-add to queue.
            }
        }
    }

    fn is_playing(&self, name: &str) -> bool {
        //crate::console_log!("{:?}", self.playing.iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>());
        self.playing
            .get(name)
            .map(|playing| !playing.is_empty())
            .unwrap_or(false)
    }

    fn stop_playing(&mut self, name: &str) {
        if let Some(playing) = self.playing.get_mut(name) {
            for removed in playing.drain(..) {
                let _ = removed.stop();
            }
        }
    }
}
