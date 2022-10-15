// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::window::event_listener::WindowEventListener;
use js_hooks::window;
use std::num::NonZeroU8;
use wasm_bindgen::JsValue;
use web_sys::{Event, FocusEvent, MouseEvent, TouchEvent, WheelEvent};
use yew::prelude::*;
use yew::{Callback, Context};

#[derive(PartialEq, Properties)]
pub struct CanvasProps {
    /// Resolution = window dimension / resolution divisor.
    pub resolution_divisor: NonZeroU8,
    /// Mouse enter, move, down, up, leave.
    pub mouse_callback: Option<Callback<MouseEvent>>,
    /// Touch start, move, end.
    pub touch_callback: Option<Callback<TouchEvent>>,
    /// Focus, blur.
    pub focus_callback: Option<Callback<FocusEvent>>,
    /// Wheel event.
    pub wheel_callback: Option<Callback<WheelEvent>>,
}

pub enum CanvasMsg {
    /// Window size has changed.
    Resize,
}

/// A window-sized canvas element with optional event listeners.
pub struct Canvas {
    _resize_event_listener: WindowEventListener<Event>,
}

impl Component for Canvas {
    type Message = CanvasMsg;
    type Properties = CanvasProps;

    fn create(ctx: &Context<Self>) -> Self {
        let resize_callback = ctx.link().callback(|_| CanvasMsg::Resize);

        Self {
            _resize_event_listener: WindowEventListener::new(
                "resize",
                move |_event| {
                    resize_callback.emit(());
                },
                false,
            ),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: CanvasMsg) -> bool {
        match msg {
            CanvasMsg::Resize => true,
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let w = window();

        let device_pixel_ratio = w.device_pixel_ratio();
        let window_width = dimension(
            w.inner_width(),
            device_pixel_ratio,
            ctx.props().resolution_divisor,
        );
        let window_height = dimension(
            w.inner_height(),
            device_pixel_ratio,
            ctx.props().resolution_divisor,
        );

        html! {
            <canvas
                id="canvas"
                style="position: absolute; width: 100%; height: 100%; z-index: -1000;"
                width={window_width}
                height={window_height}
                onmouseenter={ctx.props().mouse_callback.clone()}
                onmousemove={ctx.props().mouse_callback.clone()}
                onmousedown={ctx.props().mouse_callback.clone()}
                onmouseup={ctx.props().mouse_callback.clone()}
                onmouseleave={ctx.props().mouse_callback.clone()}
                ontouchstart={ctx.props().touch_callback.clone()}
                ontouchmove={ctx.props().touch_callback.clone()}
                ontouchend={ctx.props().touch_callback.clone()}
                onwheel={ctx.props().wheel_callback.clone()}
                onblur={ctx.props().focus_callback.clone()}
                onfocus={ctx.props().focus_callback.clone()}
                oncontextmenu={|event: MouseEvent| event.prevent_default()}
            />
        }
    }
}

fn dimension(
    resolution: Result<JsValue, JsValue>,
    device_pixel_ratio: f64,
    resolution_divisor: NonZeroU8,
) -> String {
    (resolution.unwrap().as_f64().unwrap() * device_pixel_ratio / resolution_divisor.get() as f64)
        .round()
        .to_string()
}
