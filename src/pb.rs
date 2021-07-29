#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct HardState {
    /// latest term server has seen (initialized to 0 on first boot, increases monotonically)
    #[prost(uint64, required, tag = "1")]
    pub current_term: u64,
    /// candidateId that received vote in current term (or null if none)
    #[prost(string, optional, tag = "2")]
    pub voted_for: ::core::option::Option<::prost::alloc::string::String>,
    /// index of highest log entry known to be committed (initialized to 0, increases monotonically)
    #[prost(uint64, required, tag = "3")]
    pub commit_index: u64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct Entry {
    #[prost(enumeration = "EntryType", optional, tag = "1")]
    pub typ: ::core::option::Option<i32>,
    #[prost(bytes = "vec", optional, tag = "2")]
    pub data: ::core::option::Option<::prost::alloc::vec::Vec<u8>>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AppendEntriesRequest {
    /// leader's term
    #[prost(uint64, required, tag = "1")]
    pub term: u64,
    /// so follower can redirect clients
    #[prost(string, required, tag = "2")]
    pub leader_id: ::prost::alloc::string::String,
    /// index of log entry immediately preceding new ones
    #[prost(uint64, optional, tag = "3")]
    pub prev_log_index: ::core::option::Option<u64>,
    /// term of prev_log_index entry
    #[prost(uint64, optional, tag = "4")]
    pub prev_log_term: ::core::option::Option<u64>,
    /// log entries to store (empty for heartbeat; may send more
    /// than one for efficiency)
    #[prost(message, repeated, tag = "5")]
    pub entries: ::prost::alloc::vec::Vec<Entry>,
    /// leader's commit index
    #[prost(uint64, required, tag = "6")]
    pub leader_commit: u64,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AppendEntriesResponse {
    /// current term, for leader to update itself
    #[prost(uint64, required, tag = "1")]
    pub term: u64,
    /// true if follower contained entry matching prev_log_index
    /// and prev_log_term
    #[prost(bool, required, tag = "2")]
    pub success: bool,
}
/// The vote request candidates (§5.2).
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct VoteRequest {
    /// candidate's term
    #[prost(uint64, required, tag = "1")]
    pub term: u64,
    /// candidate requesting vote
    #[prost(string, required, tag = "2")]
    pub candidate_id: ::prost::alloc::string::String,
    /// index of candidate's last log entry
    #[prost(uint64, optional, tag = "3")]
    pub last_log_index: ::core::option::Option<u64>,
    /// term of candidate's last log entry
    #[prost(uint64, optional, tag = "4")]
    pub last_log_term: ::core::option::Option<u64>,
}
/// The vote response
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct VoteResponse {
    /// current term, for candidate to update itself
    #[prost(uint64, required, tag = "1")]
    pub term: u64,
    /// true means candidate received vote
    #[prost(bool, required, tag = "2")]
    pub vote_granted: bool,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AddServerRequest {
    /// address of server to add to configuration
    #[prost(string, required, tag = "1")]
    pub addr: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct AddServerResponse {
    /// OK if server was added successfully
    /// NOT_LEADER if not leader
    /// TIMEOUT if new server does not make progress for an election timeout or if the last round takes longer than the election timeout.
    #[prost(enumeration = "Status", required, tag = "1")]
    pub status: i32,
    /// address of recent leader, if known
    #[prost(string, optional, tag = "2")]
    pub leader_hint: ::core::option::Option<::prost::alloc::string::String>,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct RemoveServerRequest {
    /// address of server to remove from configuration
    #[prost(string, required, tag = "1")]
    pub old_server: ::prost::alloc::string::String,
}
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, ::prost::Message)]
pub struct RemoveServerResponse {
    /// OK if server was removed successfully
    #[prost(enumeration = "Status", required, tag = "1")]
    pub status: i32,
    /// address of recent leader, if known
    #[prost(string, optional, tag = "2")]
    pub leader_hint: ::core::option::Option<::prost::alloc::string::String>,
}
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum EntryType {
    Normal = 0,
    OldSnapshot = 1,
    PeerState = 2,
    AddPeer = 3,
    RemovePeer = 4,
    LeaderTransfer = 5,
    Snapshot = 6,
}
#[derive(
    serde::Serialize,
    serde::Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
)]
#[repr(i32)]
pub enum Status {
    Ok = 0,
    NotLeader = 1,
    Timeout = 2,
    SessionExpired = 3,
}
#[doc = r" Generated client implementations."]
pub mod raft_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct RaftClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl RaftClient<tonic::transport::Channel> {
        #[doc = r" Attempt to create a new client by connecting to a given endpoint."]
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> RaftClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::ResponseBody: Body + Send + Sync + 'static,
        T::Error: Into<StdError>,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> RaftClient<InterceptedService<T, F>>
        where
            F: FnMut(tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status>,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            RaftClient::new(InterceptedService::new(inner, interceptor))
        }
        #[doc = r" Compress requests with `gzip`."]
        #[doc = r""]
        #[doc = r" This requires the server to support it otherwise it might respond with an"]
        #[doc = r" error."]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        #[doc = r" Enable decompressing responses with `gzip`."]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        #[doc = " Invoked by leader to replicate log entries; also used as heartbeat."]
        pub async fn append_entries(
            &mut self,
            request: impl tonic::IntoRequest<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::AppendEntriesResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/pb.Raft/AppendEntries");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " Invoked by candidates to gather votes."]
        pub async fn vote(
            &mut self,
            request: impl tonic::IntoRequest<super::VoteRequest>,
        ) -> Result<tonic::Response<super::VoteResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/pb.Raft/Vote");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " Invoked by admin to add a server to the cluster configuration"]
        pub async fn add_server(
            &mut self,
            request: impl tonic::IntoRequest<super::AddServerRequest>,
        ) -> Result<tonic::Response<super::AddServerResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/pb.Raft/AddServer");
            self.inner.unary(request.into_request(), path, codec).await
        }
        #[doc = " Invoked by admin to remove a server from the cluster configuration"]
        pub async fn remove_server(
            &mut self,
            request: impl tonic::IntoRequest<super::RemoveServerRequest>,
        ) -> Result<tonic::Response<super::RemoveServerResponse>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/pb.Raft/RemoveServer");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
#[doc = r" Generated server implementations."]
pub mod raft_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[doc = "Generated trait containing gRPC methods that should be implemented for use with RaftServer."]
    #[async_trait]
    pub trait Raft: Send + Sync + 'static {
        #[doc = " Invoked by leader to replicate log entries; also used as heartbeat."]
        async fn append_entries(
            &self,
            request: tonic::Request<super::AppendEntriesRequest>,
        ) -> Result<tonic::Response<super::AppendEntriesResponse>, tonic::Status>;
        #[doc = " Invoked by candidates to gather votes."]
        async fn vote(
            &self,
            request: tonic::Request<super::VoteRequest>,
        ) -> Result<tonic::Response<super::VoteResponse>, tonic::Status>;
        #[doc = " Invoked by admin to add a server to the cluster configuration"]
        async fn add_server(
            &self,
            request: tonic::Request<super::AddServerRequest>,
        ) -> Result<tonic::Response<super::AddServerResponse>, tonic::Status>;
        #[doc = " Invoked by admin to remove a server from the cluster configuration"]
        async fn remove_server(
            &self,
            request: tonic::Request<super::RemoveServerRequest>,
        ) -> Result<tonic::Response<super::RemoveServerResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct RaftServer<T: Raft> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Raft> RaftServer<T> {
        pub fn new(inner: T) -> Self {
            let inner = Arc::new(inner);
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: FnMut(tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status>,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for RaftServer<T>
    where
        T: Raft,
        B: Body + Send + Sync + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = Never;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/pb.Raft/AppendEntries" => {
                    #[allow(non_camel_case_types)]
                    struct AppendEntriesSvc<T: Raft>(pub Arc<T>);
                    impl<T: Raft> tonic::server::UnaryService<super::AppendEntriesRequest> for AppendEntriesSvc<T> {
                        type Response = super::AppendEntriesResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppendEntriesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).append_entries(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppendEntriesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pb.Raft/Vote" => {
                    #[allow(non_camel_case_types)]
                    struct VoteSvc<T: Raft>(pub Arc<T>);
                    impl<T: Raft> tonic::server::UnaryService<super::VoteRequest> for VoteSvc<T> {
                        type Response = super::VoteResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::VoteRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).vote(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VoteSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pb.Raft/AddServer" => {
                    #[allow(non_camel_case_types)]
                    struct AddServerSvc<T: Raft>(pub Arc<T>);
                    impl<T: Raft> tonic::server::UnaryService<super::AddServerRequest> for AddServerSvc<T> {
                        type Response = super::AddServerResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AddServerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).add_server(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AddServerSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/pb.Raft/RemoveServer" => {
                    #[allow(non_camel_case_types)]
                    struct RemoveServerSvc<T: Raft>(pub Arc<T>);
                    impl<T: Raft> tonic::server::UnaryService<super::RemoveServerRequest> for RemoveServerSvc<T> {
                        type Response = super::RemoveServerResponse;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::RemoveServerRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).remove_server(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = RemoveServerSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: Raft> Clone for RaftServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Raft> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Raft> tonic::transport::NamedService for RaftServer<T> {
        const NAME: &'static str = "pb.Raft";
    }
}
