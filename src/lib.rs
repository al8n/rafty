extern crate derive_getters;
extern crate lazy_static;

mod commands;
pub mod errors;
mod log;
mod mem_store;
pub mod pb;
mod stable;
mod state;

pub struct Node {}

type Bytes = Vec<u8>;
