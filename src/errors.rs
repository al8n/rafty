use std::string::FromUtf8Error;
use thiserror::Error;

/// `Errors` represents custom errors in this crate.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum Errors {
    /// `LogNotFound` when the specific log entry is not found
    #[error("log not found")]
    LogNotFound,

    #[error("unable to store logs within log store, err: {0}")]
    UnableToStoreLogs(String),

    /// `NotFound` when some else is not found, which is different from `LogNotFound`
    #[error("not found")]
    NotFound,

    /// `FromUtf8Error` when cannot convert UTF8 bytes to `String`
    #[error("cannot convert UTF8 to String")]
    FromUtf8Error(#[from] FromUtf8Error),

    // config related errors
    #[error("local_id cannot be empty")]
    EmptyLocalID,

    #[error("heartbeat_timeout is too low")]
    ShortHeartbeatTimeout,

    #[error("election_timeout is too low")]
    ShortElectionTimeout,

    #[error("commit_timeout is too low")]
    ShortCommitTimeout,

    #[error("max_append_entries is too large")]
    LargeMaxAppendEntries,

    #[error("snapshot_interval is too short")]
    ShortSnapshotInterval,

    #[error("leader_lease_timeout is too short")]
    ShortLeaderLeaseTimeout,

    #[error("leader_lease_timeout cannot be larger than heartbeat timeout")]
    LeaderLeaseTimeoutLargerThanHeartbeatTimeout,

    #[error("election_timeout must be equal or greater than heartbeat timeout")]
    ElectionTimeoutSmallerThanHeartbeatTimeout,

    // API related errors
    /// `ErrLeader` is returned when an operation can't be completed on a
    /// leader node.
    #[error("node is the leader")]
    Leader,

    /// `ErrNotLeader` is returned when an operation can't be completed on a
    /// follower or candidate node.
    #[error("node is not the leader")]
    NotLeader,

    /// `ErrLeadershipLost` is returned when a leader fails to commit a log entry
    /// because it's been deposed in the process.
    #[error("leadership lost while committing log")]
    LeadershipLost,

    /// `AbortedByRestore` is returned when a leader fails to commit a log
    /// entry because it's been superseded by a user snapshot restore.
    #[error("snapshot restored while committing log")]
    AbortedByRestore,

    /// `RaftShutdown` is returned when operations are requested against an
    /// inactive Raft.
    #[error("raft is already shutdown")]
    RaftShutdown,

    /// `EnqueueTimeout` is returned when a command fails due to a timeout.
    #[error("timed out enqueuing operation")]
    EnqueueTimeout,

    /// `NothingNewToSnapshot` is returned when trying to create a snapshot
    /// but there's nothing new committed to the `FSM` since we started.
    #[error("nothing new to snapshot")]
    NothingNewToSnapshot,

    // /// `UnsupportedProtocol` is returned when an operation is attempted
    // /// that's not supported by the current protocol version.
    // #[error("operation not supported with current protocol version")]
    // UnsupportedProtocol,
    /// `CantBootstrap` is returned when attempt is made to bootstrap a
    /// cluster that already has state present.
    #[error("bootstrap only works on new clusters")]
    CantBootstrap,

    /// `LeadershipTransferInProgress` is returned when the leader is rejecting
    /// client requests because it is attempting to transfer leadership.
    #[error("leadership transfer in progress")]
    LeadershipTransferInProgress,

    #[error("found duplicate server ID in configuration: `{0}`")]
    DuplicateServerID(u64),

    #[error("found duplicate server address in configuration: `{0}`")]
    DuplicateServerAddress(String),

    #[error("need at least one voter in configuration")]
    NonVoter,

    #[error("configuration changed since {prev_index} (latest is {current_index})")]
    ConfigurationChanged { current_index: u64, prev_index: u64 },
}
