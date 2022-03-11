use client_util::frontend::Frontend;
use core_protocol::id::ServerId;
use serde::Serialize;

pub struct Yew;

impl<P: Serialize> Frontend<P> for Yew {
    fn set_ui_props(&self, props: P) {}

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
