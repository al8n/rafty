/* Copyright 2021 Al Liu (https://github.com/al8n). Licensed under Apache-2.0.
 *
 * Copyright 2017 The Hashicorp's Raft repository authors(https://github.com/hashicorp/raft) authors. Licensed under MPL-2.0.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;

use crate::errors::Errors;
use crate::log::{Log, LogStore};
use crate::stable::StableStore;
use crate::Bytes;

/// MemStore implements the LogStore and StableStore trait.
/// It should NOT EVER be used for production. It is used only for
/// unit tests. Use the MDBStore implementation instead.
pub struct MemStore {
    low_index: usize,
    high_index: usize,
    logs: HashMap<u64, Arc<Log>>,
    kv: HashMap<String, Bytes>,
    kv_int: HashMap<String, u64>,
}

impl MemStore {
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

impl LogStore for MemStore {
    fn first_index(&self) -> Result<usize, Errors> {
        Ok(self.low_index)
    }

    fn last_index(&self) -> Result<usize, Errors> {
        Ok(self.high_index)
    }

    fn get_log(&self, index: usize) -> Result<Arc<Log>, Errors> {
        match self.logs.get(&(index as u64)) {
            None => Err(Errors::LogNotFound),
            Some(l) => Ok(l.clone()),
        }
    }

    fn store_log(&mut self, log: Arc<Log>) -> Result<(), Errors> {
        self.store_logs(vec![log])
    }

    fn store_logs(&mut self, logs: Vec<Arc<Log>>) -> Result<(), Errors> {
        for l in logs.iter() {
            self.logs.insert(l.index, l.clone());

            if self.low_index == 0 {
                self.low_index = l.index as usize;
            }

            if l.index > self.high_index as u64 {
                self.high_index = l.index as usize;
            }
        }
        Ok(())
    }

    fn delete_range(&mut self, min: u64, max: u64) -> Result<(), Errors> {
        for j in min..(max + 1) {
            self.logs.remove(&j);
        }

        if min <= self.low_index as u64 {
            self.low_index = (max + 1) as usize;
        }

        if max >= self.high_index as u64 {
            self.high_index = (min - 1) as usize;
        }

        if self.low_index > self.high_index {
            self.low_index = 0;
            self.high_index = 0;
        }

        Ok(())
    }
}

impl StableStore for MemStore {
    fn set(&mut self, key: Bytes, val: Bytes) -> Result<()> {
        let key = String::from_utf8(key.to_vec())?;
        self.kv.insert(key, val);
        Ok(())
    }

    fn get(&self, key: Bytes) -> Result<Bytes, Errors> {
        let key = String::from_utf8(key.to_vec());

        match key {
            Ok(key) => match self.kv.get(&key) {
                None => Err(Errors::NotFound),
                Some(val) => Ok(Bytes::from(val.clone())),
            },
            Err(e) => Err(Errors::FromUtf8Error(e)),
        }
    }

    fn set_u64(&mut self, key: Bytes, val: u64) -> Result<()> {
        let key = String::from_utf8(key.to_vec())?;
        self.kv_int.insert(key, val);
        Ok(())
    }

    fn get_u64(&self, key: Bytes) -> Result<u64, Errors> {
        let key = String::from_utf8(key.to_vec());

        match key {
            Ok(key) => match self.kv_int.get(&key) {
                None => Err(Errors::NotFound),
                Some(val) => Ok(*val),
            },
            Err(e) => Err(Errors::FromUtf8Error(e)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::errors::Errors;
    use crate::log::{Log, LogStore, LogType};
    use crate::mem_store::MemStore;
    use crate::stable::StableStore;
    use bytes::Bytes;
    use std::sync::Arc;

    #[test]
    fn test_log_store() -> anyhow::Result<()> {
        let mut ms = MemStore::new();

        assert_eq!(ms.first_index().unwrap(), 0);
        assert_eq!(ms.last_index().unwrap(), 0);

        ms.store_log(Arc::new(Log::new(0, 0, LogType::Command)))?;
        ms.store_logs(vec![
            Arc::new(Log::new(1, 0, LogType::Command)),
            Arc::new(Log::new(2, 0, LogType::Command)),
            Arc::new(Log::new(3, 0, LogType::Command)),
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
