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
use crate::config::{ProtocolVersion, SnapshotVersion};
use crate::log::Log;
use std::sync::Arc;

/// `RPCHeader` is a common sub-structure used to pass along protocol version and
/// other information about the cluster. For older Raft implementations before
/// versioning was added this will default to a zero-valued structure when read
/// by newer Raft versions.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RPCHeader {
    /// `version` is the version of the protocol the sender is
    /// speaking.
    version: ProtocolVersion,
}

impl RPCHeader {
    pub fn new(version: ProtocolVersion) -> Self {
        Self { version }
    }

    /// Get the version of the protocol the sender is speaking.
    pub fn get_version(&self) -> ProtocolVersion {
        self.version
    }
}

/// `WithRPCHeader` is an trait that exposes the RPC header.
pub trait WithRPCHeader {
    fn get_rpc_header(&self) -> RPCHeader;
}

macro_rules! with_rpc_header {
    ($($t:ty),*) => {
        $(impl WithRPCHeader for $t {
            fn get_rpc_header(&self) -> RPCHeader {
                self.header
            }
        })*
    }
}

/// `AppendEntriesRequest` is the command used to append entries to the
/// replicated log.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AppendEntriesRequest {
    header: RPCHeader,

    // Provide the current term and leader
    term: u64,
    leader_id: u64,

    // Provide the previous entries for integrity checking
    prev_log_entry: u64,
    prev_log_term: u64,

    // New entries to commit
    entries: Vec<Log>,

    // Commit index on the leader
    leader_commit_index: u64,
}

/// `AppendEntriesResponse` is the response returned from an
/// `AppendEntriesRequest`.
#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct AppendEntriesResponse {
    header: RPCHeader,

    /// Newer term if leader is out of date
    term: u64,

    /// `last_log` is a hint to help accelerate rebuilding slow nodes
    last_log: u64,

    /// `success`: We may not succeed if we have a conflicting entry
    success: bool,

    /// There are scenarios where this request didn't succeed
    /// but there's no need to wait/back-off the next attempt.
    no_retry_backoff: bool,
}

/// `RequestVoteRequest` is the command used by a candidate to ask a Raft peer
/// for a vote in an election.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct RequestVoteRequest {
    header: RPCHeader,

    // Provide the term and our id
    term: u64,
    candidate_id: u64,

    // Used to ensure safety
    last_log_index: u64,
    last_log_term: u64,

    /// Used to indicate to peers if this vote was triggered by a leadership
    /// transfer. It is required for leadership transfer to work, because servers
    /// wouldn't vote otherwise if they are aware of an existing leader.
    leader_ship_transfer: bool,
}

/// `RequestVoteResponse` is the response returned from a `RequestVoteRequest`.
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct RequestVoteResponse {
    header: RPCHeader,

    // Newer term if leader is out of date.
    term: u64,

    // Is the vote granted
    granted: bool,
}

/// `InstallSnapshotRequest` is the command sent to a Raft peer to bootstrap its
/// log (and state machine) from a snapshot on another peer.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct InstallSnapshotRequest {
    header: RPCHeader,
    version: SnapshotVersion,

    term: u64,
    leader: u64,

    last_log_index: u64,
    last_log_term: u64,

    /// cluster membership
    configuration: Vec<u8>,

    /// `Log` index where 'Configuration' entry was originally written.
    configuration_index: u64,

    /// Size of the snapshot
    size: u64,
}

/// `InstallSnapshotResponse` is the response returned from an
/// `InstallSnapshotRequest`.
#[derive(Copy, Clone, Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct InstallSnapshotResponse {
    header: RPCHeader,

    term: u64,
    success: bool,
}

/// `TimeoutNowRequest` is the command used by a leader to signal another server to
/// start an election.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct TimeoutNowRequest {
    header: RPCHeader,
}

/// `TimeoutNowResponse` is the response to `TimeoutNowRequest`.
#[derive(Copy, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub struct TimeoutNowResponse {
    header: RPCHeader,
}

with_rpc_header!(
    AppendEntriesRequest,
    AppendEntriesResponse,
    RequestVoteRequest,
    RequestVoteResponse,
    InstallSnapshotRequest,
    InstallSnapshotResponse,
    TimeoutNowRequest,
    TimeoutNowResponse
);
