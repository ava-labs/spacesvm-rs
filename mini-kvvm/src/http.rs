use avalanche_proto::{
    http::{
        http_server::Http,
        HttpRequest, HandleSimpleHttpResponse, HandleSimpleHttpRequest
    },
    google::protobuf::Empty,

};
use jsonrpc_http_server::jsonrpc_core::IoHandler;
use prost::bytes::Bytes;
use tonic::{Request, Response, Status};


pub struct Server {
    http_handler: IoHandler,
}

impl Server {
    pub fn new(http_handler: IoHandler) -> Self {
        Server { http_handler }
    }
}

#[tonic::async_trait]
impl Http for Server {
    async fn handle(&self, _req: Request<HttpRequest>) -> Result<Response<Empty>, Status> {
        Err(tonic::Status::unimplemented("handle"))
    }

    // handle_simple handles http requests over http2 using a simple request response model.
    // Websockets are not supported.
    async fn handle_simple(
        &self,
        req: Request<HandleSimpleHttpRequest>,
    ) -> Result<Response<HandleSimpleHttpResponse>, Status> {
        let req = req.into_inner();
        let body = String::from_utf8(req.body.to_vec())
            .map_err(|_| Status::internal("failed to convert request body to utf8 string"));

        let handler_resp = self
            .http_handler
            .handle_request(body.unwrap().as_str())
            .await
            .ok_or_else(|| Status::internal("failed to get response from rpc handler"))?;

        let resp = HandleSimpleHttpResponse {
            code: 200,
            body: Bytes::from(handler_resp.into_bytes()),
            // TODO: headers?
            headers: vec![],
        };

        Ok(Response::new(resp))
    }
}