use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::JsValue;
use web_sys::{window, Storage};

/// For interacting with the web local storage API.
pub struct LocalStorage {
    inner: Option<Storage>,
}

/// Errors that can occur with local storage.
pub enum Error {
    /// Javascript error.
    Js(JsValue),
    /// Serialization error.
    Serde(serde_json::Error),
    /// Local storage API is not available.
    Nonexistent,
}

impl LocalStorage {
    /// If local storage API is unavailable, future calls will return `Err(Error::Nonexistent)`.
    pub(crate) fn new() -> Self {
        Self {
            inner: window().unwrap().local_storage().ok().flatten(),
        }
    }

    /// Gets a key from local storage, returning None if it doesn't exist or any error occurs.
    pub fn get<V: DeserializeOwned>(&self, key: &str) -> Option<V> {
        self.try_get(key).ok().flatten()
    }

    /// Gets a key from local storage, returning Ok(None) if it doesn't exist or Err if an error occurs.
    pub fn try_get<V: DeserializeOwned>(&self, key: &str) -> Result<Option<V>, Error> {
        let inner = self.inner.as_ref().ok_or(Error::Nonexistent)?;

        let s: Option<String> = inner.get(key).map_err(|e| Error::Js(e))?;

        match s {
            Some(s) => serde_json::from_str(&s).map_err(|e| Error::Serde(e)),
            None => Ok(None),
        }
    }

    /// Sets a key in local storage to a value.
    pub fn set<V: Serialize>(&mut self, key: &str, value: Option<V>) -> Result<(), Error> {
        let inner = self.inner.as_ref().ok_or(Error::Nonexistent)?;

        match value {
            Some(ref v) => inner
                .set(key, &serde_json::to_string(v).map_err(|e| Error::Serde(e))?)
                .map_err(|e| Error::Js(e)),
            None => inner.delete(key).map_err(|e| Error::Js(e)),
        }
    }
}
