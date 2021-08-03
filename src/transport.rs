use crate::config::ServerAddress;

/// `Transport` provides an interface for network transports
/// to allow [`Raft`](crate::raft::Raft) to communicate with other nodes.
#[cfg(not(feature = "default"))]
pub(crate) trait Transport {
    /// `Consumer` returns a channel that can be used to
    /// consume and respond to RPC requests.
    fn consumer(&self) -> Receiver<RPC>;

    /// local_addr is used to return our local address to distinguish from our peers.
    fn local_addr(&self) -> ServerAddress;


}
