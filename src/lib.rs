#[macro_use]
extern crate serde;
extern crate rmp_serde as rmps;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate parse_display;

#[macro_use]
mod macros;

pub mod commands;
/// `errors` module contains the custom errors in this crate.
pub mod errors;

pub mod log;
pub mod mem_metrics;
pub mod mem_store;


cfg_default!(
    /// `pb` module contains the auto-generated RPC code.
    pub mod pb;
);

pub mod stable;
pub mod state;

/// Finite State Machine
pub mod fsm;

/// Log compaction trait
pub mod snapshot;

pub mod commitment;
/// The configuration for a Raft node
pub mod config;
pub mod idgen;
pub mod utils;
pub mod discard_snapshot;
pub mod transport;

#[cfg(feature = "default")]
mod wg;
mod replication;
mod future;
mod ch;

/// `Bytes` is alias for `Vec<u8>`
pub type Bytes = Vec<u8>;
