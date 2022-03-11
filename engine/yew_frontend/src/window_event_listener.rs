use crate::wasm_bindgen::UnwrapThrowExt;
use gloo_events::EventListener;
use std::marker::PhantomData;
use wasm_bindgen::JsCast;
use web_sys::window;

/// Listens to a certain event type on the window.
pub struct WindowEventListener<E> {
    _inner: EventListener,
    _spooky: PhantomData<E>,
}

impl<E: JsCast> WindowEventListener<E> {
    pub fn new(name: &'static str, mut callback: impl FnMut(&E) + 'static) -> Self {
        Self {
            _inner: EventListener::new(&window().unwrap(), name, move |event| {
                let typed_event = event.dyn_ref::<E>().unwrap_throw();
                callback(&typed_event);
            }),
            _spooky: PhantomData,
        }
    }
}
