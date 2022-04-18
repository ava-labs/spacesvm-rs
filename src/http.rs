#![allow(dead_code)]
#![allow(unused_imports)]
use tonic::{Request, Response, Status};

use httppb::http_server::HttpServer;
use jsonrpc_http_server::jsonrpc_core::{IoHandler, Params, Value};
use log::info;

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
    async fn handle(&self, req: Request<httppb::HttpRequest>) -> Result<Response<()>, Status> {
        Err(tonic::Status::unimplemented("handle"))
    }

    async fn handle_simple(
        &self,
        req: Request<httppb::HandleSimpleHttpRequest>,
    ) -> Result<Response<httppb::HandleSimpleHttpResponse>, Status> {
        let body = String::from_utf8(req.into_inner().body)
            .expect("failed to convert request body to utf8 string");

        let handler_resp = self
            .http_handler
            .handle_request(body.as_str())
            .await
            .ok_or_else(|| Status::internal("failed to get response from rpc handler"))?;

        let resp_body_bytes = handler_resp.into_bytes();
        let resp = httppb::HandleSimpleHttpResponse {
            code: 200,
            body: resp_body_bytes,
            // TODO: handle
            headers: vec![],
        };

        Ok(Response::new(resp))
    }
}
