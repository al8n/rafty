use crate::errors::Errors;
use crossbeam::channel::Receiver;
use parse_display::{Display, FromStr};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

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
    notify_ch: Receiver<bool>,

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
    #[inline]
    pub fn finalize(self, notify_ch: Receiver<bool>) -> Result<Config, Errors> {
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
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr)]
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
// pub type ServerAddress = String;
#[derive(Display, FromStr, Copy, Clone, Eq, PartialEq, Debug)]
#[display(style = "CamelCase")]
pub enum ServerAddress {
    /// `IPv4` stands for an IPv4 address
    #[display("IPv4: {0}")]
    IPv4(Ipv4Addr),

    /// `IPv6` stands for an IPv6 address
    #[display("IPv6: {0}")]
    IPv6(Ipv6Addr),
}

/// `Server` tracks the information about a single server in a configuration.
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

/// `Configuration` tracks which servers are in the cluster, and whether they have
/// votes. This should include the local server, if it's a member of the cluster.
/// The servers are listed no particular order, but each should only appear once.
/// These entries are appended to the log during membership changes.
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::ProtocolVersion::ProtocolVersionMax;
    use crossbeam::channel::unbounded;

    #[test]
    fn test_config_builder() {
        let (_, rx) = unbounded::<bool>();
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
        let (_, rx) = unbounded::<bool>();
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

        let (_, rx) = unbounded::<bool>();
        let rc = ReloadableConfig::default();
        let config = ConfigBuilder::default()
            .set_local_id(123)
            .finalize(rx)
            .unwrap();
        assert!(rc == config.clone());
        let rc = ReloadableConfig::default();
        assert_eq!(rc.apply(config.clone()).trailing_logs, config.trailing_logs);
    }
}
