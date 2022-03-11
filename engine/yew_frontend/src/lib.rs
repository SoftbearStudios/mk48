extern crate client_util;
extern crate core_protocol;
extern crate gloo_events;
extern crate gloo_render;
extern crate serde;
extern crate wasm_bindgen;
extern crate web_sys;
extern crate yew;

mod canvas;
mod frontend;
mod section;
mod window_event_listener;

use crate::canvas::Canvas;
use crate::section::Section;

use client_util::game_client::GameClient;
use client_util::infrastructure::Infrastructure;
use frontend::Yew;
use gloo_render::{request_animation_frame, AnimationFrame};
use std::num::NonZeroU8;
use yew::prelude::*;

struct App<G: GameClient + 'static> {
    infrastructure: Option<Infrastructure<G>>,
    _animation_frame: AnimationFrame,
}

#[derive(Default, PartialEq, Properties)]
struct AppProps {}

enum AppMsg {
    Frame { time: f64 },
}

impl<G: GameClient + 'static> App<G> {
    pub fn create_animation_frame(ctx: &Context<Self>) -> AnimationFrame {
        let link = ctx.link().clone();
        request_animation_frame(move |time| link.send_message(AppMsg::Frame { time }))
    }
}

impl<G: GameClient + 'static> Component for App<G> {
    type Message = AppMsg;
    type Properties = AppProps;

    fn create(ctx: &Context<Self>) -> Self {
        Self {
            infrastructure: None,
            _animation_frame: Self::create_animation_frame(ctx),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: AppMsg) -> bool {
        match msg {
            AppMsg::Frame { time } => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.frame((time * 0.001) as f32);
                }
                self._animation_frame = Self::create_animation_frame(ctx);
                false
            }
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <Canvas resolution_divisor={NonZeroU8::new(1).unwrap()}/>
                <div id="test_section">
                    <Section name="Section">
                        <p>{"Hi"}</p>
                    </Section>
                </div>
            </>
        }
    }

    fn rendered(&mut self, _ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.infrastructure = Some(Infrastructure::new(G::new(), Box::new(Yew)));
        }
    }
}

pub fn entry_point<G: GameClient + 'static>() {
    yew::start_app::<App<G>>();
}
