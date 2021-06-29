use crate::errors::Errors;
use anyhow::Result;
use chrono::{DateTime, Utc};
use crossbeam::channel::{select, Receiver};
use derive_getters::Getters;
use metrics::gauge;
use parse_display::{Display, FromStr};
use std::time::Duration;

/// LogType describes various types of log entries
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr)]
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
#[derive(Debug, Clone, Getters)]
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
}

/// LogStore is used to provide an interface for storing
/// and retrieving logs in a durable fashion.
pub trait LogStore {
    /// `first_index` returns the first index written. 0 for no entries.
    fn first_index(&self) -> Result<u64, Errors>;

    /// `last_index` returns the last index written. 0 for no entries.
    fn last_index(&self) -> Result<u64, Errors>;

    /// `get_log` gets a log entry at a given index.
    fn get_log(&self, index: u64) -> Result<Log, Errors>;

    /// `store_log` stores a log entry
    fn store_log(&mut self, log: Log) -> Result<(), Errors>;

    /// `store_logs` stores multiple log entries.
    fn store_logs(&mut self, logs: Vec<Log>) -> Result<(), Errors>;

    /// `delete_range` deletes a range of log entries. The range is inclusive.
    fn delete_range(&mut self, min: u64, max: u64) -> Result<(), Errors>;

    /// `oldest_log` returns the oldest log in the store.
    fn oldest_log(&self) -> Result<Log, Errors> {
        // We might get unlucky and have a truncate right between getting first log
        // index and fetching it so keep trying until we succeed or hard fail.
        let mut last_fail_idx = 0;
        let mut last_err: Errors = Errors::LogNotFound;

        loop {
            let first_idx = self.first_index()?;
            if first_idx == 0 {
                return Err(Errors::LogNotFound);
            }

            if first_idx == last_fail_idx {
                // Got same index as last time around which errored, don't bother trying
                // to fetch it again just return the error
                return Err(last_err);
            }

            match self.get_log(first_idx) {
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

    /// `emit_log_store_metrics` emits the information to the metrics
    fn emit_log_store_metrics(&self, prefix: String, interval: Duration, stop_chan: Receiver<()>) {
        let key = prefix;

        select! {
            recv(stop_chan) -> _ => return,
            default(interval) => {
                // In error case emit 0 as the age
                let mut age_ms = 0i64;
                if let Ok(l) = self.oldest_log() {
                    if l.append_at.timestamp_millis() != 0 {
                        age_ms = Utc::now().signed_duration_since(l.append_at).num_milliseconds();
                    }
                }

                gauge!(vec![key, "oldest.log.age".to_string()].join("."), age_ms as f64);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::log::{Log, LogStore, LogType};
    use crate::mem_metrics::{
        get_gauge, get_registered, setup_mem_metrics, MetricsBasic, MetricsType,
    };
    use crate::mem_store::MemStore;
    use chrono::{DateTime, Utc};
    use metrics::{GaugeValue, Key};
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_oldest_log() {
        struct Case {
            name: String,
            logs: Vec<Log>,
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
                    Log {
                        index: 1234,
                        term: 1,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    },
                    Log {
                        index: 1235,
                        term: 1,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    },
                    Log {
                        index: 1236,
                        term: 2,
                        typ: LogType::Command,
                        data: vec![],
                        extensions: vec![],
                        append_at: Utc::now(),
                    },
                ],
                want_idx: 1234,
                want_err: false,
            },
        ];

        for case in cases {
            let mut s = MemStore::new();
            let st = s.store_logs(case.logs);
            assert_eq!((), st.unwrap());

            match s.oldest_log() {
                Ok(got) => {
                    if case.want_err {
                        panic!("wanted error got nil");
                    }
                    assert_eq!(got.index, case.want_idx);
                }
                Err(e) => {
                    if !case.want_err {
                        panic!("wanted no error got {}", e);
                    }
                }
            }
        }
    }

    #[test]
    fn test_emit_log_store_metrics() {
        setup_mem_metrics();
        let start = Utc::now();

        let mut s = MemStore::new();

        let logs = vec![
            Log {
                index: 1234,
                term: 1,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            },
            Log {
                index: 1235,
                term: 1,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            },
            Log {
                index: 1236,
                term: 2,
                typ: LogType::Command,
                data: vec![],
                extensions: vec![],
                append_at: Utc::now(),
            },
        ];

        s.store_logs(logs).unwrap();

        let (stop_ch_tx, stop_ch_rx) = crossbeam::channel::bounded::<()>(1);

        std::thread::spawn(move || {
            sleep(Duration::from_millis(5));
            stop_ch_tx.send(()).unwrap();
        });

        s.emit_log_store_metrics(
            "raft.test".to_string(),
            std::time::Duration::from_millis(1),
            stop_ch_rx,
        );

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
    }
}
