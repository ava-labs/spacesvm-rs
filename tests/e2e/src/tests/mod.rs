use std::{
    thread,
    time::{Duration, Instant},
};

use avalanche_network_runner_sdk::{BlockchainSpec, Client, GlobalConfig, StartRequest};
use log::{info, warn};

#[tokio::test]
async fn e2e() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let (ep, is_set) = crate::get_network_runner_grpc_endpoint();
    assert!(is_set);

    let cli = Client::new(&ep).await;

    // Allow server time to become ready.
    thread::sleep(Duration::from_millis(2000));

    info!("ping...");
    let resp = cli.ping().await.expect("failed ping");
    info!("network-runner is running (ping response {:?})", resp);

    let (exec_path, is_set) = crate::get_network_runner_avalanchego_path();
    assert!(is_set);
    info!("exec_path {:?})", exec_path);

    let (whitelisted_subnets, is_set) = crate::get_network_runner_whitelisted_subnets();
    assert!(is_set);
    info!("whitelisted_subnets {:?})", whitelisted_subnets);

    let global_config = GlobalConfig {
        log_level: String::from("info"),
    };

    let mut blockchain_specs = Vec::new();
    blockchain_specs.push(BlockchainSpec {
        vm_name: "minikvvm".to_string(),
        genesis: "/tmp/minikvvm.genesis.json".to_string(),
        subnet_id: None,
    });

    info!("starting...");
    let resp = cli
        .start(StartRequest {
            exec_path: exec_path,
            whitelisted_subnets: Some(whitelisted_subnets),
            global_node_config: Some(serde_json::to_string(&global_config).unwrap()),
            blockchain_specs,
            ..Default::default()
        })
        .await
        .expect("failed start");
    info!(
        "started avalanchego cluster with network-runner: {:?}",
        resp
    );

    // enough time for network-runner to get ready
    thread::sleep(Duration::from_secs(20));

    info!("checking custom vm healthiness...");
    let mut ready = false;
    let timeout = Duration::from_secs(300);
    let interval = Duration::from_secs(15);
    let start = Instant::now();
    let mut cnt: u128 = 0;
    loop {
        let elapsed = start.elapsed();
        if elapsed.gt(&timeout) {
            break;
        }

        let itv = {
            if cnt == 0 {
                // first poll with no wait
                Duration::from_secs(1)
            } else {
                interval
            }
        };
        thread::sleep(itv);

        cnt += 1;
        ready = {
            match cli.status().await {
                Ok(status) => {
                    if status.cluster_info.is_some() {
                        warn!("client resp: {:?}", resp);
                        continue;
                    }
                    warn!("custom vms healthy");
                    true
                }

                Err(e) => {
                    warn!("not healthy yet {}", e);
                    false
                }
            }
        };

        if ready {
            break;
        }
    }

    assert!(ready);

    info!("checking status...");
    let status = cli.status().await.expect("failed status");
    assert!(status.cluster_info.is_some());
    let cluster_info = status.cluster_info.unwrap();
    let mut rpc_eps: Vec<String> = Vec::new();
    for (node_name, iv) in cluster_info.node_infos.into_iter() {
        info!("{}: {}", node_name, iv.uri);
        rpc_eps.push(iv.uri.clone());
    }
    info!("avalanchego RPC endpoints: {:?}", rpc_eps);

    // TODO: do some tests...

    info!("stopping...");
    let _resp = cli.stop().await.expect("failed stop");
    info!("successfully stopped network");
}
