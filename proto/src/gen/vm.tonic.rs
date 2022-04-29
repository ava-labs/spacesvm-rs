// @generated
/// Generated client implementations.
pub mod vm_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct VmClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl VmClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> VmClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Default + Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> VmClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            VmClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn initialize(
            &mut self,
            request: impl tonic::IntoRequest<super::InitializeRequest>,
        ) -> Result<tonic::Response<super::InitializeResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Initialize");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn set_state(
            &mut self,
            request: impl tonic::IntoRequest<super::SetStateRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/SetState");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn shutdown(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Shutdown");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn create_handlers(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::CreateHandlersResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/CreateHandlers");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn create_static_handlers(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<
                tonic::Response<super::CreateStaticHandlersResponse>,
                tonic::Status,
            > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/vm.VM/CreateStaticHandlers",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn connected(
            &mut self,
            request: impl tonic::IntoRequest<super::ConnectedRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Connected");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn disconnected(
            &mut self,
            request: impl tonic::IntoRequest<super::DisconnectedRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Disconnected");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn build_block(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::BuildBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/BuildBlock");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn parse_block(
            &mut self,
            request: impl tonic::IntoRequest<super::ParseBlockRequest>,
        ) -> Result<tonic::Response<super::ParseBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/ParseBlock");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/GetBlock");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn set_preference(
            &mut self,
            request: impl tonic::IntoRequest<super::SetPreferenceRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/SetPreference");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn health(
            &mut self,
            request: impl tonic::IntoRequest<super::HealthRequest>,
        ) -> Result<tonic::Response<super::HealthResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Health");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn version(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::VersionResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Version");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn app_request(
            &mut self,
            request: impl tonic::IntoRequest<super::AppRequestMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/AppRequest");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn app_request_failed(
            &mut self,
            request: impl tonic::IntoRequest<super::AppRequestFailedMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/AppRequestFailed");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn app_response(
            &mut self,
            request: impl tonic::IntoRequest<super::AppResponseMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/AppResponse");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn app_gossip(
            &mut self,
            request: impl tonic::IntoRequest<super::AppGossipMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/AppGossip");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn gather(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::GatherResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/Gather");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn block_verify(
            &mut self,
            request: impl tonic::IntoRequest<super::BlockVerifyRequest>,
        ) -> Result<tonic::Response<super::BlockVerifyResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/BlockVerify");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn block_accept(
            &mut self,
            request: impl tonic::IntoRequest<super::BlockAcceptRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/BlockAccept");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn block_reject(
            &mut self,
            request: impl tonic::IntoRequest<super::BlockRejectRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/BlockReject");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_ancestors(
            &mut self,
            request: impl tonic::IntoRequest<super::GetAncestorsRequest>,
        ) -> Result<tonic::Response<super::GetAncestorsResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/GetAncestors");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn batched_parse_block(
            &mut self,
            request: impl tonic::IntoRequest<super::BatchedParseBlockRequest>,
        ) -> Result<tonic::Response<super::BatchedParseBlockResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/BatchedParseBlock");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn verify_height_index(
            &mut self,
            request: impl tonic::IntoRequest<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::VerifyHeightIndexResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/VerifyHeightIndex");
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_block_id_at_height(
            &mut self,
            request: impl tonic::IntoRequest<super::GetBlockIdAtHeightRequest>,
        ) -> Result<tonic::Response<super::GetBlockIdAtHeightResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static("/vm.VM/GetBlockIDAtHeight");
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod vm_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with VmServer.
    #[async_trait]
    pub trait Vm: Send + Sync + 'static {
        async fn initialize(
            &self,
            request: tonic::Request<super::InitializeRequest>,
        ) -> Result<tonic::Response<super::InitializeResponse>, tonic::Status>;
        async fn set_state(
            &self,
            request: tonic::Request<super::SetStateRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn shutdown(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn create_handlers(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::CreateHandlersResponse>, tonic::Status>;
        async fn create_static_handlers(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::CreateStaticHandlersResponse>, tonic::Status>;
        async fn connected(
            &self,
            request: tonic::Request<super::ConnectedRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn disconnected(
            &self,
            request: tonic::Request<super::DisconnectedRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn build_block(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::BuildBlockResponse>, tonic::Status>;
        async fn parse_block(
            &self,
            request: tonic::Request<super::ParseBlockRequest>,
        ) -> Result<tonic::Response<super::ParseBlockResponse>, tonic::Status>;
        async fn get_block(
            &self,
            request: tonic::Request<super::GetBlockRequest>,
        ) -> Result<tonic::Response<super::GetBlockResponse>, tonic::Status>;
        async fn set_preference(
            &self,
            request: tonic::Request<super::SetPreferenceRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn health(
            &self,
            request: tonic::Request<super::HealthRequest>,
        ) -> Result<tonic::Response<super::HealthResponse>, tonic::Status>;
        async fn version(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::VersionResponse>, tonic::Status>;
        async fn app_request(
            &self,
            request: tonic::Request<super::AppRequestMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn app_request_failed(
            &self,
            request: tonic::Request<super::AppRequestFailedMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn app_response(
            &self,
            request: tonic::Request<super::AppResponseMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn app_gossip(
            &self,
            request: tonic::Request<super::AppGossipMsg>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn gather(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::GatherResponse>, tonic::Status>;
        async fn block_verify(
            &self,
            request: tonic::Request<super::BlockVerifyRequest>,
        ) -> Result<tonic::Response<super::BlockVerifyResponse>, tonic::Status>;
        async fn block_accept(
            &self,
            request: tonic::Request<super::BlockAcceptRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn block_reject(
            &self,
            request: tonic::Request<super::BlockRejectRequest>,
        ) -> Result<tonic::Response<::pbjson_types::Empty>, tonic::Status>;
        async fn get_ancestors(
            &self,
            request: tonic::Request<super::GetAncestorsRequest>,
        ) -> Result<tonic::Response<super::GetAncestorsResponse>, tonic::Status>;
        async fn batched_parse_block(
            &self,
            request: tonic::Request<super::BatchedParseBlockRequest>,
        ) -> Result<tonic::Response<super::BatchedParseBlockResponse>, tonic::Status>;
        async fn verify_height_index(
            &self,
            request: tonic::Request<::pbjson_types::Empty>,
        ) -> Result<tonic::Response<super::VerifyHeightIndexResponse>, tonic::Status>;
        async fn get_block_id_at_height(
            &self,
            request: tonic::Request<super::GetBlockIdAtHeightRequest>,
        ) -> Result<tonic::Response<super::GetBlockIdAtHeightResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct VmServer<T: Vm> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Vm> VmServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.accept_compression_encodings.enable_gzip();
            self
        }
        /// Compress responses with `gzip`, if the client supports it.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.send_compression_encodings.enable_gzip();
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for VmServer<T>
    where
        T: Vm,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/vm.VM/Initialize" => {
                    #[allow(non_camel_case_types)]
                    struct InitializeSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::InitializeRequest>
                    for InitializeSvc<T> {
                        type Response = super::InitializeResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::InitializeRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).initialize(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = InitializeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/SetState" => {
                    #[allow(non_camel_case_types)]
                    struct SetStateSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::SetStateRequest>
                    for SetStateSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetStateRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).set_state(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SetStateSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Shutdown" => {
                    #[allow(non_camel_case_types)]
                    struct ShutdownSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for ShutdownSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).shutdown(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ShutdownSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/CreateHandlers" => {
                    #[allow(non_camel_case_types)]
                    struct CreateHandlersSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for CreateHandlersSvc<T> {
                        type Response = super::CreateHandlersResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_handlers(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateHandlersSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/CreateStaticHandlers" => {
                    #[allow(non_camel_case_types)]
                    struct CreateStaticHandlersSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for CreateStaticHandlersSvc<T> {
                        type Response = super::CreateStaticHandlersResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_static_handlers(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateStaticHandlersSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Connected" => {
                    #[allow(non_camel_case_types)]
                    struct ConnectedSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::ConnectedRequest>
                    for ConnectedSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ConnectedRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).connected(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ConnectedSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Disconnected" => {
                    #[allow(non_camel_case_types)]
                    struct DisconnectedSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::DisconnectedRequest>
                    for DisconnectedSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::DisconnectedRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).disconnected(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = DisconnectedSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/BuildBlock" => {
                    #[allow(non_camel_case_types)]
                    struct BuildBlockSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for BuildBlockSvc<T> {
                        type Response = super::BuildBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).build_block(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BuildBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/ParseBlock" => {
                    #[allow(non_camel_case_types)]
                    struct ParseBlockSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::ParseBlockRequest>
                    for ParseBlockSvc<T> {
                        type Response = super::ParseBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ParseBlockRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).parse_block(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ParseBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/GetBlock" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::GetBlockRequest>
                    for GetBlockSvc<T> {
                        type Response = super::GetBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_block(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/SetPreference" => {
                    #[allow(non_camel_case_types)]
                    struct SetPreferenceSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::SetPreferenceRequest>
                    for SetPreferenceSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SetPreferenceRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).set_preference(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SetPreferenceSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Health" => {
                    #[allow(non_camel_case_types)]
                    struct HealthSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::HealthRequest>
                    for HealthSvc<T> {
                        type Response = super::HealthResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::HealthRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).health(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = HealthSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Version" => {
                    #[allow(non_camel_case_types)]
                    struct VersionSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for VersionSvc<T> {
                        type Response = super::VersionResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).version(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VersionSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/AppRequest" => {
                    #[allow(non_camel_case_types)]
                    struct AppRequestSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::AppRequestMsg>
                    for AppRequestSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppRequestMsg>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).app_request(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppRequestSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/AppRequestFailed" => {
                    #[allow(non_camel_case_types)]
                    struct AppRequestFailedSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::AppRequestFailedMsg>
                    for AppRequestFailedSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppRequestFailedMsg>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).app_request_failed(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppRequestFailedSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/AppResponse" => {
                    #[allow(non_camel_case_types)]
                    struct AppResponseSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::AppResponseMsg>
                    for AppResponseSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppResponseMsg>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).app_response(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppResponseSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/AppGossip" => {
                    #[allow(non_camel_case_types)]
                    struct AppGossipSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::AppGossipMsg>
                    for AppGossipSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AppGossipMsg>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).app_gossip(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AppGossipSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/Gather" => {
                    #[allow(non_camel_case_types)]
                    struct GatherSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for GatherSvc<T> {
                        type Response = super::GatherResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).gather(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GatherSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/BlockVerify" => {
                    #[allow(non_camel_case_types)]
                    struct BlockVerifySvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::BlockVerifyRequest>
                    for BlockVerifySvc<T> {
                        type Response = super::BlockVerifyResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BlockVerifyRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).block_verify(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BlockVerifySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/BlockAccept" => {
                    #[allow(non_camel_case_types)]
                    struct BlockAcceptSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::BlockAcceptRequest>
                    for BlockAcceptSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BlockAcceptRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).block_accept(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BlockAcceptSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/BlockReject" => {
                    #[allow(non_camel_case_types)]
                    struct BlockRejectSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::BlockRejectRequest>
                    for BlockRejectSvc<T> {
                        type Response = ::pbjson_types::Empty;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BlockRejectRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).block_reject(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BlockRejectSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/GetAncestors" => {
                    #[allow(non_camel_case_types)]
                    struct GetAncestorsSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<super::GetAncestorsRequest>
                    for GetAncestorsSvc<T> {
                        type Response = super::GetAncestorsResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetAncestorsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_ancestors(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetAncestorsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/BatchedParseBlock" => {
                    #[allow(non_camel_case_types)]
                    struct BatchedParseBlockSvc<T: Vm>(pub Arc<T>);
                    impl<
                        T: Vm,
                    > tonic::server::UnaryService<super::BatchedParseBlockRequest>
                    for BatchedParseBlockSvc<T> {
                        type Response = super::BatchedParseBlockResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BatchedParseBlockRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).batched_parse_block(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BatchedParseBlockSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/VerifyHeightIndex" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyHeightIndexSvc<T: Vm>(pub Arc<T>);
                    impl<T: Vm> tonic::server::UnaryService<::pbjson_types::Empty>
                    for VerifyHeightIndexSvc<T> {
                        type Response = super::VerifyHeightIndexResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<::pbjson_types::Empty>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_height_index(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyHeightIndexSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/vm.VM/GetBlockIDAtHeight" => {
                    #[allow(non_camel_case_types)]
                    struct GetBlockIDAtHeightSvc<T: Vm>(pub Arc<T>);
                    impl<
                        T: Vm,
                    > tonic::server::UnaryService<super::GetBlockIdAtHeightRequest>
                    for GetBlockIDAtHeightSvc<T> {
                        type Response = super::GetBlockIdAtHeightResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetBlockIdAtHeightRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).get_block_id_at_height(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetBlockIDAtHeightSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Vm> Clone for VmServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Vm> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Vm> tonic::transport::NamedService for VmServer<T> {
        const NAME: &'static str = "vm.VM";
    }
}
