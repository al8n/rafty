#![feature(box_syntax)]
// #![deny(missing_docs)]

extern crate derive_getters;

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rmp_serde as rmps;

#[macro_use]
extern crate lazy_static;

mod commands;

/// `errors` module contains the custom errors in this crate.
pub mod errors;

mod log;
mod mem_metrics;
mod mem_store;

/// `pb` module contains the auto-generated RPC code.
pub mod pb;
mod stable;
mod state;

/// Finite State Machine
pub mod fsm;

/// Log compaction trait
pub mod snapshot;

mod commitment;
/// The configuration for a Raft node
pub mod config;
mod idgen;
mod utils;

/// `Bytes` is alias for `Vec<u8>`
pub type Bytes = Vec<u8>;
