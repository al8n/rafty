use crate::state::State;
use crate::config::{ProtocolVersion, Configuration};
use crate::future::LogFuture;
use crate::fsm::FSM;


cfg_not_default!(
    use crossbeam::atomic::AtomicCell;
);

cfg_default!(
    use std::sync::atomic::AtomicPtr;
    use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
);


/// Raft implements a Raft node.
pub struct Raft<FSM, LF>
where FSM: FSM {
    state: State,

    /// `protocol_version` is used to inter-operate with Raft servers running
    /// different versions of the library. See comments in config.go for more
    /// details.
    protocol_version: ProtocolVersion,

    /// `apply_tx_ch` is used to async send logs to the main thread to
    /// be committed and applied to the FSM.
    #[cfg(feature = "default")]
    apply_tx_ch: UnboundedSender<LogFuture<LF>>,

    /// `apply_rx_ch` is used to async receive logs to the main thread to
    /// be committed and applied to the FSM.
    #[cfg(feature = "default")]
    apply_rx_ch: UnboundedReceiver<LogFuture<LF>>,

    /// `conf` stores the current configuration to use. This is the most recent one
    /// provided. All reads of config values should use the config() helper method
    /// to read this safely.
    #[cfg(feature = "default")]
    conf: AtomicPtr<Configuration>,

    /// `apply_tx_ch` is used to async send logs to the main thread to
    /// be committed and applied to the FSM.
    #[cfg(not(feature = "default"))]
    apply_tx_ch: Sender<LogFuture<LF>>,

    /// `apply_rx_ch` is used to async receive logs to the main thread to
    /// be committed and applied to the FSM.
    #[cfg(not(feature = "default"))]
    apply_rx_ch: Receiver<LogFuture<LF>>,

    #[cfg(not(feature = "default"))]
    /// `conf` stores the current configuration to use. This is the most recent one
    /// provided. All reads of config values should use the config() helper method
    /// to read this safely.
    conf: AtomicCell<Configuration>,

    /// `fsm` is the client state machine to apply commands to
    fsm: FSM,


}

impl<FSM, LF> Raft<FSM, LF> {
    pub fn wait_shutdown(&mut self) {
        self.state.wait_shutdown()
    }
}