use std::collections::HashMap;
use anyhow::Result;
use crate::log::{Log, LogStore};

/// MemStore implements the LogStore and StableStore trait.
/// It should NOT EVER be used for production. It is used only for
/// unit tests. Use the MDBStore implementation instead.
pub struct MemStore<'a> {
    low_index: u64,
    high_index: u64,
    logs: HashMap<u64, &'a Log>,
    kv: HashMap<String, Vec<u8>>,
    kv_int: HashMap<String, u64>,
}

impl<'a> MemStore<'a> {
    pub fn new() -> Self {
        Self {
            low_index: 0,
            high_index: 0,
            logs: HashMap::new(),
            kv: HashMap::new(),
            kv_int: HashMap::new(),
        }
    }
}

impl<'a> LogStore for MemStore<'a> {
    fn first_index(&self) -> Result<u64> {
        todo!()
    }

    fn last_index(&self) -> Result<u64> {
        todo!()
    }

    fn get_log(&self, index: u64, log: &mut Log) -> Result<()> {
        todo!()
    }

    fn store_log(&self, log: &Log) -> Result<()> {
        todo!()
    }

    fn store_logs(&self, logs: Vec<&Log>) -> Result<()> {
        todo!()
    }

    fn delete_range(&self, min: u64, max: u64) -> Result<()> {
        todo!()
    }
}

