use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;

use crate::errors::Errors;
use crate::log::{Log, LogStore};
use crate::stable::StableStore;
use crate::Bytes;

/// MemStore implements the LogStore and StableStore trait.
/// It should NOT EVER be used for production. It is used only for
/// unit tests. Use the MDBStore implementation instead.
pub struct MemStore {
    core: Arc<RwLock<MemStoreCore>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            core: MemStoreCore::new(),
        }
    }
}

impl LogStore for MemStore {
    fn first_index(&self) -> Result<u64, Errors> {
        Ok(self.core.read().low_index)
    }

    fn last_index(&self) -> Result<u64, Errors> {
        Ok(self.core.read().high_index)
    }

    fn get_log(&self, index: u64) -> Result<Log, Errors> {
        match self.core.read().logs.get(&index) {
            None => Err(Errors::LogNotFound),
            Some(l) => Ok(l.clone()),
        }
    }

    fn store_log(&mut self, log: Log) -> Result<(), Errors> {
        self.store_logs(vec![log])
    }

    fn store_logs(&mut self, logs: Vec<Log>) -> Result<(), Errors> {
        let ls = &mut self.core.write();
        for l in logs.iter() {
            ls.logs.insert(l.index, l.clone());

            if ls.low_index == 0 {
                ls.low_index = l.index;
            }

            if l.index > ls.high_index {
                ls.high_index = l.index;
            }
        }
        Ok(())
    }

    fn delete_range(&mut self, min: u64, max: u64) -> Result<(), Errors> {
        let c = &mut self.core.write();
        for j in min..max {
            c.logs.remove(&j);
        }

        if min <= c.low_index {
            c.low_index = max + 1;
        }

        if max >= c.high_index {
            c.high_index = min - 1;
        }

        if c.low_index > c.high_index {
            c.low_index = 0;
            c.high_index = 0;
        }

        Ok(())
    }
}

impl StableStore for MemStore {
    fn set(&self, key: Bytes, val: Bytes) -> Result<()> {
        let key = String::from_utf8(key.to_vec())?;
        self.core.write().kv.insert(key, val);
        Ok(())
    }

    fn get(&self, key: Bytes) -> Result<Bytes, Errors> {
        let key = String::from_utf8(key.to_vec());

        match key {
            Ok(key) => match self.core.read().kv.get(&key) {
                None => Err(Errors::NotFound),
                Some(val) => Ok(Bytes::from(val.clone())),
            },
            Err(e) => Err(Errors::FromUtf8Error(e)),
        }
    }

    fn set_u64(&self, key: Bytes, val: u64) -> Result<()> {
        let key = String::from_utf8(key.to_vec())?;
        self.core.write().kv_int.insert(key, val);
        Ok(())
    }

    fn get_u64(&self, key: Bytes) -> Result<u64, Errors> {
        let key = String::from_utf8(key.to_vec());

        match key {
            Ok(key) => match self.core.read().kv_int.get(&key) {
                None => Err(Errors::NotFound),
                Some(val) => Ok(*val),
            },
            Err(e) => Err(Errors::FromUtf8Error(e)),
        }
    }
}

struct MemStoreCore {
    low_index: u64,
    high_index: u64,
    logs: HashMap<u64, Log>,
    kv: HashMap<String, Bytes>,
    kv_int: HashMap<String, u64>,
}

impl MemStoreCore {
    pub fn new() -> Arc<RwLock<Self>> {
        let c = Self {
            low_index: 0,
            high_index: 0,
            logs: HashMap::new(),
            kv: HashMap::new(),
            kv_int: HashMap::new(),
        };

        Arc::new(RwLock::new(c))
    }
}

#[cfg(test)]
mod test {
    use crate::errors::Errors;
    use crate::log::{Log, LogStore, LogType};
    use crate::mem_store::MemStore;
    use crate::stable::StableStore;
    use bytes::Bytes;

    #[test]
    fn test_log_store() -> anyhow::Result<()> {
        let mut ms = MemStore::new();

        assert_eq!(ms.first_index().unwrap(), 0);
        assert_eq!(ms.last_index().unwrap(), 0);

        ms.store_log(Log::new(0, 0, LogType::Command))?;
        ms.store_logs(vec![
            Log::new(1, 0, LogType::Command),
            Log::new(2, 0, LogType::Command),
            Log::new(3, 0, LogType::Command),
        ])?;

        let l = ms.get_log(1)?;
        assert_eq!(l.index, 1);

        ms.delete_range(0, 2)?;

        let not_found = ms.get_log(1);
        assert_eq!(Errors::LogNotFound, not_found.unwrap_err());

        ms.set("abc".as_bytes().to_vec(), "val".as_bytes().to_vec())?;
        ms.set_u64("u64".as_bytes().to_vec(), 69)?;

        let g = ms.get("abc".as_bytes().to_vec())?;
        assert_eq!(g, Bytes::from("val"));

        let gu64 = ms.get_u64("u64".as_bytes().to_vec())?;
        assert_eq!(gu64, 69);
        Ok(())
    }
}
