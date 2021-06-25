use std::fmt::{Display, Formatter, Result as FormatResult};
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use crossbeam::channel::{select, Receiver};
use derive_getters::Getters;
use metrics::gauge;

/// LogType describes various types of log entries
#[derive(Debug, Copy, Clone)]
pub enum LogType {
    /// Command is applied to a user FSM
    Command,

    /// Noop is used to assert leadership
    Noop,

    /// AddPeerDeprecated is used to add a new peer, This should only be used with
    /// older protocol versions designed to be compatible with unversioned
    /// Raft servers. See comments in config.rs for details
    AddPeerDeprecated,

    /// RemovePeerDeprecated is used to remove an existing peer. This should only be
    /// used with older protocol versions designed to be compatible with
    /// unversioned Raft servers. See comments in config.rs for details.
    RemovePeerDeprecated,

    /// Barrier is used to ensure all preceding operations have been
    /// applied to the FSM. It is similar to Noop, but instead of returning
    /// once committed, it only returns once the FSM manager acks it. Otherwise
    /// it is possible there are operations committed but not yet applied to
    /// the FSM
    Barrier,

    /// Configuration establishes a membership change configuration. It is
    /// created when a server is added, removed, promoted, etc. Only used
    /// when protocol version 1 or greater is in use.
    Configuration,
}

impl Display for LogType {
    /// fmt returns LogType as a human readable string
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        match self {
            LogType::Command => write!(f, "Command"),
            LogType::Noop => write!(f, "Noop"),
            LogType::AddPeerDeprecated => write!(f, "AddPeerDeprecated"),
            LogType::RemovePeerDeprecated => write!(f, "RemovePeerDeprecated"),
            LogType::Barrier => write!(f, "Barrier"),
            LogType::Configuration => write!(f, "Configuration"),
        }
    }
}

/// Log entries are replicated to all members of the Raft cluster
/// and form the heart of the replicated state machine.
#[derive(Debug, Clone, Getters)]
pub struct Log {
    /// index holds the index of the log entry
    index: u64,

    /// term holds the election term of the log entry
    term: u64,

    /// typ holds the type of the log entry
    typ: LogType,

    /// data holds the log entry's type-specific data,
    data: Vec<u8>,

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
    extensions: Vec<u8>,

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
    append_at: DateTime<Utc>,
}

pub enum LogStoreError {}

/// LogStore is used to provide an interface for storing
/// and retrieving logs in a durable fashion.
pub trait LogStore {
    /// first_index returns the first index written. 0 for no entries.
    fn first_index(&self) -> Result<u64>;

    /// last_index returns the last index written. 0 for no entries.
    fn last_index(&self) -> Result<u64>;

    /// get_log gets a log entry at a given index.
    fn get_log(&self, index: u64, log: &mut Log) -> Result<()>;

    /// store_log stores a log entry
    fn store_log(&self, log: &Log) -> Result<()>;

    /// store_logs stores multiple log entries.
    fn store_logs(&self, logs: Vec<&Log>) -> Result<()>;

    /// delete_range deletes a range of log entries. The range is inclusive.
    fn delete_range(&self, min: u64, max: u64) -> Result<()>;
}

pub fn oldest_log(s: Box<dyn LogStore>) -> Result<Log> {
    // let mut log: Log;
    // let mut last_fail_idx = -1;
    // let mut last_err: dyn std::error::Error = ();
    unimplemented!()
    // loop {
    //     let first_idx = s.first_index()?;
    //     if first_idx == 0 {
    //         return Err();
    //     }
    //     if first_idx == last_fail_idx {
    //         return last_err;
    //     }
    //     let err = s.get_log(first_idx, &mut log);
    //     if let Ok(_) = err {
    //         break;
    //     }
    //
    //     last_fail_idx = first_idx;
    //     last_err = err.unwrap_err();
    // }
    //
    // Ok(log)
}

pub fn emit_log_store_metrics(
    s: Box<dyn LogStore>,
    prefix: Vec<String>,
    interval: Duration,
    stop_chan: Receiver<()>,
) {
    let key = {
        let pref: String = prefix.join(".");
        pref + "oldestLogAge"
    };

    select! {
        recv(stop_chan) -> sig => return,
        default(interval) => {
            let mut age_ms = 0i64;
            if let Ok(l) = oldest_log(s) {
                if l.append_at.timestamp_millis() != 0 {
                    age_ms = Utc::now().signed_duration_since(l.append_at).num_milliseconds();
                }
            }
            
            gauge!(key, age_ms as f64);
        },
    }
}

#[cfg(test)]
mod tests {
    use metrics::gauge;
    #[test]
    fn test_gauge() {
        gauge!(
            vec![String::from("vbc"), String::from("asdasd")].join("."),
            6.8,
        );
    }
}
