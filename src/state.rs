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
use parse_display::{Display, FromStr};
use parking_lot::Mutex;
use std::sync::Arc;
use std::cmp::max;
use crate::wg::WaitGroup;

cfg_default!(
    use tokio::spawn;
);

cfg_not_default!(
    use std::thread::spawn;
);

/// State captures the state of a Raft node: Follower, Candidate, Leader, or Shutdown
#[derive(Display, FromStr, Debug, Copy, Clone, Eq, PartialEq)]
#[display(style = "CamelCase")]
pub(crate) enum StateType {
    /// Follower is the initial state of a Raft node.
    Follower,

    /// Leader is one of the valid states of a Raft node
    Leader,

    /// Candidate is one of the valid states of a Raft node
    Candidate,

    /// Shutdown is the terminal state of a Raft node
    Shutdown,
}

/// `State` is used to maintain various state variables
/// and provides an trait to set/get the variables in a
/// thread safe manner.
pub(crate) struct State {
    /// latest term server has seen
    current_term: u64,

    /// index of highest log entry known to be committed (initialized to 0, increases monotonically)
    commit_index: u64,

    /// index of highest log entry applied to state machine (initialized to 0, increases monotonically)
    last_applied: u64,

    /// cache the latest snapshot index
    last_log_index: u64,

    /// cache the latest snapshot term
    last_log_term: u64,

    /// cache the latest snapshot index
    last_snapshot_index: u64,

    /// cache the latest snapshot term
    last_snapshot_term: u64,

    /// the current state type
    typ: StateType,

    /// Tracks running threads
    group: WaitGroup,
}

impl State {
    pub fn new(current_term: u64, commit_index: u64, last_applied: u64, last_log_index: u64, last_log_term: u64, last_snapshot_index: u64, last_snapshot_term: u64, typ: StateType) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            current_term,
            commit_index,
            last_applied,
            last_log_index,
            last_log_term,
            last_snapshot_index,
            last_snapshot_term,
            typ,
            group: WaitGroup::new(),
        }))
    }

    pub fn get_state(&self) -> StateType {
        self.typ
    }

    pub fn set_state(&mut self, typ: StateType) {
        self.typ = typ
    }

    pub fn get_current_term(&self) -> u64 {
        self.current_term
    }

    pub fn set_current_term(&mut self, term: u64) {
        self.current_term = term
    }

    pub fn get_last_log(&self) -> (u64, u64) {
        (self.last_log_index, self.last_log_term)
    }

    pub fn set_last_log(&mut self, index: u64, term: u64) {
        self.last_log_index = index;
        self.last_log_term = term;
    }

    pub fn get_last_snapshot(&self) -> (u64, u64) {
        (self.last_snapshot_index, self.last_snapshot_term)
    }

    pub fn set_last_snapshot(&mut self, index: u64, term: u64) {
        self.last_snapshot_index = index;
        self.last_snapshot_term = term;
    }

    pub fn get_commit_index(&self) -> u64 {
        self.commit_index
    }

    pub fn set_commit_index(&mut self, index: u64) {
        self.commit_index = index;
    }

    pub fn get_last_applied(&self) -> u64 {
        self.last_applied
    }

    pub fn set_last_applied(&mut self, index: u64) {
        self.last_applied = index;
    }

    pub fn get_last_index(&self) -> u64 {
        max(self.last_log_index, self.last_snapshot_index)
    }

    pub fn get_last_entry(&self) -> (u64, u64) {
        if self.last_log_index >= self.last_log_term {
            return (self.last_log_index, self.last_log_term);
        }
        (self.last_snapshot_index, self.last_snapshot_term)
    }

    #[cfg(feature = "default")]
    /// Start a thread and properly handle the race between a routine
    /// starting and incrementing, and exiting and decrementing.
    pub async fn spawn<F>(&self, f: F)
    where F: FnOnce() + Send + 'static {
        let wg = self.group.add(1);
        spawn(async move {
            f();
            wg.done();
        });
    }

    #[cfg(feature = "default")]
    /// wait for all threads in waitgroup finish
    pub async fn wait_shutdown(&mut self) {
        self.group.wait().await;
    }

    #[cfg(not(feature = "default"))]
    /// Start a thread and properly handle the race between a routine
    /// starting and incrementing, and exiting and decrementing.
    pub fn spawn<F>(&self, f: F)
        where F: FnOnce() {
        let wg = self.clone();
        spawn(move || {
            f();
            drop(wg);
        });
    }

    #[cfg(not(feature = "default"))]
    /// wait for all threads in waitgroup finish
    pub fn wait_shutdown(&mut self) {
        self.group.wait();
    }
}

mod tests {
    use crate::state::StateType;

    #[test]
    fn test_state() {
        let s = StateType::Follower;

        assert_eq!(format!("{}", s), "Follower");
        assert_eq!(StateType::Candidate.to_string(), "Candidate");
        assert_eq!(StateType::Shutdown.to_string(), "Shutdown");
        assert_eq!(StateType::Leader.to_string(), "Leader");
    }
}
