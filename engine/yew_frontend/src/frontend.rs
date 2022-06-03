use client_util::context::CoreState;
use client_util::frontend::Frontend;
use client_util::game_client::GameClient;
use core_protocol::id::{LanguageId, ServerId};
use std::ops::Deref;
use std::rc::Rc;
use yew::{use_context, Callback, Properties};

pub struct Yew<P> {
    pub(crate) set_ui_props: Callback<P>,
}

#[derive(Properties, PartialEq)]
pub struct PropertiesWrapper<P: PartialEq> {
    pub props: P,
}

impl<P: PartialEq> Deref for PropertiesWrapper<P> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.props
    }
}

/// Non-game-specific context wrapper.
#[derive(Clone)]
pub struct Ctw {
    pub language_id: LanguageId,
    /// A copy of the core state.
    pub state: Rc<CoreState>,
}

impl Ctw {
    pub fn use_core_state() -> Rc<CoreState> {
        let ctx = use_context::<Self>().unwrap();
        ctx.state
    }
}

impl PartialEq for Ctw {
    fn eq(&self, other: &Self) -> bool {
        self.language_id == other.language_id && Rc::ptr_eq(&self.state, &other.state)
    }
}

/// Game-specific context wrapper.
pub struct Gctw<G: GameClient> {
    pub send_ui_event_callback: Callback<G::UiEvent>,
}

impl<G: GameClient> Clone for Gctw<G> {
    fn clone(&self) -> Self {
        Self {
            send_ui_event_callback: self.send_ui_event_callback.clone(),
        }
    }
}

impl<G: GameClient> PartialEq for Gctw<G> {
    fn eq(&self, other: &Self) -> bool {
        self.send_ui_event_callback
            .eq(&other.send_ui_event_callback)
    }
}

impl<G: GameClient> Gctw<G> {
    pub fn send_ui_event(&self, ui_event: G::UiEvent) {
        self.send_ui_event_callback.emit(ui_event);
    }

    /// Only works in function component.
    pub fn use_ui_event(ui_event: G::UiEvent) {
        let ctx = use_context::<Self>().unwrap();
        ctx.send_ui_event(ui_event);
    }

    /// Only works in function component.
    pub fn use_ui_event_callback() -> Callback<G::UiEvent> {
        let ctx = use_context::<Self>().unwrap();
        ctx.send_ui_event_callback.clone()
    }
}

impl<P: PartialEq> Frontend<P> for Yew<P> {
    fn set_ui_props(&self, props: P) {
        self.set_ui_props.emit(props);
    }

    fn get_real_host(&self) -> Option<String> {
        None
    }

    fn get_real_encryption(&self) -> Option<bool> {
        None
    }

    fn get_ideal_server_id(&self) -> Option<ServerId> {
        None
    }
}
