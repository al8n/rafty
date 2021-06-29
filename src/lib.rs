#![feature(box_syntax)]
// #![deny(missing_docs)]

extern crate derive_getters;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate derive_builder;

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

/// The configuration for a Raft node
pub mod config;
mod idgen;

/// `Bytes` is alias for `Vec<u8>`
pub type Bytes = Vec<u8>;
