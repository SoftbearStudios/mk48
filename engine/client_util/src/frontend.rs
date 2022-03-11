use core_protocol::id::ServerId;
use serde::Serialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

pub trait Frontend<P: Serialize> {
    /// Set the props used to render the UI.
    fn set_ui_props(&self, props: P);
    /// Gets url hosting client files.
    fn get_real_host(&self) -> Option<String>;
    /// True iif should use HTTPS/WSS.
    fn get_real_encryption(&self) -> Option<bool>;
    /// Gets the server's response for ideal [`ServerId`].
    fn get_ideal_server_id(&self) -> Option<ServerId>;
}

pub struct Svelte;

impl<P: Serialize> Frontend<P> for Svelte {
    /// Javascript must implement part of this.
    fn set_ui_props(&self, props: P) {
        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            // props must be a JsValue corresponding to a US instance.
            #[wasm_bindgen(js_name = "setProps")]
            pub fn set_props(props: JsValue);
        }

        let ser = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
        set_props(props.serialize(&ser).unwrap());
    }

    fn get_real_host(&self) -> Option<String> {
        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            #[wasm_bindgen(js_name = "getRealHost", catch)]
            pub fn get_real_host() -> Result<String, JsValue>;
        }

        get_real_host().ok()
    }

    fn get_real_encryption(&self) -> Option<bool> {
        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            #[wasm_bindgen(js_name = "getRealEncryption", catch)]
            pub fn get_real_encryption() -> Result<bool, JsValue>;
        }

        get_real_encryption().ok()
    }

    fn get_ideal_server_id(&self) -> Option<ServerId> {
        #[wasm_bindgen(raw_module = "../../../src/App.svelte")]
        extern "C" {
            #[wasm_bindgen(js_name = "getIdealServerId", catch)]
            pub fn get_ideal_server_id() -> Result<u8, JsValue>;
        }

        get_ideal_server_id().ok().and_then(ServerId::new)
    }
}
