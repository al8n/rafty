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
use crate::config::{Configuration, ServerID, ServerSuffrage};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

/// `Commitment` is used to advance the leader's commit index. The leader and
/// replication routines report in newly written entries with Match(), and
/// this notifies on commit_ch when the commit index has advanced.
pub struct Commitment {
    /// notified when commitIndex increases
    commit_rx_ch: UnboundedReceiver<()>,
    commit_tx_ch: UnboundedSender<()>,

    /// voter ID to log index: the server stores up through this log entry
    match_indexes: HashMap<ServerID, u64>,
    /// a quorum stores up through this log entry. monotonically increases.
    commit_index: u64,

    /// the first index of this leader's term: this needs to be replicated to a
    /// majority of the cluster before this leader may mark anything committed
    /// (per Raft's commitment rule)
    start_index: u64,
}

impl Commitment {
    pub fn new(
        commit_tx_ch: UnboundedSender<()>,
        commit_rx_ch: UnboundedReceiver<()>,
        cfg: Configuration,
        start_index: u64,
    ) -> Arc<Mutex<Self>> {
        let match_indexes = {
            let mut m = HashMap::<ServerID, u64>::new();

            for s in cfg.servers {
                if s.suffrage == ServerSuffrage::Voter {
                    m.insert(s.id, 0);
                }
            }
            m
        };

        let val = Self {
            commit_rx_ch,
            commit_tx_ch,
            match_indexes,
            commit_index: 0,
            start_index,
        };
        Arc::new(Mutex::new(val))
    }

    /// Called when a new cluster membership configuration is created: it will be
    /// used to determine `Commitment` from now on. 'configuration' is the servers in
    /// the cluster.
    pub fn set_configuration(&mut self, cfg: Configuration) {
        let mut new_match_indexes = HashMap::<ServerID, u64>::new();
        for s in cfg.servers {
            if s.suffrage == ServerSuffrage::Voter {
                new_match_indexes.insert(s.id, *self.match_indexes.get(&s.id).unwrap_or(&0u64));
                // defaults to 0
            }
        }
        self.match_indexes = new_match_indexes;
        self.recalculate();
    }

    /// Called by leader after `commit_rx_ch` is notified
    pub fn get_commit_index(&self) -> u64 {
        self.commit_index
    }

    /// `match_server` is called once a server completes writing entries to disk: either the
    /// leader has written the new entry or a follower has replied to an
    /// AppendEntries RPC. The given server's disk agrees with this server's log up
    /// through the given index.
    pub fn match_server(&mut self, server: ServerID, match_index: u64) {
        match self.match_indexes.get_mut(&server) {
            None => {}
            Some(prev) => {
                if match_index > *prev {
                    *prev = match_index;
                    self.recalculate();
                }
            }
        }
    }

    /// Internal helper to calculate new `commit_index` from `match_indexes`.
    /// Must be called with lock held.
    fn recalculate(&mut self) {
        let size = self.match_indexes.len();
        if size == 0 {
            return;
        }

        let mut matched = Vec::<u64>::with_capacity(size);
        for (_, v) in &self.match_indexes {
            matched.push(*v);
        }
        matched.sort();

        let quorum_match_index = matched[(matched.len() - 1) / 2];
        if quorum_match_index > self.commit_index && quorum_match_index >= self.start_index {
            self.commit_index = quorum_match_index;
            self.commit_tx_ch.send(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Server, ServerAddress};
    use tokio::sync::mpsc::unbounded_channel;

    fn make_configuration(voters: Vec<u64>) -> Configuration {
        let mut servers = Vec::<Server>::new();

        for v in voters {
            servers.push(Server {
                suffrage: ServerSuffrage::Voter,
                id: v,
                address: ServerAddress::from(format!("{}addr", v)),
            });
        }

        Configuration::with_servers(servers)
    }

    fn voters(n: u64) -> Configuration {
        let severs = vec![1u64, 2, 3, 4, 5, 6, 7];
        if n > 7 {
            panic!("only up to 7 servers implemented")
        }

        make_configuration(
            severs
                .into_iter()
                .filter(|val| *val <= n)
                .collect::<Vec<u64>>(),
        )
    }

    #[tokio::test]
    async fn test_commitment_set_voters() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(
            commit_tx,
            commit_rx,
            make_configuration(vec![100, 101, 102]),
            0,
        );

        c.lock().match_server(100, 10);
        c.lock().match_server(101, 20);
        c.lock().match_server(102, 30);

        // commit_index: 20
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock()
            .set_configuration(make_configuration(vec![102, 103, 104]));
        // 102: 30, 103: 0, 104: 0
        c.lock().match_server(104, 40);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 30, "expected 30 entries committed, found {}", idx);

        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }
    }

    #[test]
    fn test_commitment_match_max() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, voters(5), 4);

        c.lock().match_server(1, 8);
        c.lock().match_server(2, 8);
        c.lock().match_server(2, 1);
        c.lock().match_server(3, 8);

        let idx = c.lock().get_commit_index();
        assert_eq!(
            idx, 8,
            "calling match with an earlier index should be ignored"
        )
    }

    #[tokio::test]
    async fn test_commitment_match_nonvoting() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, voters(5), 4);

        c.lock().match_server(1, 8);
        c.lock().match_server(2, 8);
        c.lock().match_server(3, 8);

        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().match_server(90, 10);
        c.lock().match_server(91, 10);
        c.lock().match_server(92, 10);

        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 8, "non-voting servers shouldn't be able to commit");
    }

    #[tokio::test]
    async fn test_commitment_recalculate() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, voters(5), 0);

        c.lock().match_server(1, 30);
        c.lock().match_server(2, 20);

        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 0, "shouldn't commit after two of five servers");

        c.lock().match_server(3, 10);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 10, "expected 10 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().match_server(4, 15);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 15, "expected 15 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().set_configuration(voters(3));
        // 1:30, 2: 20, 3: 10
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 20, "expected 20 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().set_configuration(voters(4));
        // 1:30, 2: 20, 3: 10, 4: 0
        c.lock().match_server(2, 25);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 20, "expected 20 entries committed, found {}", idx);

        c.lock().match_server(4, 23);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 23, "expected 23 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }
    }

    #[tokio::test]
    async fn test_commitment_recalculate_start_index() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, voters(5), 4);

        c.lock().match_server(1, 3);
        c.lock().match_server(2, 3);
        c.lock().match_server(3, 3);

        let idx = c.lock().get_commit_index();
        assert_eq!(
            idx, 0,
            "can't commit until startIndex is replicated to a quorum"
        );

        c.lock().match_server(1, 4);
        c.lock().match_server(2, 4);
        c.lock().match_server(3, 4);

        let idx = c.lock().get_commit_index();
        assert_eq!(
            idx, 4,
            "should be able to commit startIndex once replicated to a quorum"
        );
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }
    }

    #[tokio::test]
    async fn test_commitment_no_voter_sanity() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, make_configuration(vec![]), 4);

        c.lock().match_server(1, 10);
        c.lock().set_configuration(make_configuration(vec![]));
        c.lock().match_server(1, 10);

        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 0, "no voting servers: shouldn't be able to commit");

        // add a voter so we can commit something and then remove it
        c.lock().set_configuration(voters(1));
        c.lock().match_server(1, 10);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 10, "expected 10 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().set_configuration(make_configuration(vec![]));
        c.lock().match_server(1, 20);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 10, "expected 10 entries committed, found {}", idx);
    }

    #[tokio::test]
    async fn test_commitment_single_voter() {
        let (commit_tx, commit_rx) = unbounded_channel::<()>();
        let c = Commitment::new(commit_tx, commit_rx, voters(1), 4);
        c.lock().match_server(1, 10);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 10, "expected 10 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }

        c.lock().set_configuration(voters(1));
        c.lock().match_server(1, 12);
        let idx = c.lock().get_commit_index();
        assert_eq!(idx, 12, "expected 12 entries committed, found {}", idx);
        let rx = c.lock().commit_rx_ch.recv().await;
        if let Some(v) = rx {
            assert_eq!(v, (), "expected commit notify");
        }
    }
}
