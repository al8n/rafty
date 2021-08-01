use crate::config::Configuration;
use crate::errors::Error;
use crossbeam::{
    channel::{Sender, Receiver, bounded},
};


/// Future is used to represent an action that may occur in the future.
pub trait Future {
    /// Error blocks until the future arrives and then returns the error status
    /// of the future. This may be called any number of times - all calls will
    /// return the same value, however is not OK to call this method twice
    /// concurrently on the same Future instance.
    /// Error will only return generic errors related to raft, such
    /// as ErrLeadershipLost, or ErrRaftShutdown. Some operations, such as
    /// ApplyLog, may also return errors from other methods.
    fn error(&self) -> Error;
}

/// IndexFuture is used for future actions that can result in a raft log entry
/// being created.
pub trait IndexFuture: Future {
    /// Index holds the index of the newly applied log entry.
    /// This must not be called until after the Error method has returned.
    fn index(&self) -> u64;
}

/// ApplyFuture is used for Apply and can return the FSM response.
pub trait ApplyFuture<T>: IndexFuture {

    /// Response returns the FSM response as returned by the FSM.Apply method. This
    /// must not be called until after the Error method has returned.
    /// Note that if FSM.Apply returns an error, it will be returned by Response,
    /// and not by the Error method, so it is always important to check Response
    /// for errors from the FSM.
    fn response(&self) -> T;
}

/// ConfigurationFuture is used for GetConfiguration and can return the
/// latest configuration in use by Raft.
pub trait ConfigurationFuture: IndexFuture {

    /// Configuration contains the latest configuration. This must
    /// not be called until after the Error method has returned.
    fn configuration(&self) -> Configuration;
}

/// SnapshotFuture is used for waiting on a user-triggered snapshot to complete.
pub trait SnapshotFuture: Future {
    // TODO: implement snapshot future
    // fn open(&self) -> ;
}

/// LeadershipTransferFuture is used for waiting on a user-triggered leadership
/// transfer to complete.
pub trait LeadershipTransferFuture: Future {}

/// ErrorFuture is used to return a static error.
pub struct ErrorFuture {
    error: Error,
}

impl Future for ErrorFuture {
    fn error(&self) -> Error {
        self.error.clone()
    }
}

impl IndexFuture for ErrorFuture {
    fn index(&self) -> u64 {
        0
    }
}

impl ApplyFuture<()> for ErrorFuture {
    fn response(&self) -> () {
        ()
    }
}

/// DeferError can be embedded to allow a future
/// to provide an error in the future
pub struct DeferError {
    err: Option<Error>,
    err_tx_ch: Option<Sender<Error>>,
    err_rx_ch: Option<Receiver<Error>>,
    responded: bool,
    shutdown_tx_ch: Option<Sender<()>>,
    shutdown_rx_ch: Option<Receiver<()>>,
}

impl Default for DeferError {
    fn default() -> Self {
        Self {
            err: None,
            err_tx_ch: None,
            err_rx_ch: None,
            responded: false,
            shutdown_tx_ch: None,
            shutdown_rx_ch: None
        }
    }
}

impl DeferError {
    pub fn init() -> Self {
        let mut de = Self::default();
        let (tx, rx) = bounded::<Error>(1);
        de.err_rx_ch = Some(rx);
        de.err_tx_ch = Some(tx);
        de
    }

    pub fn set_error(&mut self, err: Error) {
        self.err = Some(err);
    }
}

impl Future for DeferError {
    fn error(&self) -> Error {
         todo!()
    }
}