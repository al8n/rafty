extern crate serde;
extern crate rmp_serde as rmps;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;

mod commands;
/// `errors` module contains the custom errors in this crate.
mod errors;

mod log;
mod mem_metrics;
mod mem_store;


cfg_default!(
    /// `pb` module contains the auto-generated RPC code.
    pub mod pb;
);

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
