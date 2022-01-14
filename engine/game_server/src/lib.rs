#![feature(drain_filter)]
#![feature(new_uninit)]
#![feature(get_mut_unchecked)]
#![feature(async_closure)]
#![feature(hash_drain_filter)]
#![feature(generic_associated_types)]

pub mod context;
pub mod entry_point;
pub mod game_service;
pub mod infrastructure;
pub mod protocol;
