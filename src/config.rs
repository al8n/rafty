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
use crate::errors::Errors;
use crate::fsm::{FSMSnapshot, FSM};
use crate::log::Log;
use parse_display::{Display, FromStr};
use rmps::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Cursor;
use std::time::Duration;
#[cfg(not(feature = "default"))]
use crossbeam::channel::Receiver;
#[cfg(feature = "default")]
use tokio::{
    io::AsyncRead,
    sync::mpsc::UnboundedReceiver
};
use std::sync::Arc;


static DEFAULT_HEARTBEAT_TIMEOUT: Duration = Duration::from_millis(1000);
static DEFAULT_ELECTION_TIMEOUT: Duration = Duration::from_millis(1000);
static DEFAULT_COMMIT_TIMEOUT: Duration = Duration::from_millis(50);
static DEFAULT_MAX_APPEND_ENTRIES: u64 = 64;
static DEFAULT_BATCH_APPLY_CH: bool = false;
static DEFAULT_SHUTDOWN_ON_REMOVE: bool = true;
static DEFAULT_TRAILING_LOGS: u64 = 10240;
static DEFAULT_SNAPSHOT_INTERVAL: Duration = Duration::from_secs(120);
static DEFAULT_SNAPSHOT_THRESHOLD: u64 = 8192;
static DEFAULT_LEADER_LEASE_TIMEOUT: Duration = Duration::from_millis(500);
static DEFAULT_LOG_LEVEL: &'static str = "DEBUG";
static DEFAULT_NO_SNAPSHOT_RESTORE_ON_START: bool = false;
static DEFAULT_STARTUP: bool = false;

/// `ProtocolVersion` is the version of the protocol (which includes RPC messages
/// as well as Raft-specific log entries) that this server can _understand_. Use
/// the `protocol_version` member of the `Config` object to control the version of
/// the protocol to use when _speaking_ to other servers. Note that depending on
/// the protocol version being spoken, some otherwise understood RPC messages
/// may be refused. See dispositionRPC for details of this logic.
///
/// There are notes about the upgrade path in the description of the versions
/// below. If you are starting a fresh cluster then there's no reason not to
/// jump right to the latest protocol version.
///
/// **Version History**
///
/// 0: Protocol adding full support for server IDs and new ID-based server APIs
/// (AddVoter, AddNonvoter, etc.)
///
/// 1: Protocol adding full support for server IDs and new ID-based server APIs
/// (AddVoter, AddNonvoter, etc.)
///
/// **N.B.** You may notice that version 0 and version 1 have the same definition. For now, they are the same, but version 1 aims to prepare for future change.
///
#[derive(Copy, Clone, Display, FromStr, Eq, PartialEq, Debug)]
#[display(style = "CamelCase")]
pub enum ProtocolVersion {
    /// `ProtocolVersionMin` is the minimum protocol version
    ProtocolVersionMin = 0,

    /// `ProtocolVersionMax` is the maximum protocol version
    ProtocolVersionMax = 1,
}

/// `SnapshotVersion` is the version of snapshots that this server can understand.
/// Currently, it is always assumed that the server generates the latest version,
/// though this may be changed in the future to include a configurable version.
///
/// **Version History**
///
/// 0: New format which adds support for a full configuration structure and its
/// associated log index, with support for server IDs and non-voting server
/// modes. To ease upgrades, this also includes the legacy peers structure but
/// that will never be used by servers that understand version 1 snapshots.
/// Since the original Raft library didn't enforce any versioning, we must
/// include the legacy peers structure for this version, but we can deprecate
/// it in the next snapshot version.
///
/// 1: New format which adds support for a full configuration structure and its
/// associated log index, with support for server IDs and non-voting server
/// modes. To ease upgrades, this also includes the legacy peers structure but
/// that will never be used by servers that understand version 1 snapshots.
/// Since the original Raft library didn't enforce any versioning, we must
/// include the legacy peers structure for this version, but we can deprecate
/// it in the next snapshot version.
///
/// **N.B.** You may notice that version 0 and version 1 have the same definition. For now, they are the same, but version 1 aims to prepare for future change.
#[derive(Copy, Clone, Display, FromStr, Eq, PartialEq, Debug)]
#[display(style = "CamelCase")]
pub enum SnapshotVersion {
    /// `SnapshotVersionMin` is the minimum snapshot version
    SnapshotVersionMin = 0,

    /// `SnapshotVersionMax` is the maximum snapshot version
    SnapshotVersionMax = 1,
}

/// `Config` provides any necessary configuration for the `Raft` server.
#[derive(Debug, Clone)]
pub struct Config {
    /// `protocol_version` allows a Raft server to inter-operate with older
    /// Raft servers running an older version of the code. This is used to
    /// version the wire protocol as well as Raft-specific log entries that
    /// the server uses when _speaking_ to other servers. There is currently
    /// no auto-negotiation of versions so all servers must be manually
    /// configured with compatible versions. See ProtocolVersionMin and
    /// ProtocolVersionMax for the versions of the protocol that this server
    /// can _understand_.
    protocol_version: ProtocolVersion,

    /// `heartbeat_timeout` specifies the time in follower state without a leader before we attempt an election
    heartbeat_timeout: Duration,

    /// `election_timeout` specifies the time in candidate state without a leader before we attempt an election.
    election_timeout: Duration,

    /// `commit_timeout` controls the time without an Apply operation
    /// before we heartbeat to ensure a timely commit. Due to random
    /// staggering, may be delayed as much as 2x this value.
    commit_timeout: Duration,

    /// `max_append_entries` controls the maximum number of append entries
    /// to send at once. We want to strike a balance between efficiency
    /// and avoiding waste if the follower is going to reject because of
    /// an inconsistent log.
    max_append_entries: u64,

    /// `batch_apply_ch` indicates whether we should buffer `apply_ch`
    /// to size `max_append_entries`. This enables batch log commitment,
    /// but breaks the timeout guarantee on apply. Specifically,
    /// a log can be added to the `apply_ch` buffer but not actually be
    /// processed until after the specified timeout.
    batch_apply_ch: bool,

    /// If we are a member of a cluster, and `remove_peer` is invoked for the
    /// local node, then we forget all peers and transition into the follower state.
    /// If `shut_down_on_remove` is set, we additional shutdown Raft. Otherwise,
    /// we can become a leader of a cluster containing only this node.
    shut_down_on_remove: bool,

    /// `trailing_logs` controls how many logs we leave after a snapshot. This is used
    /// so that we can quickly replay logs on a follower instead of being forced to
    /// send an entire snapshot. The value passed here is the initial setting used.
    /// This can be tuned during operation using `reload_config`.
    trailing_logs: u64,

    /// `snapshot_interval` controls how often we check if we should perform a
    /// snapshot. We randomly stagger between this value and 2x this value to avoid
    /// the entire cluster from performing a snapshot at once. The value passed here is the initial setting used. This can be tuned during operation using `reload_config`.
    snapshot_interval: Duration,

    /// `snapshot_threshold` controls how many outstanding logs there must be before
    /// we perform a snapshot. This is to prevent excessive snapshotting by
    /// replaying a small set of logs instead. The value passed here is the initial setting used. This can be tuned during operation using `reload_config`.
    snapshot_threshold: u64,

    /// `leader_lease_timeout` is used to control how long the "lease" lasts
    /// for being the leader without being able to contract a quorum
    /// of nodes. If we reach this interval without contact, we will
    /// step down as leader.
    leader_lease_timeout: Duration,

    /// `local_id` is a unique ID for this server across all time.`
    local_id: ServerID,

    /// `notify_ch` is used to provide a channel that will be notified of leadership
    /// changes. Raft will block writing to this channel, so it should either be
    /// buffered or aggressively consumed.
    #[cfg(not(feature = "default"))]
    notify_ch: Receiver<bool>,

    #[cfg(feature = "default")]
    notify_ch: Arc<UnboundedReceiver<bool>>,

    // TODO: log related fields start
    /// `log_output` is used as a sink for logs, unless `logger` is specified.
    /// Defaults to os.stderr
    // log_output: io.Writer
    /// `log_level` represents a log level. If the value does not match a known
    /// logging level
    // log_level: String,

    // logger: hclog.Logger
    // TODO: log related fields end

    /// `no_snapshot_restore_on_start` controls if raft will restore a snapshot to the
    /// `FSM` on start. This is useful if your `FSM` recovers from other mechanisms
    /// than raft snapshotting. Snapshot metadata will still be used to initialize
    /// raft's configuration and index values.
    no_snapshot_restore_on_start: bool,

    /// `skip_startup` allows `new_raft` to bypass all background work threads.
    skip_startup: bool,
}

impl Config {
    /// `validate_config` is used to validate a sane configuration
    #[inline]
    fn validate_config(self) -> Result<Self, Errors> {
        if self.local_id == 0 {
            return Err(Errors::EmptyLocalID);
        }

        if self.heartbeat_timeout < Duration::from_millis(5) {
            return Err(Errors::ShortHeartbeatTimeout);
        }

        if self.election_timeout < Duration::from_millis(5) {
            return Err(Errors::ShortElectionTimeout);
        }

        if self.commit_timeout < Duration::from_millis(1) {
            return Err(Errors::ShortCommitTimeout);
        }

        if self.max_append_entries > 1024 {
            return Err(Errors::LargeMaxAppendEntries);
        }

        if self.snapshot_interval < Duration::from_millis(5) {
            return Err(Errors::ShortSnapshotInterval);
        }

        if self.leader_lease_timeout < Duration::from_millis(5) {
            return Err(Errors::ShortLeaderLeaseTimeout);
        }

        if self.leader_lease_timeout > self.heartbeat_timeout {
            return Err(Errors::LeaderLeaseTimeoutLargerThanHeartbeatTimeout);
        }

        if self.election_timeout < self.heartbeat_timeout {
            return Err(Errors::ElectionTimeoutSmallerThanHeartbeatTimeout);
        }

        Ok(self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ConfigBuilder {
    /// `protocol_version` allows a Raft server to inter-operate with older
    /// Raft servers running an older version of the code. This is used to
    /// version the wire protocol as well as Raft-specific log entries that
    /// the server uses when _speaking_ to other servers. There is currently
    /// no auto-negotiation of versions so all servers must be manually
    /// configured with compatible versions. See ProtocolVersionMin and
    /// ProtocolVersionMax for the versions of the protocol that this server
    /// can _understand_.
    protocol_version: Option<ProtocolVersion>,

    /// `heartbeat_timeout` specifies the time in follower state without a leader before we attempt an election
    heartbeat_timeout: Option<Duration>,

    /// `election_timeout` specifies the time in candidate state without a leader before we attempt an election.
    election_timeout: Option<Duration>,

    /// `commit_timeout` controls the time without an Apply operation
    /// before we heartbeat to ensure a timely commit. Due to random
    /// staggering, may be delayed as much as 2x this value.
    commit_timeout: Option<Duration>,

    /// `max_append_entries` controls the maximum number of append entries
    /// to send at once. We want to strike a balance between efficiency
    /// and avoiding waste if the follower is going to reject because of
    /// an inconsistent log.
    max_append_entries: Option<u64>,

    /// `batch_apply_ch` indicates whether we should buffer `apply_ch`
    /// to size `max_append_entries`. This enables batch log commitment,
    /// but breaks the timeout guarantee on apply. Specifically,
    /// a log can be added to the `apply_ch` buffer but not actually be
    /// processed until after the specified timeout.
    batch_apply_ch: Option<bool>,

    /// If we are a member of a cluster, and `remove_peer` is invoked for the
    /// local node, then we forget all peers and transition into the follower state.
    /// If `shut_down_on_remove` is set, we additional shutdown Raft. Otherwise,
    /// we can become a leader of a cluster containing only this node.
    shut_down_on_remove: Option<bool>,

    /// `trailing_logs` controls how many logs we leave after a snapshot. This is used
    /// so that we can quickly replay logs on a follower instead of being forced to
    /// send an entire snapshot. The value passed here is the initial setting used.
    /// This can be tuned during operation using `reload_config`.
    trailing_logs: Option<u64>,

    /// `snapshot_interval` controls how often we check if we should perform a
    /// snapshot. We randomly stagger between this value and 2x this value to avoid
    /// the entire cluster from performing a snapshot at once. The value passed here is the initial setting used. This can be tuned during operation using `reload_config`.
    snapshot_interval: Option<Duration>,

    /// `snapshot_threshold` controls how many outstanding logs there must be before
    /// we perform a snapshot. This is to prevent excessive snapshotting by
    /// replaying a small set of logs instead. The value passed here is the initial setting used. This can be tuned during operation using `reload_config`.
    snapshot_threshold: Option<u64>,

    /// `leader_lease_timeout` is used to control how long the "lease" lasts
    /// for being the leader without being able to contract a quorum
    /// of nodes. If we reach this interval without contact, we will
    /// step down as leader.
    leader_lease_timeout: Option<Duration>,

    /// `local_id` is a unique ID for this server across all time.`
    local_id: Option<ServerID>,

    // TODO: log related fields start
    /// `log_output` is used as a sink for logs, unless `logger` is specified.
    /// Defaults to os.stderr
    // log_output: io.Writer
    /// `log_level` represents a log level. If the value does not match a known
    /// logging level
    // log_level: String,

    // logger: hclog.Logger
    // TODO: log related fields end

    /// `no_snapshot_restore_on_start` controls if raft will restore a snapshot to the
    /// `FSM` on start. This is useful if your `FSM` recovers from other mechanisms
    /// than raft snapshotting. Snapshot metadata will still be used to initialize
    /// raft's configuration and index values.
    no_snapshot_restore_on_start: Option<bool>,

    /// `skip_startup` allows `new_raft` to bypass all background work threads.
    skip_startup: Option<bool>,
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            protocol_version: Some(ProtocolVersion::ProtocolVersionMax),
            heartbeat_timeout: Some(DEFAULT_HEARTBEAT_TIMEOUT),
            election_timeout: Some(DEFAULT_ELECTION_TIMEOUT),
            commit_timeout: Some(DEFAULT_COMMIT_TIMEOUT),
            max_append_entries: Some(DEFAULT_MAX_APPEND_ENTRIES),
            batch_apply_ch: Some(DEFAULT_BATCH_APPLY_CH),
            shut_down_on_remove: Some(DEFAULT_SHUTDOWN_ON_REMOVE),
            trailing_logs: Some(DEFAULT_TRAILING_LOGS),
            snapshot_interval: Some(DEFAULT_SNAPSHOT_INTERVAL),
            snapshot_threshold: Some(DEFAULT_SNAPSHOT_THRESHOLD),
            leader_lease_timeout: Some(DEFAULT_LEADER_LEASE_TIMEOUT),
            local_id: None,
            no_snapshot_restore_on_start: Some(DEFAULT_NO_SNAPSHOT_RESTORE_ON_START),
            skip_startup: Some(DEFAULT_STARTUP),
        }
    }
}

impl ConfigBuilder {
    /// `set_protocol_version` set the value of `protocol_version` in `ConfigBuilder`.
    #[inline]
    pub const fn set_protocol_version(mut self, protocol_version: ProtocolVersion) -> Self {
        self.protocol_version = Some(protocol_version);
        self
    }

    /// `set_local_id` set the value of `local_id` in `ConfigBuilder`.
    #[inline]
    pub const fn set_local_id(mut self, id: ServerID) -> Self {
        self.local_id = Some(id);
        self
    }

    /// `set_heartbeat_timeout` set the value of `heartbeat_timeout` in `ConfigBuilder`.
    #[inline]
    pub const fn set_heartbeat_timeout(mut self, timeout: Duration) -> Self {
        self.heartbeat_timeout = Some(timeout);
        self
    }

    /// `set_election_timeout` set the value of `election_timeout` in `ConfigBuilder`.
    #[inline]
    pub const fn set_election_timeout(mut self, timeout: Duration) -> Self {
        self.election_timeout = Some(timeout);
        self
    }

    /// `set_commit_timeout` set the value of `commit_timeout` in `ConfigBuilder`.
    #[inline]
    pub const fn set_commit_timeout(mut self, timeout: Duration) -> Self {
        self.commit_timeout = Some(timeout);
        self
    }

    /// `set_max_append_entries` set the value of `max_append_entries` in `ConfigBuilder`.
    #[inline]
    pub const fn set_max_append_entries(mut self, max_entries: u64) -> Self {
        self.max_append_entries = Some(max_entries);
        self
    }

    /// `set_batch_apply_ch` set the value of `batch_apply_ch` in `ConfigBuilder`.
    #[inline]
    pub const fn set_batch_apply_ch(mut self, batch_apply_ch: bool) -> Self {
        self.batch_apply_ch = Some(batch_apply_ch);
        self
    }

    /// `set_shut_down_on_remove` set the value of `shut_down_on_remove` in `ConfigBuilder`.
    #[inline]
    pub const fn set_shut_down_on_remove(mut self, shut_down_on_remove: bool) -> Self {
        self.shut_down_on_remove = Some(shut_down_on_remove);
        self
    }

    /// `set_trailing_logs` set the value of `trailing_logs` in `ConfigBuilder`.
    #[inline]
    pub const fn set_trailing_logs(mut self, trailing_logs: u64) -> Self {
        self.trailing_logs = Some(trailing_logs);
        self
    }

    /// `set_snapshot_interval` set the value of `snapshot_interval` in `ConfigBuilder`
    #[inline]
    pub const fn set_snapshot_interval(mut self, timeout: Duration) -> Self {
        self.snapshot_interval = Some(timeout);
        self
    }

    /// `set_snapshot_threshold` set the value of `snapshot_threshold` in `ConfigBuilder`
    #[inline]
    pub const fn set_snapshot_threshold(mut self, threshold: u64) -> Self {
        self.snapshot_threshold = Some(threshold);
        self
    }

    /// `set_leader_lease_timeout` set the value of `leader_lease_timeout` in `ConfigBuilder`
    #[inline]
    pub const fn set_leader_lease_timeout(mut self, timeout: Duration) -> Self {
        self.leader_lease_timeout = Some(timeout);
        self
    }

    /// `set_no_snapshot_restore_on_start` set the value of `no_snapshot_restore_on_start` in `ConfigBuilder`
    #[inline]
    pub const fn set_no_snapshot_restore_on_start(
        mut self,
        no_snapshot_restore_on_start: bool,
    ) -> Self {
        self.no_snapshot_restore_on_start = Some(no_snapshot_restore_on_start);
        self
    }

    /// `set_skip_startup` set the value of `skip_startup` in `ConfigBuilder`
    #[inline]
    pub const fn set_skip_startup(mut self, skip_startup: bool) -> Self {
        self.skip_startup = Some(skip_startup);
        self
    }

    /// `finalize` returns a `Result<Config, Errors>`
    #[cfg(feature = "default")]
    #[inline]
    pub fn finalize(
        self,
        notify_ch: UnboundedReceiver<bool>,
    ) -> Result<Config, Errors> {
        let c = Config {
            protocol_version: self.protocol_version.unwrap(),
            heartbeat_timeout: self.heartbeat_timeout.unwrap(),
            election_timeout: self.election_timeout.unwrap(),
            commit_timeout: self.commit_timeout.unwrap(),
            max_append_entries: self.max_append_entries.unwrap(),
            batch_apply_ch: self.batch_apply_ch.unwrap(),
            shut_down_on_remove: self.shut_down_on_remove.unwrap(),
            trailing_logs: self.trailing_logs.unwrap(),
            snapshot_interval: self.snapshot_interval.unwrap(),
            snapshot_threshold: self.snapshot_threshold.unwrap(),
            leader_lease_timeout: self.leader_lease_timeout.unwrap(),
            local_id: self.local_id.unwrap(),
            notify_ch: Arc::new(notify_ch),
            no_snapshot_restore_on_start: self.no_snapshot_restore_on_start.unwrap(),
            skip_startup: self.skip_startup.unwrap(),
        };

        c.validate_config()
    }

    /// `finalize` returns a `Result<Config, Errors>`
    #[cfg(not(feature = "default"))]
    #[inline]
    pub fn finalize(
        self,
        notify_ch: Receiver<bool>,
    ) -> Result<Config, Errors> {
        let c = Config {
            protocol_version: self.protocol_version.unwrap(),
            heartbeat_timeout: self.heartbeat_timeout.unwrap(),
            election_timeout: self.election_timeout.unwrap(),
            commit_timeout: self.commit_timeout.unwrap(),
            max_append_entries: self.max_append_entries.unwrap(),
            batch_apply_ch: self.batch_apply_ch.unwrap(),
            shut_down_on_remove: self.shut_down_on_remove.unwrap(),
            trailing_logs: self.trailing_logs.unwrap(),
            snapshot_interval: self.snapshot_interval.unwrap(),
            snapshot_threshold: self.snapshot_threshold.unwrap(),
            leader_lease_timeout: self.leader_lease_timeout.unwrap(),
            local_id: self.local_id.unwrap(),
            notify_ch,
            no_snapshot_restore_on_start: self.no_snapshot_restore_on_start.unwrap(),
            skip_startup: self.skip_startup.unwrap(),
        };

        c.validate_config()
    }
}

/// `ReloadableConfig` is the subset of `Config` that may be reconfigured during
/// runtime using raft.ReloadConfig. We choose to duplicate fields over embedding
/// or accepting a `Config` but only using specific fields to keep the API clear.
/// Reconfiguring some fields is potentially dangerous so we should only
/// selectively enable it for fields where that is allowed.
pub struct ReloadableConfig {
    /// `trailing_logs` controls how many logs we leave after a snapshot. This is used
    /// so that we can quickly replay logs on a follower instead of being forced to
    /// send an entire snapshot. The value passed here updates the setting at runtime
    /// which will take effect as soon as the next snapshot completes and truncation
    // occurs.
    trailing_logs: u64,

    /// `snapshot_interval` controls how often we check if we should perform a snapshot.
    /// We randomly stagger between this value and 2x this value to avoid the entire
    /// cluster from performing a snapshot at once.
    snapshot_interval: Duration,

    /// `snapshot_threshold` controls how many outstanding logs there must be before
    /// we perform a snapshot. This is to prevent excessive snapshots when we can
    /// just replay a small set of logs.
    snapshot_threshold: u64,
}

impl ReloadableConfig {
    /// `apply` sets the reloadable fields on the passed Config to the values in
    /// `ReloadableConfig`. It returns a copy of `Config` with the fields from this
    /// `ReloadableConfig` set.
    pub fn apply(&self, to: Config) -> Config {
        if self == &to {
            return to;
        }

        let mut toc = to.clone();
        toc.trailing_logs = self.trailing_logs;
        toc.snapshot_threshold = self.snapshot_threshold;
        toc.snapshot_interval = self.snapshot_interval;
        toc
    }

    /// `from_config` copies the reloadable fields from the passed `Config`.
    pub fn from_config(from: Config) -> Self {
        Self {
            trailing_logs: from.trailing_logs,
            snapshot_interval: from.snapshot_interval,
            snapshot_threshold: from.snapshot_threshold,
        }
    }
}

impl Default for ReloadableConfig {
    fn default() -> Self {
        Self {
            trailing_logs: DEFAULT_TRAILING_LOGS,
            snapshot_interval: DEFAULT_SNAPSHOT_INTERVAL,
            snapshot_threshold: DEFAULT_SNAPSHOT_THRESHOLD,
        }
    }
}

impl PartialEq<Config> for ReloadableConfig {
    fn eq(&self, other: &Config) -> bool {
        self.trailing_logs == other.trailing_logs
            && self.snapshot_threshold == other.snapshot_threshold
            && self.snapshot_interval == other.snapshot_interval
    }
}

/// `ServerSuffrage` determines whether a `Server` in a `Configuration` gets a vote.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr, Serialize, Deserialize)]
#[display(style = "CamelCase")]
pub enum ServerSuffrage {
    /// `Voter` is a server whose vote is counted in elections and whose match index
    /// is used in advancing the leader's commit index.
    Voter,

    /// `Nonvoter` is a server that receives log entries but is not considered for
    /// elections or commitment purposes.
    Nonvoter,

    /// `Staging` is a server that acts like a nonvoter with one exception: once a
    /// staging server receives enough log entries to be sufficiently caught up to
    /// the leader's log, the leader will invoke a membership change to change
    /// the `Staging` server to a `Voter`.
    Staging,
}

/// `ServerID` is a unique string identifying a server for all time.
pub type ServerID = u64;

/// `ServerAddress` is a network address for a server that a transport can contact.
pub type ServerAddress = String;
// #[derive(Display, FromStr, Copy, Clone, Eq, PartialEq, Debug)]
// #[display(style = "CamelCase")]
// pub enum ServerAddress {
//     /// `IPv4` stands for an IPv4 address
//     #[display("IPv4: {0}")]
//     IPv4(Ipv4Addr),
//
//     /// `IPv6` stands for an IPv6 address
//     #[display("IPv6: {0}")]
//     IPv6(Ipv6Addr),
// }

/// `Server` tracks the information about a single server in a configuration.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Server {
    /// `suffrage` determines whether the server gets a vote.
    pub suffrage: ServerSuffrage,

    /// `id` is a unique number ([Sonyflake distributed unique ID generator](https://github.com/sony/sonyflake) ) identifying this server for all time.
    ///
    /// Thanks for Arne Bahlo, the author of [sonyflake-rs](https://github.com/bahlo/sonyflake-rs).
    pub id: ServerID,

    /// `address` is its network address that a transport can contact.
    pub address: ServerAddress,
}

cfg_test!(
    impl std::fmt::Display for Server {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{} {} {}", self.suffrage, self.id, self.address)
        }
    }
);

/// `Configuration` tracks which servers are in the cluster, and whether they have
/// votes. This should include the local server, if it's a member of the cluster.
/// The servers are listed no particular order, but each should only appear once.
/// These entries are appended to the log during membership changes.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Configuration {
    pub servers: Vec<Server>,
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    pub fn with_servers(servers: Vec<Server>) -> Self {
        Self { servers }
    }
}

cfg_test!(
    impl std::fmt::Display for Configuration {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let x = self
                .servers
                .iter()
                .map(|val| format!("{{{}}}", *val))
                .collect::<Vec<String>>();
            write!(f, "{{[{}]}}", x.join(" "))
        }
    }
);


/// `ConfigurationStore` provides an interface that can optionally be implemented by FSMs
/// to store configuration updates made in the replicated log. In general this is only
/// necessary for FSMs that mutate durable state directly instead of applying changes
/// in memory and snapshotting periodically. By storing configuration changes, the
/// persistent FSM state can behave as a complete snapshot, and be able to recover
/// without an external snapshot just for persisting the raft configuration.
pub trait ConfigurationStore<T>: FSM<T> {
    /// ConfigurationStore is a superset of the FSM functionality

    /// StoreConfiguration is invoked once a log entry containing a configuration
    /// change is committed. It takes the index at which the configuration was
    /// written and the configuration value.
    fn store_configuration(&self, index: u64, cfg: Configuration);
}

struct NopConfigurationStore;

impl<T> FSM<T> for NopConfigurationStore {
    fn apply(&self, l: Log) -> T {
        todo!()
    }

    fn snapshot(&self) -> Box<dyn FSMSnapshot> {
        todo!()
    }

    fn restore(&self, r: Box<dyn AsyncRead>) -> anyhow::Result<(), std::io::Error> {
        todo!()
    }
}

impl<T> ConfigurationStore<T> for NopConfigurationStore {
    fn store_configuration(&self, index: u64, cfg: Configuration) {}
}

#[derive(Display, FromStr, Debug, Copy, Clone, Eq, PartialEq)]
#[display(style = "CamelCase")]
pub enum ConfigurationChangeCommand {
    /// `AddStaging` makes a server Staging unless its Voter.
    AddStaging,
    /// `AddNonvoter` makes a server Nonvoter unless its Staging or Voter.
    AddNonvoter,
    /// `DemoteVoter` makes a server Nonvoter unless its absent.
    DemoteVoter,
    /// `RemoveServer` removes a server entirely from the cluster membership.
    RemoveServer,
    /// `Promote` is created automatically by a leader; it turns a Staging server
    /// into a Voter.
    Promote,
}

/// `ConfigurationChangeRequest` describes a change that a leader would like to
/// make to its current configuration. It's used only within a single server
/// (never serialized into the log), as part of `ConfigurationChangeFuture`.
#[derive(Debug, Clone, Eq, PartialEq)]
struct ConfigurationChangeRequest {
    command: ConfigurationChangeCommand,
    server_id: ServerID,
    server_address: ServerAddress, // only present for `AddStaging`, `AddNonvoter`

    /// `prev_index`, if nonzero, is the index of the only configuration upon which
    /// this change may be applied; if another configuration entry has been
    /// added in the meantime, this request will fail.
    prev_index: u64,
}

/// `Configurations` is state tracked on every server about its Configurations.
/// Note that, per Diego's dissertation, there can be at most one uncommitted
/// configuration at a time (the next configuration may not be created until the
/// prior one has been committed).
///
/// One downside to storing just two configurations is that if you try to take a
/// snapshot when your state machine hasn't yet applied the committedIndex, we
/// have no record of the configuration that would logically fit into that
/// snapshot. We disallow snapshots in that case now. An alternative approach,
/// which LogCabin uses, is to track every configuration change in the
/// log.
struct Configurations {
    /// `committed` is the `latest` configuration in the log/snapshot that has been
    /// `committed` (the one with the largest index).
    committed: Configuration,

    /// `committed_index` is the log index where 'committed' was written.
    committed_index: u64,

    /// `latest` is the latest configuration in the log/snapshot (may be committed
    /// or uncommitted)
    latest: Configuration,

    /// latest_index is the log index where 'latest' was written.
    latest_index: u64,
}

/// `has_vote` returns true if the server identified by 'id' is a Voter in the
/// provided `Configuration`.
fn has_vote(configuration: Configuration, id: ServerID) -> bool {
    for s in configuration.servers {
        if s.id == id {
            return s.suffrage == ServerSuffrage::Voter;
        }
    }
    false
}

/// `check_configuration` tests a cluster membership configuration for common
/// errors.
fn check_configuration(configuration: Configuration) -> Result<Configuration, Errors> {
    let mut id_set = HashMap::<ServerID, bool>::new();
    let mut address_set = HashMap::<ServerAddress, bool>::new();
    let mut voters = 0;
    for s in &configuration.servers {
        // TODO: check whether server id is valid

        if let Some(_) = id_set.get(&s.id) {
            return Err(Errors::DuplicateServerID(s.id));
        }
        id_set.insert(s.id, true);

        if let Some(_) = address_set.get(&s.address) {
            return Err(Errors::DuplicateServerAddress(s.clone().address));
        }

        address_set.insert(s.clone().address, true);
        if s.suffrage == ServerSuffrage::Voter {
            voters += 1;
        }
    }

    if voters == 0 {
        return Err(Errors::NonVoter);
    }

    Ok(configuration)
}

/// `next_configuration` generates a new `Configuration` from the current one and a
/// configuration change request. It's split from `append_configuration_entry` so
/// that it can be unit tested easily.
fn next_configuration(
    current: Configuration,
    current_index: u64,
    change: ConfigurationChangeRequest,
) -> Result<Configuration, Errors> {
    if change.prev_index > 0 && change.prev_index != current_index {
        return Err(Errors::ConfigurationChanged {
            current_index,
            prev_index: change.prev_index,
        });
    }

    let mut configuration = current.clone();

    match change.command {
        ConfigurationChangeCommand::AddStaging => {
            // TODO: barf on new address?
            let new_server = Server {
                // TODO: This should add the server as Staging, to be automatically
                // promoted to Voter later. However, the promotion to Voter is not yet
                // implemented, and doing so is not trivial with the way the leader loop
                // coordinates with the replication goroutines today. So, for now, the
                // server will have a vote right away, and the Promote case below is
                // unused.
                suffrage: ServerSuffrage::Voter,
                id: change.clone().server_id,
                address: change.clone().server_address,
            };

            let mut found = false;
            for (idx, s) in configuration.servers.iter().enumerate() {
                if s.id == change.server_id {
                    if s.suffrage == ServerSuffrage::Voter {
                        configuration.servers[idx].address = change.server_address;
                    } else {
                        configuration.servers[idx] = new_server.clone();
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                configuration.servers.push(new_server);
            }
        }
        ConfigurationChangeCommand::AddNonvoter => {
            let new_server = Server {
                suffrage: ServerSuffrage::Nonvoter,
                id: change.clone().server_id,
                address: change.clone().server_address,
            };

            let mut found = false;
            for (idx, s) in configuration.servers.iter().enumerate() {
                if s.id == change.server_id {
                    if s.suffrage != ServerSuffrage::Nonvoter {
                        configuration.servers[idx].address = change.server_address;
                    } else {
                        configuration.servers[idx] = new_server.clone();
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                configuration.servers.push(new_server);
            }
        }
        ConfigurationChangeCommand::DemoteVoter => {
            for (idx, s) in configuration.servers.iter().enumerate() {
                if s.id == change.server_id {
                    configuration.servers[idx].suffrage = ServerSuffrage::Nonvoter;
                    break;
                }
            }
        }
        ConfigurationChangeCommand::RemoveServer => {
            for (idx, s) in configuration.servers.iter().enumerate() {
                if s.id == change.server_id {
                    configuration.servers.remove(idx);
                    break;
                }
            }
        }
        ConfigurationChangeCommand::Promote => {
            for (idx, s) in configuration.servers.iter().enumerate() {
                if s.id == change.server_id && s.suffrage == ServerSuffrage::Staging {
                    configuration.servers[idx].suffrage = ServerSuffrage::Voter;
                    break;
                }
            }
        }
    }

    // Make sure we didn't do something bad like remove the last voter
    match check_configuration(configuration) {
        Ok(c) => Ok(c),
        Err(e) => Err(e),
    }
}

/// `encode_configuration` serializes a `Configuration` using MsgPack, or panics on
/// errors.
pub fn encode_configuration(configuration: Configuration) -> Vec<u8> {
    let mut buf = Vec::<u8>::new();
    match configuration.serialize(&mut Serializer::new(&mut buf)) {
        Ok(_) => buf,
        Err(e) => panic!("{}", e),
    }
}

/// `decode_configuration` deserializes a Configuration using MsgPack, or panics on
/// errors.
pub fn decode_configuration(buf: Vec<u8>) -> Configuration {
    let cur = Cursor::new(&buf[..]);
    let mut de = Deserializer::new(cur);

    let cfg: Configuration = Deserialize::deserialize(&mut de).unwrap();
    cfg
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::ConfigurationChangeCommand::AddNonvoter;
    use crate::config::ProtocolVersion::ProtocolVersionMax;
    #[cfg(not(feature = "default"))]
    use crossbeam::channel::unbounded;

    #[cfg(feature = "default")]
    use tokio::sync::mpsc::unbounded_channel;

    fn sample_configuration() -> Configuration {
        Configuration::with_servers(vec![
            Server {
                suffrage: ServerSuffrage::Nonvoter,
                id: 0,
                address: "addr0".to_string(),
            },
            Server {
                suffrage: ServerSuffrage::Voter,
                id: 1,
                address: "addr1".to_string(),
            },
            Server {
                suffrage: ServerSuffrage::Staging,
                id: 2,
                address: "addr2".to_string(),
            },
        ])
    }

    fn single_server() -> Configuration {
        Configuration::with_servers(vec![Server {
            suffrage: ServerSuffrage::Voter,
            id: 1,
            address: "addr1x".to_string(),
        }])
    }

    fn one_of_each() -> Configuration {
        Configuration::with_servers(vec![
            Server {
                suffrage: ServerSuffrage::Voter,
                id: 1,
                address: "addr1x".to_string(),
            },
            Server {
                suffrage: ServerSuffrage::Staging,
                id: 2,
                address: "addr2x".to_string(),
            },
            Server {
                suffrage: ServerSuffrage::Nonvoter,
                id: 3,
                address: "addr3x".to_string(),
            },
        ])
    }

    fn voter_pair() -> Configuration {
        Configuration::with_servers(vec![
            Server {
                suffrage: ServerSuffrage::Voter,
                id: 1,
                address: "addr1x".to_string(),
            },
            Server {
                suffrage: ServerSuffrage::Voter,
                id: 2,
                address: "addr2x".to_string(),
            },
        ])
    }

    #[derive(Clone)]
    struct NextConfigurationTests {
        current: Configuration,
        command: ConfigurationChangeCommand,
        server_id: u64,
        next: String,
    }

    fn next_configuration_tests() -> Vec<NextConfigurationTests> {
        vec![
            // AddStaging: was missing
            NextConfigurationTests {
                current: Configuration::new(),
                command: ConfigurationChangeCommand::AddStaging,
                server_id: 1,
                next: "{[{Voter 1 addr1}]}".to_string(),
            },
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::AddStaging,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Voter 2 addr2}]}".to_string(),
            },
            // AddStaging: was Voter.
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::AddStaging,
                server_id: 1,
                next: "{[{Voter 1 addr1}]}".to_string(),
            },
            // AddStaging: was Staging
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::AddStaging,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Voter 2 addr2} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // AddStaging: was Nonvoter
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::AddStaging,
                server_id: 3,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2x} {Voter 3 addr3}]}".to_string(),
            },
            // AddNonvoter: was missing
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::AddNonvoter,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Nonvoter 2 addr2}]}".to_string(),
            },
            // AddNonvoter: was Voter.
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::AddNonvoter,
                server_id: 1,
                next: "{[{Voter 1 addr1}]}".to_string(),
            },
            // AddNonvoter: was Staging
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::AddNonvoter,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // AddNonvoter: was Nonvoter.
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::AddNonvoter,
                server_id: 3,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2x} {Nonvoter 3 addr3}]}".to_string(),
            },
            // DemoteVoter: was missing
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::DemoteVoter,
                server_id: 2,
                next: "{[{Voter 1 addr1x}]}".to_string(),
            },
            // DemoteVoter: was Voter
            NextConfigurationTests {
                current: voter_pair(),
                command: ConfigurationChangeCommand::DemoteVoter,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Nonvoter 2 addr2x}]}".to_string(),
            },
            // DemoteVoter: was Staging
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::DemoteVoter,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Nonvoter 2 addr2x} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // DemoteVoter: was Nonvoter
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::DemoteVoter,
                server_id: 3,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2x} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // RemoveServer: was missing
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::RemoveServer,
                server_id: 2,
                next: "{[{Voter 1 addr1x}]}".to_string(),
            },
            // RemoveServer: was Voter
            NextConfigurationTests {
                current: voter_pair(),
                command: ConfigurationChangeCommand::RemoveServer,
                server_id: 2,
                next: "{[{Voter 1 addr1x}]}".to_string(),
            },
            // RemoveServer: was Staging
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::RemoveServer,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // RemoveServer: was Nonvoter
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::RemoveServer,
                server_id: 3,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2x}]}".to_string(),
            },
            // Promote: was missing
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::Promote,
                server_id: 2,
                next: "{[{Voter 1 addr1x}]}".to_string(),
            },
            // Promote: was Voter
            NextConfigurationTests {
                current: single_server(),
                command: ConfigurationChangeCommand::Promote,
                server_id: 1,
                next: "{[{Voter 1 addr1x}]}".to_string(),
            },
            // Promote: was Staging
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::Promote,
                server_id: 2,
                next: "{[{Voter 1 addr1x} {Voter 2 addr2x} {Nonvoter 3 addr3x}]}".to_string(),
            },
            // Promote: was Nonvoter
            NextConfigurationTests {
                current: one_of_each(),
                command: ConfigurationChangeCommand::Promote,
                server_id: 3,
                next: "{[{Voter 1 addr1x} {Staging 2 addr2x} {Nonvoter 3 addr3x}]}".to_string(),
            },
        ]
    }

    #[test]
    fn test_config_builder() {
        #[cfg(not(feature = "default"))]
        let (_, rx) = unbounded::<bool>();
        #[cfg(feature = "default")]
        let (_, rx) = unbounded_channel::<bool>();
        let config = ConfigBuilder::default()
            .set_local_id(123)
            .set_protocol_version(ProtocolVersionMax)
            .set_heartbeat_timeout(Duration::from_millis(1000))
            .set_election_timeout(Duration::from_millis(1000))
            .set_commit_timeout(Duration::from_millis(50))
            .set_max_append_entries(128)
            .set_batch_apply_ch(true)
            .set_shut_down_on_remove(true)
            .set_trailing_logs(10240)
            .set_snapshot_interval(Duration::from_secs(120))
            .set_snapshot_threshold(8192)
            .set_leader_lease_timeout(Duration::from_millis(500))
            .set_no_snapshot_restore_on_start(false)
            .set_skip_startup(false)
            .finalize(rx)
            .unwrap();
        assert!(!config.skip_startup);
    }

    #[test]
    fn test_reloadable_config() {
        #[cfg(not(feature = "default"))]
        let (_, rx) = unbounded::<bool>();
        #[cfg(feature = "default")]
        let (_, rx) = unbounded_channel::<bool>();

        let config = ConfigBuilder::default()
            .set_local_id(123)
            .finalize(rx)
            .unwrap();
        let rc = ReloadableConfig {
            trailing_logs: DEFAULT_TRAILING_LOGS,
            snapshot_interval: Duration::from_secs(60),
            snapshot_threshold: DEFAULT_SNAPSHOT_THRESHOLD,
        };

        assert_eq!(
            rc.apply(config.clone()).snapshot_interval,
            Duration::from_secs(60)
        );

        let rc = ReloadableConfig::from_config(config);
        assert_eq!(rc.snapshot_interval, Duration::from_secs(120));

        #[cfg(not(feature = "default"))]
        let (_, rx) = unbounded::<bool>();
        #[cfg(feature = "default")]
        let (_, rx) = unbounded_channel::<bool>();

        let rc = ReloadableConfig::default();
        let config = ConfigBuilder::default()
            .set_local_id(123)
            .finalize(rx)
            .unwrap();
        assert!(rc == config.clone());
        let rc = ReloadableConfig::default();
        assert_eq!(rc.apply(config.clone()).trailing_logs, config.trailing_logs);
    }

    #[test]
    fn test_configuration_has_vote() {
        assert!(
            !has_vote(sample_configuration(), 0),
            "server id 0 should not have vote"
        );
        assert!(
            has_vote(sample_configuration(), 1),
            "server id 1 should have vote"
        );
        assert!(
            !has_vote(sample_configuration(), 2),
            "server id 2 should not have vote"
        );
        assert!(
            !has_vote(sample_configuration(), 12345),
            "server other id should not have vote"
        );
    }

    #[test]
    fn test_configuration_next_configuration_table() {
        for (idx, tt) in next_configuration_tests().into_iter().enumerate() {
            let req = ConfigurationChangeRequest {
                command: tt.command,
                server_id: tt.server_id,
                server_address: format!("addr{}", tt.server_id),
                prev_index: 0,
            };
            let next = next_configuration(tt.clone().current, 1, req);
            match next {
                Ok(next) => {
                    assert_eq!(
                        format!("{}", next),
                        tt.next,
                        "nextConfiguration {} returned {}, expected {}",
                        idx,
                        next,
                        tt.next
                    );
                }
                Err(err) => {
                    eprintln!(
                        "nextConfiguration {} should have succeeded, got {}",
                        idx, err
                    );
                    continue;
                }
            }
        }
    }

    #[test]
    fn test_configuration_next_configuration_prev_index() {
        // stable prev_index
        let req = ConfigurationChangeRequest {
            command: ConfigurationChangeCommand::AddStaging,
            server_id: 1,
            server_address: "addr1".to_string(),
            prev_index: 1,
        };
        match next_configuration(single_server(), 2, req) {
            Ok(_) => {
                panic!(
                    "next_configuration should have failed due to intervening configuration change"
                );
            }
            Err(e) => {
                let s = format!("{}", e);
                assert!(
                    s.contains("changed"),
                    "next_configuration should have failed due to intervening configuration change"
                );
            }
        }

        // current prev_index
        let req = ConfigurationChangeRequest {
            command: ConfigurationChangeCommand::AddStaging,
            server_id: 2,
            server_address: "addr2".to_string(),
            prev_index: 2,
        };
        match next_configuration(single_server(), 2, req) {
            Ok(_) => {}
            Err(e) => panic!("nextConfiguration should have succeeded, got {}", e),
        }

        // zero prev_index
        let req = ConfigurationChangeRequest {
            command: ConfigurationChangeCommand::AddStaging,
            server_id: 3,
            server_address: "addr3".to_string(),
            prev_index: 0,
        };
        match next_configuration(single_server(), 2, req) {
            Ok(_) => {}
            Err(e) => panic!("nextConfiguration should have succeeded, got {}", e),
        }
    }

    #[test]
    fn test_configuration_next_configuration_check_configuration() {
        let req = ConfigurationChangeRequest {
            command: AddNonvoter,
            server_id: 1,
            server_address: "addr1".to_string(),
            prev_index: 0,
        };

        match next_configuration(Configuration::new(), 1, req) {
            Ok(_) => {
                panic!("next_configuration should have failed for not having a voter");
            }
            Err(e) => {
                let s = format!("{}", e);
                assert!(
                    s.contains("at least one voter"),
                    "next_configuration should have failed for not having a voter"
                );
            }
        }
    }

    #[test]
    fn test_configuration_encode_decode_configuration() {
        let cfg = decode_configuration(encode_configuration(sample_configuration()));
        assert_eq!(cfg, sample_configuration());
    }
}
