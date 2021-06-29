use crate::errors::Errors;
use crossbeam::channel::Receiver;
use parse_display::{Display, FromStr};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

/// `Config` provides any necessary configuration for the `Raft` server.
#[derive(Debug, Clone)]
pub struct Config {
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
    /// `new` returns a `Config` with usable defaults.
    pub fn new(local_id: ServerID, notify_ch: Receiver<bool>) -> Self {
        Self {
            heartbeat_timeout: Duration::from_millis(1000),
            election_timeout: Duration::from_millis(1000),
            commit_timeout: Duration::from_millis(50),
            max_append_entries: 64,
            batch_apply_ch: false,
            shut_down_on_remove: true,
            trailing_logs: 10240,
            snapshot_interval: Duration::from_secs(120),
            snapshot_threshold: 8192,
            leader_lease_timeout: Duration::from_millis(500),
            local_id,
            notify_ch,
            no_snapshot_restore_on_start: false,
            skip_startup: false,
        }
    }

    /// `validate_config` is used to validate a sane configuration
    pub fn validate_config(&self) -> Result<(), Errors> {
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

        Ok(())
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
    suffrage: ServerSuffrage,

    /// `id` is a unique number ([Sonyflake distributed unique ID generator](https://github.com/sony/sonyflake) ) identifying this server for all time.
    ///
    /// Thanks for Arne Bahlo, the author of [sonyflake-rs](https://github.com/bahlo/sonyflake-rs).
    id: ServerID,

    /// `address` is its network address that a transport can contact.
    address: ServerAddress,
}

/// `Configuration` tracks which servers are in the cluster, and whether they have
/// votes. This should include the local server, if it's a member of the cluster.
/// The servers are listed no particular order, but each should only appear once.
/// These entries are appended to the log during membership changes.
pub struct Configuration {
    servers: Vec<Server>,
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
