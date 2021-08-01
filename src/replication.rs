use std::time::Duration;
use crate::config::Server;
use crate::commitment::Commitment;

cfg_default!(
    use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, Sender, Receiver};
);

cfg_not_default!(
    use crossbeam::channel::{Sender, Receiver};
);

static MAX_FAILURE_SCALE: u64 = 12;
static FAILURE_WAIT: Duration = Duration::from_millis(10);

/// `FollowReplication` is in charge of sending snapshots and log entries from
/// this leader during this particular term to a remote follower.
struct FollowReplication<'a> {
    /// `current_term` is the term of this leader, to be included in AppendEntries
    /// requests.
    current_term: u64,

    /// `next_index` is the index of the next log entry to send to the follower,
    /// which may fall past the end of the log.
    next_index: u64,

    /// `peer` contains the network address and ID of the remote follower.
    peer: Server,

    /// `commitment` tracks the entries acknowledged by followers so that the
    /// leader's commit index can advance. It is updated on successful
    /// AppendEntries responses.
    commitment: &'a Commitment,

    // stopCh is notified/closed when this leader steps down or the follower is
    // removed from the cluster. In the follower removed case, it carries a log
    // index; replication should be attempted with a best effort up through that
    // index, before exiting.
    #[cfg(feature = "default")]
    stop_tx: Sender<u64>,
    #[cfg(feature = "default")]
    stop_rx: Receiver<u64>,

    #[cfg(not(feature = "default"))]
    stop_tx: Sender<u64>,
    #[cfg(not(feature = "default"))]
    stop_rx: Receiver<u64>,

    // triggerCh is notified every time new entries are appended to the log.
    #[cfg(feature = "default")]
    trigger_tx: UnboundedSender<u64>,
    #[cfg(feature = "default")]
    trigger_rx: UnboundedReceiver<u64>,

    #[cfg(not(feature = "default"))]
    trigger_tx: Sender<()>,
    #[cfg(not(feature = "default"))]
    trigger_rx: Receiver<()>,


}