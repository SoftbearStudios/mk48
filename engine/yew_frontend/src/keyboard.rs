// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::window::event_listener::WindowEventListener;
use web_sys::{FocusEvent, KeyboardEvent};
use yew::Callback;

pub(crate) struct KeyboardEventsListener {
    _blur_event_listener: WindowEventListener<FocusEvent>,
    _focus_event_listener: WindowEventListener<FocusEvent>,
    _keydown_event_listener: WindowEventListener<KeyboardEvent>,
    _keyup_event_listener: WindowEventListener<KeyboardEvent>,
}

impl KeyboardEventsListener {
    pub fn new(
        keyboard_callback: Callback<KeyboardEvent>,
        focus_callback: Callback<FocusEvent>,
    ) -> Self {
        let focus_callback_clone = focus_callback.clone();
        let keyboard_callback_clone = keyboard_callback.clone();
        Self {
            _blur_event_listener: WindowEventListener::new(
                "blur",
                move |event: &FocusEvent| {
                    focus_callback.emit(event.clone());
                },
                true,
            ),
            _focus_event_listener: WindowEventListener::new(
                "focus",
                move |event: &FocusEvent| {
                    focus_callback_clone.emit(event.clone());
                },
                true,
            ),
            _keyup_event_listener: WindowEventListener::new(
                "keyup",
                move |event: &KeyboardEvent| {
                    keyboard_callback.emit(event.clone());
                },
                true,
            ),
            _keydown_event_listener: WindowEventListener::new(
                "keydown",
                move |event: &KeyboardEvent| {
                    keyboard_callback_clone.emit(event.clone());
                },
                true,
            ),
        }
    }
}
