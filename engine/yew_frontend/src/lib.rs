extern crate client_util;
extern crate core_protocol;
extern crate gloo_events;
extern crate gloo_render;
extern crate serde;
extern crate wasm_bindgen;
extern crate web_sys;
extern crate yew;

mod canvas;
pub mod component;
pub mod dialog;
pub mod frontend;
mod keyboard;
pub mod overlay;
pub mod svg;
mod translation;
pub mod window;

use crate::canvas::Canvas;

use client_util::game_client::GameClient;
use client_util::infrastructure::Infrastructure;
use core_protocol::id::LanguageId;
use frontend::{Ctw, Gctw, PropertiesWrapper, Yew};
use gloo_render::{request_animation_frame, AnimationFrame};
use keyboard::KeyboardEventsListener;
use std::marker::PhantomData;
use std::num::NonZeroU8;
use web_sys::{FocusEvent, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};
use yew::prelude::*;

struct App<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>>
where
    G::UiProps: Default + PartialEq + Clone,
{
    infrastructure: Option<Infrastructure<G>>,
    ui_props: G::UiProps,
    _animation_frame: AnimationFrame,
    _keyboard_events_listener: KeyboardEventsListener,
    _spooky: PhantomData<UI>,
}

#[derive(Default, PartialEq, Properties)]
struct AppProps {}

enum AppMsg<G: GameClient> {
    SetUiProps(G::UiProps),
    SendUiEvent(G::UiEvent),
    Frame { time: f64 },
    Keyboard(KeyboardEvent),
    KeyboardFocus(FocusEvent),
    Mouse(MouseEvent),
    Touch(TouchEvent),
    MouseFocus(FocusEvent),
    Wheel(WheelEvent),
}

impl<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>> App<G, UI>
where
    G::UiProps: Default + PartialEq + Clone,
{
    pub fn create_animation_frame(ctx: &Context<Self>) -> AnimationFrame {
        let link = ctx.link().clone();
        request_animation_frame(move |time| link.send_message(AppMsg::Frame { time }))
    }
}

impl<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>> Component
    for App<G, UI>
where
    G::UiProps: Default + PartialEq + Clone,
{
    type Message = AppMsg<G>;
    type Properties = AppProps;

    fn create(ctx: &Context<Self>) -> Self {
        let keyboard_callback = ctx.link().callback(AppMsg::Keyboard);
        let keyboard_focus_callback = ctx.link().callback(AppMsg::KeyboardFocus);

        Self {
            infrastructure: None,
            ui_props: G::UiProps::default(),
            _animation_frame: Self::create_animation_frame(ctx),
            _keyboard_events_listener: KeyboardEventsListener::new(
                keyboard_callback,
                keyboard_focus_callback,
            ),
            _spooky: PhantomData,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: AppMsg<G>) -> bool {
        match msg {
            AppMsg::SetUiProps(props) => {
                self.ui_props = props;
                return true;
            }
            AppMsg::SendUiEvent(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.ui_event(event);
                }
            }
            AppMsg::Frame { time } => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.frame((time * 0.001) as f32);
                }
                self._animation_frame = Self::create_animation_frame(ctx);
            }
            AppMsg::Keyboard(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.keyboard(event);
                }
            }
            AppMsg::KeyboardFocus(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.keyboard_focus(event);
                }
            }
            AppMsg::Mouse(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.mouse(event);
                }
            }
            AppMsg::Touch(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.touch(event);
                }
            }
            AppMsg::MouseFocus(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.mouse_focus(event);
                }
            }
            AppMsg::Wheel(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.wheel(event);
                }
            }
        }
        false
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let mouse_callback = ctx.link().callback(AppMsg::Mouse);
        let touch_callback = ctx.link().callback(AppMsg::Touch);
        let focus_callback = ctx.link().callback(AppMsg::MouseFocus);
        let wheel_callback = ctx.link().callback(AppMsg::Wheel);
        let send_ui_event_callback = ctx.link().callback(AppMsg::SendUiEvent);

        let context = Ctw {
            language_id: LanguageId::English,
        };

        let game_context = Gctw {
            send_ui_event_callback,
        };

        html! {
            <>
                <Canvas
                    resolution_divisor={NonZeroU8::new(1).unwrap()}
                    {mouse_callback}
                    {touch_callback}
                    {focus_callback}
                    {wheel_callback}
                />
                <ContextProvider<Ctw> {context}>
                    <ContextProvider<Gctw<G>> context={game_context}>
                        <UI props={self.ui_props.clone()}/>
                    </ContextProvider<Gctw<G>>>
                </ContextProvider<Ctw>>
            </>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let set_ui_props = ctx.link().callback(AppMsg::SetUiProps);
            self.infrastructure = Some(Infrastructure::new(
                G::new(),
                Box::new(Yew { set_ui_props }),
            ));
        }
    }
}

pub fn entry_point<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>>()
where
    G::UiProps: Default + PartialEq + Clone,
{
    yew::start_app::<App<G, UI>>();
}
