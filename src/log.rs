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
use crate::errors::Error;
use chrono::{DateTime, Utc};
use metrics::gauge;
use parking_lot::RwLock;
use parse_display::{Display, FromStr};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "default")]
use tokio::time::sleep;
#[cfg(feature = "default")]
use tokio::{select, sync::mpsc::Receiver};

#[cfg(not(feature = "default"))]
use crossbeam::{
    select,
    channel::Receiver
};

/// LogType describes various types of log entries
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "CamelCase")]
pub enum LogType {
    /// `Command` is applied to a user FSM
    Command,

    /// `Noop` is used to assert leadership
    Noop,

    /// `AddPeerDeprecated` is used to add a new peer, This should only be used with
    /// older protocol versions designed to be compatible with unversioned
    /// Raft servers. See comments in config.rs for details
    AddPeerDeprecated,

    /// `RemovePeerDeprecated` is used to remove an existing peer. This should only be
    /// used with older protocol versions designed to be compatible with
    /// unversioned Raft servers. See comments in config.rs for details.
    RemovePeerDeprecated,

    /// `Barrier` is used to ensure all preceding operations have been
    /// applied to the FSM. It is similar to Noop, but instead of returning
    /// once committed, it only returns once the FSM manager acks it. Otherwise
    /// it is possible there are operations committed but not yet applied to
    /// the FSM
    Barrier,

    /// `Configuration` establishes a membership change configuration. It is
    /// created when a server is added, removed, promoted, etc. Only used
    /// when protocol version 1 or greater is in use.
    Configuration,
}

/// Log entries are replicated to all members of the Raft cluster
/// and form the heart of the replicated state machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    /// index holds the index of the log entry
    pub index: u64,

    /// term holds the election term of the log entry
    pub term: u64,

    /// typ holds the type of the log entry
    pub typ: LogType,

    /// data holds the log entry's type-specific data,
    pub data: Vec<u8>,

    /// extensions holds an opaque byte slice of information for middleware. It
    /// is up to the client of the library to properly modify this as it adds
    /// layers and remove those layers when appropriate. This value is a part of
    /// the log, so very large values could cause timing issues.
    ///
    /// **N.B.** It is *up to the client* to handle upgrade paths. For instance if
    /// using this with go-raftchunking, the client should ensure that all Raft
    /// peers are using a version that can handle that extension before ever
    /// actually triggering chunking behavior. It is sometimes sufficient to
    /// ensure that non-leaders are upgraded first, then the current leader is
    /// upgraded, but a leader changeover during this process could lead to
    /// trouble, so gating extension behavior via some flag in the client
    /// program is also a good idea.
    pub extensions: Vec<u8>,

    /// appended_at stores the time the leader first appended this log to it's
    /// log_store. Followers will observe the leader's time. It is not used for
    /// coordination or as part of the replication protocol at all. It exists only
    /// to provide operational information for example how many seconds worth of
    /// logs are present on the leader which might impact follower's ability to
    /// catch up after restoring a large snapshot. We should never rely on this
    /// being in the past when appending on a follower or reading a log back since
    /// the clock skew can mean a follower could see a log with a future timestamp.
    /// In general too the leader is not required to persist the log before
    /// delivering to followers although the current implementation happens to do
    /// this.
    pub append_at: DateTime<Utc>,
}

impl Log {
    pub fn new(index: u64, term: u64, typ: LogType) -> Self {
        Self {
            index,
            term,
            typ,
            data: vec![],
            extensions: vec![],
            append_at: Utc::now(),
        }
    }

    pub fn with_arc(index: u64, term: u64, typ: LogType) -> Arc<Self> {
        Arc::new(Self::new(index, term, typ))
    }

    pub fn with_all(
        index: u64,
        term: u64,
        typ: LogType,
        data: Vec<u8>,
        ext: Vec<u8>,
        append_at: DateTime<Utc>,
    ) -> Self {
        Self {
            index,
            term,
            typ,
            data,
            extensions: ext,
            append_at,
        }
    }

    pub fn with_all_and_arc(
        index: u64,
        term: u64,
        typ: LogType,
        data: Vec<u8>,
        ext: Vec<u8>,
        append_at: DateTime<Utc>,
    ) -> Arc<Self> {
        Arc::new(Self::with_all(index, term, typ, data, ext, append_at))
    }

    pub fn set_data(&mut self, data: Vec<u8>) {
        self.data = data;
    }

    pub fn set_extensions(&mut self, ext: Vec<u8>) {
        self.extensions = ext;
    }

    pub fn set_append_time(&mut self, time: DateTime<Utc>) {
        self.append_at = time;
    }

    pub fn set_type(&mut self, typ: LogType) {
        self.typ = typ;
    }
}

impl Default for Log {
    fn default() -> Self {
        Self {
            index: 0,
            term: 0,
            typ: LogType::Command,
            data: vec![],
            extensions: vec![],
            append_at: Utc::now(),
        }
    }
}

impl From<Arc<Log>> for Log {
    fn from(al: Arc<Log>) -> Self {
        Self {
            index: al.index,
            term: al.term,
            typ: al.typ,
            data: al.data.clone(),
            extensions: al.extensions.clone(),
            append_at: al.append_at,
        }
    }
}

/// LogStore is used to provide an interface for storing
/// and retrieving logs in a durable fashion.
pub trait LogStore {
    /// `first_index` returns the first index written. 0 for no entries.
    fn first_index(&self) -> Result<usize, Error>;

    /// `last_index` returns the last index written. 0 for no entries.
    fn last_index(&self) -> Result<usize, Error>;

    /// `get_log` gets a log entry at a given index.
    fn get_log(&self, index: usize) -> Result<Arc<Log>, Error>;

    /// `store_log` stores a log entry
    fn store_log(&mut self, log: Arc<Log>) -> Result<(), Error>;

    /// `store_logs` stores multiple log entries.
    fn store_logs(&mut self, logs: Vec<Arc<Log>>) -> Result<(), Error>;

    /// `delete_range` deletes a range of log entries. The range is inclusive.
    fn delete_range(&mut self, min: u64, max: u64) -> Result<(), Error>;
}

/// `oldest_log` returns the oldest log in the store.
fn oldest_log<T>(store: T) -> Result<Arc<Log>, Error>
where
    T: LogStore + Clone,
{
    // We might get unlucky and have a truncate right between getting first log
    // index and fetching it so keep trying until we succeed or hard fail.
    let mut last_fail_idx = 0;
    let mut last_err: Error = Error::LogNotFound;

    loop {
        let first_idx = store.first_index()?;
        if first_idx == 0 {
            return Err(Error::LogNotFound);
        }

        if first_idx == last_fail_idx {
            // Got same index as last time around which errored, don't bother trying
            // to fetch it again just return the error
            return Err(last_err);
        }

        match store.get_log(first_idx) {
            Ok(l) => {
                // we found the oldest log, break the loop
                return Ok(l.clone());
            }
            Err(e) => {
                // We failed, keep trying to see if there is a new first_idx
                last_err = e;
                last_fail_idx = first_idx;
            }
        }
    }
}

cfg_default!(
    /// `emit_log_store_metrics` emits the information to the metrics
    pub async fn emit_log_store_metrics<T>(
        store: T,
        prefix: String,
        interval: Duration,
        stop_chan: &mut Receiver<()>,
    ) where
        T: LogStore + Clone,
    {
        loop {
            select! {
                    _ = stop_chan.recv() => return,
                    _ = sleep(interval) => {
                        // In error case emit 0 as the age
                        let mut age_ms = 0i64;
                        if let Ok(l) = oldest_log(store.clone()) {
                            if l.append_at.timestamp_millis() != 0 {
                                age_ms = Utc::now().signed_duration_since(l.append_at).num_milliseconds();
                            }
                        }

                        gauge!(vec![prefix.clone(), "oldest.log.age".to_string()].join("."), age_ms as f64);
                    },
                }
        }
    }
);

cfg_not_default!(
    /// `emit_log_store_metrics` emits the information to the metrics
    pub fn emit_log_store_metrics<T>(
        store: T,
        prefix: String,
        interval: Duration,
        stop_chan: &mut Receiver<()>,
    ) where
        T: LogStore + Clone,
    {
        loop {
            select! {
                recv(stop_chan) -> _ => return,
                default(interval) => {
                    // In error case emit 0 as the age
                    let mut age_ms = 0i64;
                    if let Ok(l) = oldest_log(store.clone()) {
                        if l.append_at.timestamp_millis() != 0 {
                            age_ms = Utc::now().signed_duration_since(l.append_at).num_milliseconds();
                        }
                    }
                    gauge!(vec![prefix.clone(), "oldest.log.age".to_string()].join("."), age_ms as f64);
                },
            }
        }
    }
);


/// `LogCache` wraps any `LogStore` implementation to provide an
/// in-memory ring buffer. This is used to cache access to
/// the recently written entries. For implementations that do not
/// cache themselves, this can provide a substantial boost by
/// avoiding disk I/O on recent entries.
///
/// TODO: Ring buffer is a temporary solution for cache.
pub struct LogCache<T: LogStore> {
    store: T,
    cache: HashMap<usize, Arc<Log>>,
    cap: usize,
}

impl<T: LogStore> LogCache<T> {
    pub fn new(store: T, capacity: usize) -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            store,
            cache: HashMap::with_capacity(capacity),
            cap: capacity,
        }))
    }
}

impl<T: LogStore> LogStore for LogCache<T> {
    fn first_index(&self) -> Result<usize, Error> {
        self.store.first_index()
    }

    fn last_index(&self) -> Result<usize, Error> {
        self.store.last_index()
    }

    fn get_log(&self, index: usize) -> Result<Arc<Log>, Error> {
        let cached: Option<&Arc<Log>> = self.cache.get(&(index % self.cap));

        match cached {
            None => self.store.get_log(index),
            Some(l) => Ok(l.clone()),
        }
    }

    fn store_log(&mut self, log: Arc<Log>) -> Result<(), Error> {
        self.store.store_logs(vec![log])
    }

    fn store_logs(&mut self, logs: Vec<Arc<Log>>) -> Result<(), Error> {
        let rst = self.store.store_logs(logs.clone());
        if let Err(e) = rst {
            return Err(Error::UnableToStoreLogs(format!("{}", e)));
        }

        for l in logs {
            self.cache
                .insert((l.index % self.cap as u64) as usize, l.clone());
        }
        Ok(())
    }

    fn delete_range(&mut self, min: u64, max: u64) -> Result<(), Error> {
        self.cache.clear();
        self.store.delete_range(min, max)
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::Error;
    use crate::log::{emit_log_store_metrics, oldest_log, Log, LogCache, LogStore, LogType};
    use crate::mem_metrics::{
        get_gauge, get_registered, setup_mem_metrics, MetricsBasic, MetricsType,
    };
    use crate::mem_store::MemStore;
    use chrono::Utc;
    use metrics::{GaugeValue, Key};
    use std::sync::Arc;
    use std::time::Duration;
    #[cfg(feature = "default")]
    use tokio::{time::sleep};
    #[cfg(not(feature = "default"))]
    use std::thread::sleep;

    fn mock_logs() -> Vec<Arc<Log>> {
        vec![
            Arc::new(Log {
                index: 1234,
                term: 1,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            }),
            Arc::new(Log {
                index: 1235,
                term: 1,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            }),
            Arc::new(Log {
                index: 1236,
                term: 2,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            }),
        ]
    }

    #[test]
    fn test_oldest_log() {
        struct Case {
            name: String,
            logs: Vec<Arc<Log>>,
            want_idx: u64,
            want_err: bool,
        }

        let cases = vec![
            Case {
                name: "empty logs".to_string(),
                logs: vec![],
                want_idx: 0,
                want_err: true,
            },
            Case {
                name: "simple case".to_string(),
                logs: vec![
                    Arc::new(Log {
                        index: 1234,
                        term: 1,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    }),
                    Arc::new(Log {
                        index: 1235,
                        term: 1,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    }),
                    Arc::new(Log {
                        index: 1236,
                        term: 2,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    }),
                ],
                want_idx: 1234,
                want_err: false,
            },
        ];

        for case in cases {
            let mut s = MemStore::new();
            let st = s.store_logs(case.logs);
            assert_eq!((), st.unwrap());

            match oldest_log(s) {
                Ok(got) => {
                    assert!(!case.want_err, "wanted error but got nil");
                    assert_eq!(got.index, case.want_idx);
                }
                Err(e) => {
                    assert!(case.want_err, "wanted no error got {}", e);
                }
            }
        }
    }

    #[cfg(feature = "default")]
    #[tokio::test]
    async fn test_emit_log_store_metrics() {
        setup_mem_metrics();
        let start = Utc::now();

        let mut s = MemStore::new();

        let logs = mock_logs();
        s.store_logs(logs).unwrap();

        let (stop_ch_tx, mut stop_ch_rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            emit_log_store_metrics(
                s,
                "raft.test".to_string(),
                std::time::Duration::from_millis(100),
                &mut stop_ch_rx,
            )
            .await;
        });
        sleep(Duration::from_millis(1000)).await;

        let key = Key::from_static_name("raft.test.oldest.log.age");
        let gv = get_gauge(key.clone()).unwrap();

        match gv {
            GaugeValue::Absolute(val) => {
                assert!(
                    val <= Utc::now().signed_duration_since(start).num_milliseconds() as f64,
                    "max age before test start: {}",
                    val
                );
                assert!(val >= 1.0, "max age less than interval:  {}", val);
            }
            GaugeValue::Increment(_) => panic!("expected absolute value, but got increment value"),
            GaugeValue::Decrement(_) => panic!("expected absolute value, but got decrement value"),
        }

        let bi = get_registered(key.clone()).unwrap();
        assert_eq!(bi, MetricsBasic::from_type(MetricsType::Gauge));
        stop_ch_tx.send(()).await;
    }

    #[cfg(not(feature = "default"))]
    fn test_emit_log_store_metrics() {
        setup_mem_metrics();
        let start = Utc::now();
        let mut s = MemStore::new();
        let logs = mock_logs();
        s.store_logs(logs).unwrap();

        let (stop_ch_tx, mut stop_ch_rx) = crossbeam::channel::bounded::<()>(1);

        std::thread::spawn(move || {
            emit_log_store_metrics(
                s,
                "raft.test".to_string(),
                std::time::Duration::from_millis(100),
                &mut stop_ch_rx,
            );
        });

        sleep(Duration::from_millis(1000));

        let key = Key::from_static_name("raft.test.oldest.log.age");
        let gv = get_gauge(key.clone()).unwrap();

        match gv {
            GaugeValue::Absolute(val) => {
                assert!(
                    val <= Utc::now().signed_duration_since(start).num_milliseconds() as f64,
                    "max age before test start: {}",
                    val
                );
                assert!(val >= 1.0, "max age less than interval:  {}", val);
            }
            GaugeValue::Increment(_) => panic!("expected absolute value, but got increment value"),
            GaugeValue::Decrement(_) => panic!("expected absolute value, but got decrement value"),
        }

        let bi = get_registered(key.clone()).unwrap();
        assert_eq!(bi, MetricsBasic::from_type(MetricsType::Gauge));
        stop_ch_tx.send(()).unwrap();
    }

    #[test]
    fn test_log_cache() {
        let mut store = MemStore::new();
        for i in 0..32 {
            let l = Log {
                index: i + 1,
                term: 0,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            };
            let _ = store.store_log(Arc::new(l));
        }

        let c = LogCache::<MemStore>::new(store, 20);
        // Check the indexes
        let s = c.read();
        let idx = s.first_index().unwrap();
        assert_eq!(idx, 1, "bad: {}", idx);

        let idx = s.last_index().unwrap();
        assert_eq!(idx, 32, "bad: {}", idx);

        // Try get log with a miss
        let l = s.get_log(1).unwrap();
        assert_eq!(l.index, 1, "bad: {}", l.index);
        drop(s);

        // Store Logs
        let l1 = {
            let mut l = Log::default();
            l.index = 33;
            Arc::new(l)
        };
        let l2 = {
            let mut l = Log::default();
            l.index = 34;
            Arc::new(l)
        };

        let mut s = c.write();
        let rst = s.store_logs(vec![l1, l2]).unwrap();
        assert_eq!(rst, ());
        let idx = s.last_index().unwrap();
        assert_eq!(idx, 34, "bad: {}", idx);
        drop(s);

        // Should be in the ring buffer
        let s = c.read();
        let l = s.get_log(33).unwrap();
        assert_eq!(l.index, 33);
        let l = s.get_log(34).unwrap();
        assert_eq!(l.index, 34);
        drop(s);

        // Purge the ring buffer
        let mut s = c.write();
        let x = s.delete_range(33, 34).unwrap();
        assert_eq!(x, ());
        drop(s);

        // Should not be in the ring buffer
        let s = c.read();
        let l = s.get_log(33).unwrap_err();
        assert_eq!(l, Error::LogNotFound);
        let l = s.get_log(34).unwrap_err();
        assert_eq!(l, Error::LogNotFound);
    }
}
