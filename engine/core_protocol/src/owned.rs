// SPDX-FileCopyrightText: 2021 Softbear, Inc.
// SPDX-License-Identifier: AGPL-3.0-or-later

// Don't use Arcs on client.
// Certain things like MessageDto are deduplicated on the server.

// Owned is primarily used as Owned<[T]>.
// Box and Arc serialize the same way so this works.
#[cfg(feature = "server")]
pub type Owned<T> = std::sync::Arc<T>;
#[cfg(not(feature = "server"))]
pub type Owned<T> = Box<T>;

// Dedup is used as Dedup<Expensive>.
// Take care to ensure T is sized.
// Arc<T> and T serialize the same way as long as T is sized.
#[cfg(feature = "server")]
pub type Dedup<T> = std::sync::Arc<T>;
#[cfg(not(feature = "server"))]
pub type Dedup<T> = T;
