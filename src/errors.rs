use std::string::FromUtf8Error;
use std::fmt::Formatter;

/// `Error` represents custom errors in this crate.
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum Error {
    /// `LogNotFound` when the specific log entry is not found
    LogNotFound,

    /// `PipelineReplicationNotSupported` can be returned by the transport to
    /// signal that pipeline replication is not supported in general, and that
    /// no error message should be produced.
    PipeLineReplicationNotSupported,

    /// `UnableToStoreLogs` unable to store logs within log store
    UnableToStoreLogs(String),

    /// `NotFound` when some else is not found, which is different from `LogNotFound`
    NotFound,

    /// `FromUtf8Error` when cannot convert UTF8 bytes to `String`
    FromUtf8Error(FromUtf8Error),

    /// local_id cannot be empty
    EmptyLocalID,

    /// heartbeat_timeout is too low
    ShortHeartbeatTimeout,

    /// election_timeout is too low
    ShortElectionTimeout,

    /// commit_timeout is too low
    ShortCommitTimeout,

    /// max_append_entries is too large
    LargeMaxAppendEntries,

    /// snapshot_interval is too short
    ShortSnapshotInterval,

    /// leader_lease_timeout is too short
    ShortLeaderLeaseTimeout,

    /// leader_lease_timeout cannot be larger than heartbeat timeout
    LeaderLeaseTimeoutLargerThanHeartbeatTimeout,

    /// election_timeout must be equal or greater than heartbeat timeout
    ElectionTimeoutSmallerThanHeartbeatTimeout,

    /// `ErrLeader` is returned when an operation can't be completed on a
    /// leader node.
    Leader,

    /// `ErrNotLeader` is returned when an operation can't be completed on a
    /// follower or candidate node.
    NotLeader,

    /// `ErrLeadershipLost` is returned when a leader fails to commit a log entry
    /// because it's been deposed in the process.
    LeadershipLost,

    /// `AbortedByRestore` is returned when a leader fails to commit a log
    /// entry because it's been superseded by a user snapshot restore.
    AbortedByRestore,

    /// `RaftShutdown` is returned when operations are requested against an
    /// inactive Raft.
    RaftShutdown,

    /// `EnqueueTimeout` is returned when a command fails due to a timeout.
    EnqueueTimeout,

    /// `NothingNewToSnapshot` is returned when trying to create a snapshot
    /// but there's nothing new committed to the `FSM` since we started.
    NothingNewToSnapshot,

    // /// `UnsupportedProtocol` is returned when an operation is attempted
    // /// that's not supported by the current protocol version.
    // #[error("operation not supported with current protocol version")]
    // UnsupportedProtocol,
    /// `CantBootstrap` is returned when attempt is made to bootstrap a
    /// cluster that already has state present.
    CantBootstrap,

    /// `LeadershipTransferInProgress` is returned when the leader is rejecting
    /// client requests because it is attempting to transfer leadership.
    LeadershipTransferInProgress,

    /// `DuplicateServerID` found duplicate server ID in configuration
    DuplicateServerID(u64),

    /// `DuplicateServerAddress` found duplicate server address in configuration
    DuplicateServerAddress(String),

    /// `NonVoter` need at least one voter in configuration
    NonVoter,

    /// `ConfigurationChanged` configuration changed since prev_index (latest is current_index)
    ConfigurationChanged { current_index: u64, prev_index: u64 },

    /// `FSMRestore` error
    FSMRestore(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::LogNotFound => write!(f, "log not found"),
            Error::PipeLineReplicationNotSupported => write!(f, "pipeline replication not supported"),
            Error::UnableToStoreLogs(s) => write!(f, "unable to store logs within log store, err: {}", *s),
            Error::NotFound => write!(f, "not found"),
            Error::EmptyLocalID => write!(f, "local_id cannot be empty"),
            Error::ShortHeartbeatTimeout => write!(f, "heartbeat_timeout is too low"),
            Error::ShortElectionTimeout => write!(f, "election_timeout is too low"),
            Error::ShortCommitTimeout => write!(f, "commit_timeout is too low"),
            Error::LargeMaxAppendEntries => write!(f, " max_append_entries is too large"),
            Error::ShortSnapshotInterval => write!(f, "snapshot_interval is too short"),
            Error::ShortLeaderLeaseTimeout => write!(f, "leader_lease_timeout is too short"),
            Error::LeaderLeaseTimeoutLargerThanHeartbeatTimeout => write!(f, "leader_lease_timeout cannot be larger than heartbeat timeout"),
            Error::ElectionTimeoutSmallerThanHeartbeatTimeout => write!(f, "election_timeout must be equal or greater than heartbeat timeout"),
            Error::FromUtf8Error(e) => write!(f, "cannot convert UTF8 to String. From: {}", *e),
            Error::Leader => write!(f, "node is the leader"),
            Error::NotLeader => write!(f, "node is not the leader"),
            Error::LeadershipLost => write!(f, "leadership lost while committing log"),
            Error::AbortedByRestore => write!(f, "snapshot restored while committing log"),
            Error::RaftShutdown => write!(f, "raft is already shutdown"),
            Error::EnqueueTimeout => write!(f, "timed out enqueuing operation"),
            Error::NothingNewToSnapshot => write!(f, "othing new to snapshot"),
            Error::CantBootstrap => write!(f, "bootstrap only works on new clusters"),
            Error::LeadershipTransferInProgress => write!(f, "leadership transfer in progress"),
            Error::DuplicateServerID(id) => write!(f, "found duplicate server ID in configuration: {}", *id),
            Error::DuplicateServerAddress(addr) => write!(f, "found duplicate server address in configuration: {}", *addr),
            Error::NonVoter => write!(f, "need at least one voter in configuration"),
            Error::ConfigurationChanged { current_index, prev_index } => write!(f, "configuration changed since {} (latest is {})", *prev_index, *current_index),
            #[cfg(feature = "default")]
            Error::FSMRestore(e) => write!(f, "FSM restore IO error. From: {}", *e),
        }
    }
}

impl From<FromUtf8Error> for Error {
    fn from(e: FromUtf8Error) -> Self {
        Self::FromUtf8Error(e)
    }
}
