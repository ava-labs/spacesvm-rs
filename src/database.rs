use log::info;
use tonic::transport::Channel;
use tonic::Status;

use crate::rpcdbpb::{database_client::DatabaseClient, *};

pub struct Database {
    client: DatabaseClient<Channel>,
}

impl Database {
    pub fn new(client: DatabaseClient<Channel>) -> Self {
        Database { client }
    }
}

#[tonic::async_trait]
impl crate::rpcdbpb::database_server::Database for Database {
    async fn get(&mut self, key: Vec<u8>) -> Result<Option<Vec<u8>>, Status> {
        let req = GetRequest { key };
        let resp = self.client.get(req).await?.into_inner();
        // TODO: handle db error
        Ok(Some(resp.value))
    }

    async fn put(&mut self, key: Vec<u8>, value: Vec<u8>) -> Result<(), Status> {
        let req = PutRequest { key, value };
        let resp = self.client.put(req).await?.into_inner();
        info!("db response {:?}", resp);
        // TODO: handle errors
        Ok(())
    }

    async fn delete(&mut self, key: Vec<u8>) -> Result<(), Status> {
        let req = DeleteRequest { key };
        let resp = self.client.delete(req).await?.into_inner();
        info!("db response {:?}", resp);
        // TODO: handle errors
        Ok(())
    }

    async fn close(&mut self) -> Result<tonic::Response<CloseResponse>, tonic::Status> {
        let req = CloseRequest {};
        Ok(self.client.close(req).await?)
    }
}
