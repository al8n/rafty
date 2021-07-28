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

/// State captures the state of a Raft node: Follower, Candidate, Leader, or Shutdown
#[derive(Display, FromStr, Debug, Copy, Clone, Eq, PartialEq)]
#[display(style = "CamelCase")]
pub enum StateType {
    /// Follower is the initial state of a Raft node.
    Follower,

    /// Leader is one of the valid states of a Raft node
    Leader,

    /// Candidate is one of the valid states of a Raft node
    Candidate,

    /// Shutdown is the terminal state of a Raft node
    Shutdown,
}

pub struct State {
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
}

mod tests {
    use super::*;

    #[test]
    fn test_state() {
        let s = StateType::Follower;

        assert_eq!(format!("{}", s), "Follower");
        assert_eq!(StateType::Candidate.to_string(), "Candidate");
        assert_eq!(StateType::Shutdown.to_string(), "Shutdown");
        assert_eq!(StateType::Leader.to_string(), "Leader");
    }
}
