use std::string::FromUtf8Error;
use thiserror::Error;

/// `Errors` represents custom errors in this crate.
#[derive(Error, Debug, Eq, PartialEq)]
pub enum Errors {
    /// `LogNotFound` when the specific log entry is not found
    #[error("log not found")]
    LogNotFound,

    /// `NotFound` when some else is not found, which is different from `LogNotFound`
    #[error("not found")]
    NotFound,

    /// `FromUtf8Error` when cannot convert UTF8 bytes to `String`
    #[error("cannot convert UTF8 to String")]
    FromUtf8Error(#[from] FromUtf8Error),

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
}
