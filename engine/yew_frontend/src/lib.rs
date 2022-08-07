#![feature(box_into_inner)]
// Yew `tr`
#![feature(stmt_expr_attributes)]
#![feature(int_log)]

extern crate client_util;
extern crate core;
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
pub mod event;
pub mod frontend;
mod keyboard;
pub mod overlay;
pub mod ptr_eq_rc;
pub mod svg;
pub mod translation;
pub mod window;

use crate::canvas::Canvas;
use crate::component::context_menu::ContextMenuProps;
use crate::dialog::privacy_dialog::PrivacyDialog;
use crate::dialog::settings_dialog::SettingsDialog;
use crate::dialog::terms_dialog::TermsDialog;
use crate::overlay::connection_lost::ConnectionLost;
use crate::ptr_eq_rc::PtrEqRc;
use crate::window::event_listener::WindowEventListener;
use client_util::browser_storage::BrowserStorages;
use client_util::game_client::GameClient;
use client_util::infrastructure::Infrastructure;
use client_util::setting::CommonSettings;
use core_protocol::id::InvitationId;
use core_protocol::rpc::{ChatRequest, PlayerRequest, Request, TeamRequest};
use frontend::{Ctw, Gctw, PropertiesWrapper, SettingCache, Yew};
use gloo_render::{request_animation_frame, AnimationFrame};
use keyboard::KeyboardEventsListener;
use std::marker::PhantomData;
use std::num::NonZeroU8;
use stylist::{global_style, GlobalStyle};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::future_to_promise;
use web_sys::{FocusEvent, KeyboardEvent, MouseEvent, TouchEvent, WheelEvent};
use yew::prelude::*;
use yew_router::prelude::*;

pub const CONTACT_EMAIL: &'static str = "contact@softbear.com";

struct App<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>>
where
    G::UiProps: Default + PartialEq + Clone,
{
    context_menu: Option<ContextMenuProps>,
    infrastructure: Option<Infrastructure<G>>,
    ui_props: G::UiProps,
    _animation_frame: AnimationFrame,
    _keyboard_events_listener: KeyboardEventsListener,
    _visibility_listener: WindowEventListener<Event>,
    _global_style: GlobalStyle,
    _spooky: PhantomData<UI>,
}

#[derive(Default, PartialEq, Properties)]
struct AppProps {}

enum AppMsg<G: GameClient> {
    ChangeCommonSettings(Box<dyn FnOnce(&mut CommonSettings, &mut BrowserStorages)>),
    CreateInfrastructure(Box<Infrastructure<G>>),
    Frame { time: f64 },
    KeyboardFocus(FocusEvent),
    Keyboard(KeyboardEvent),
    MouseFocus(FocusEvent),
    Mouse(MouseEvent),
    RawZoom(f32),
    SendChatRequest(ChatRequest),
    SendPlayerRequest(PlayerRequest),
    SendTeamRequest(TeamRequest),
    SendUiEvent(G::UiEvent),
    SetContextMenuProps(Option<ContextMenuProps>),
    SetUiProps(G::UiProps),
    Touch(TouchEvent),
    VisibilityChange(Event),
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
        let visibility_callback = ctx.link().callback(AppMsg::VisibilityChange);

        Self {
            context_menu: None,
            infrastructure: None,
            ui_props: G::UiProps::default(),
            _animation_frame: Self::create_animation_frame(ctx),
            _keyboard_events_listener: KeyboardEventsListener::new(
                keyboard_callback,
                keyboard_focus_callback,
            ),
            _visibility_listener: WindowEventListener::new(
                "visibilitychange",
                move |event: &Event| {
                    visibility_callback.emit(event.clone());
                },
                false,
            ),
            _global_style: global_style!(
                r#"
                html {
                    font-family: sans-serif;
                    font-size: 2vmin;
                    font-size: calc(10px + 1vmin);
                }

                body {
                    color: white;
                    margin: 0;
                    overflow: hidden;
                    padding: 0;
                    touch-action: none;
                }

                a {
                    color: white;
                }
            "#
            )
            .expect("failed to mount style"),
            _spooky: PhantomData,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: AppMsg<G>) -> bool {
        match msg {
            AppMsg::ChangeCommonSettings(change) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    change(
                        &mut infrastructure.context.common_settings,
                        &mut infrastructure.context.browser_storages,
                    );
                    // Just in case.
                    return true;
                }
            }
            AppMsg::CreateInfrastructure(infrastructure) => {
                assert!(self.infrastructure.is_none());
                self.infrastructure = Some(Box::into_inner(infrastructure));
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
            AppMsg::MouseFocus(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.mouse_focus(event);
                }
            }
            AppMsg::RawZoom(amount) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.raw_zoom(amount);
                }
            }
            AppMsg::SendChatRequest(request) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.send_request(Request::Chat(request));
                }
            }
            AppMsg::SetContextMenuProps(props) => {
                self.context_menu = props;
                return true;
            }
            AppMsg::SendPlayerRequest(request) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.send_request(Request::Player(request));
                }
            }
            AppMsg::SendTeamRequest(request) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.send_request(Request::Team(request));
                }
            }
            AppMsg::SendUiEvent(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.ui_event(event);
                }
            }
            AppMsg::SetUiProps(props) => {
                self.ui_props = props;
                return true;
            }
            AppMsg::Touch(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.touch(event);
                }
            }
            AppMsg::VisibilityChange(event) => {
                if let Some(infrastructure) = self.infrastructure.as_mut() {
                    infrastructure.visibility_change(event);
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
        let change_common_settings_callback = ctx.link().callback(AppMsg::ChangeCommonSettings);
        let chat_request_callback = ctx.link().callback(AppMsg::SendChatRequest);
        let focus_callback = ctx.link().callback(AppMsg::MouseFocus);
        let mouse_callback = ctx.link().callback(AppMsg::Mouse);
        let player_request_callback = ctx.link().callback(AppMsg::SendPlayerRequest);
        let raw_zoom_callback = ctx.link().callback(AppMsg::RawZoom);
        let send_ui_event_callback = ctx.link().callback(AppMsg::SendUiEvent);
        let set_context_menu_callback = ctx.link().callback(AppMsg::SetContextMenuProps);
        let team_request_callback = ctx.link().callback(AppMsg::SendTeamRequest);
        let touch_callback = ctx.link().callback(AppMsg::Touch);
        let wheel_callback = ctx.link().callback(AppMsg::Wheel);

        let context = Ctw {
            chat_request_callback,
            change_common_settings_callback,
            context_menu: self.context_menu.clone(),
            game_id: G::GAME_ID,
            player_request_callback,
            raw_zoom_callback,
            set_context_menu_callback,
            setting_cache: self
                .infrastructure
                .as_ref()
                .map(|i| {
                    let settings = &i.context.common_settings;

                    SettingCache {
                        alias: settings.alias,
                        chat_dialog_shown: settings.chat_dialog_shown,
                        leaderboard_dialog_shown: settings.leaderboard_dialog_shown,
                        team_dialog_shown: settings.team_dialog_shown,
                        language_id: settings.language,
                        volume: settings.volume,
                    }
                })
                .unwrap_or_default(),
            state: PtrEqRc::new(
                self.infrastructure
                    .as_ref()
                    .map(|i| i.context.state.core.clone())
                    .unwrap_or_default(),
            ),
            team_request_callback,
        };

        let game_context = Gctw {
            send_ui_event_callback,
        };

        html! {
             <BrowserRouter>
                <Canvas
                    resolution_divisor={NonZeroU8::new(1).unwrap()}
                    {mouse_callback}
                    {touch_callback}
                    {focus_callback}
                    {wheel_callback}
                />
                <ContextProvider<Ctw> {context}>
                    <ContextProvider<Gctw<G>> context={game_context}>
                        if self.infrastructure.as_ref().map(|i| i.context.connection_lost()).unwrap_or_default() {
                            <ConnectionLost/>
                        } else {
                            <>
                                <UI props={self.ui_props.clone()}/>
                                <Switch<Route> render={Switch::render(switch)} />
                            </>
                        }
                    </ContextProvider<Gctw<G>>>
                </ContextProvider<Ctw>>
            </BrowserRouter>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            let set_ui_props = ctx.link().callback(AppMsg::SetUiProps);
            let create_infrastructure_callback = ctx.link().callback(AppMsg::CreateInfrastructure);
            let _ = future_to_promise(async move {
                let infrastructure = Box::new(Infrastructure::new(
                    G::new(),
                    Box::new(Yew::new(set_ui_props).await),
                ));
                create_infrastructure_callback.emit(infrastructure);
                Ok(JsValue::NULL)
            });
        }
    }
}

pub fn entry_point<G: GameClient, UI: Component<Properties = PropertiesWrapper<G::UiProps>>>()
where
    G::UiProps: Default + PartialEq + Clone,
{
    yew::start_app::<App<G, UI>>();
}

#[derive(Clone, Copy, Routable, PartialEq)]
pub enum Route {
    #[at("/invite/:invitation_id")]
    Invitation { invitation_id: InvitationId },
    #[at("/privacy")]
    Privacy,
    #[at("/settings")]
    Settings,
    #[at("/terms")]
    Terms,
    #[at("/")]
    #[not_found]
    Home,
}

fn switch(routes: &Route) -> Html {
    match routes {
        Route::Home => html! {},
        Route::Invitation { .. } => html! {},
        Route::Privacy => html! {
            <PrivacyDialog/>
        },
        Route::Settings => html! {
            <SettingsDialog/>
        },
        Route::Terms => html! {
            <TermsDialog/>
        },
    }
}
