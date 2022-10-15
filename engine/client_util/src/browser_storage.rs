// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

use js_hooks::window;
use std::str::FromStr;
use web_sys::Storage;

/// For interacting with the local storage and session storage APIs.
pub struct BrowserStorages {
    pub local: BrowserStorage,
    pub session: BrowserStorage,
    /// Black hole; reads and writes will always return error.
    #[doc(hidden)]
    pub no_op: BrowserStorage,
}

impl BrowserStorages {
    pub fn new() -> Self {
        Self {
            local: BrowserStorage::new(window().local_storage().ok().flatten()),
            session: BrowserStorage::new(window().session_storage().ok().flatten()),
            no_op: BrowserStorage::new(None),
        }
    }
}

/// For interacting with the web storage API.
pub struct BrowserStorage {
    inner: Option<Storage>,
}

/// Errors that can occur with storages.
#[derive(Debug)]
pub enum Error {
    /// Javascript error.
    Js,
    /// Serialization error.
    FromStr,
    /// Storage API is not available.
    Nonexistent,
}

impl BrowserStorage {
    /// If storage API is unavailable, future calls will return `Err(Error::Nonexistent)`.
    pub(crate) fn new(inner: Option<Storage>) -> Self {
        Self { inner }
    }

    /// Gets a key from storage, returning None if it doesn't exist or any error occurs.
    pub fn get<V: FromStr>(&self, key: &str) -> Option<V> {
        self.try_get(key).ok().flatten()
    }

    /// Gets a key from storage, returning Ok(None) if it doesn't exist or Err if an error occurs.
    fn try_get<V: FromStr>(&self, key: &str) -> Result<Option<V>, Error> {
        self.inner
            .as_ref()
            .ok_or(Error::Nonexistent)?
            .get(key)
            .map_err(|_| Error::Js)?
            .map(|s| V::from_str(&s).map_err(|_| Error::FromStr))
            .transpose()
    }

    /// Sets a key in storage to a value.
    pub fn set<V: ToString>(&mut self, key: &str, value: Option<V>) -> Result<(), Error> {
        let inner = self.inner.as_ref().ok_or(Error::Nonexistent)?;
        match value {
            Some(v) => inner.set(key, &v.to_string()),
            None => inner.delete(key),
        }
        .map_err(|_| Error::Js)
    }
}
