// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use wasm_bindgen::JsCast;
use web_sys::Event;

pub fn event_target<T: JsCast>(event: &Event) -> T {
    let target = event.target().expect("missing event target");
    target.dyn_into::<T>().expect("wrong event target")
}
