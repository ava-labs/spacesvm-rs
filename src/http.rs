#![allow(dead_code)]
#![allow(unused_imports)]

use httppb::http_server::HttpServer;
use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params, Value};

use crate::httppb;

pub struct Server {
    http_handler: IoHandler,
}

impl Server {
    pub fn new(http_handler: IoHandler) -> Self {
        Server { http_handler }
    }
}

#[tonic::async_trait]
impl httppb::http_server::Http for Server {
    async fn handle(
        &self,
        _request: tonic::Request<httppb::HttpRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        Err(tonic::Status::unimplemented("handle"))
    }

    async fn handle_simple(
        &self,
        _request: tonic::Request<httppb::HandleSimpleHttpRequest>,
    ) -> Result<tonic::Response<httppb::HandleSimpleHttpResponse>, tonic::Status> {
        Err(tonic::Status::unimplemented("handle_simple"))
    }
}
