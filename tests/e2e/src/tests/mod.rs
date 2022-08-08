use std::{
    process::Command,
    thread,
    time::{Duration, Instant},
};

use log::{info, warn};

use avalanche_network_runner_sdk::{Client, GlobalConfig, StartRequest};

const RELEASE: &str = "v1.7.16";

#[tokio::test]
async fn e2e() {
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let (ep, is_set) = get_network_runner_grpc_endpoint();
    assert!(is_set);

    let cli = Client::new(&ep).await;

    info!("ping...");
    let resp = cli.ping().await.expect("failed ping");
    info!("network-runner is running (ping response {:?})", resp);

    // Download avalanchego
    let (exec_path, _plugin_path) =
        avalanche_installer::avalanchego::download(None, None, Some(String::from(RELEASE)))
            .await
            .expect("failed to download avalanchego");

    info!(
        "running avalanchego version {}",
        get_avalanchego_version(&exec_path)
    );

    let global_config = GlobalConfig {
        log_level: String::from("info"),
    };

    // TODO: add custom vms for "mini-kvvm"
    info!("starting...");
    let resp = cli
        .start(StartRequest {
            exec_path,
            global_node_config: Some(serde_json::to_string(&global_config).unwrap()),
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

    info!("checking cluster healthiness...");
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

        ready = {
            match cli.health().await {
                Ok(_) => {
                    info!("healthy now!");
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

        cnt += 1;
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

fn get_avalanchego_version(exec_path: &String) -> String {
    let output = Command::new(exec_path)
        .arg("--version")
        .output()
        .expect("failed to get avalanchego version");
    format!("{}", String::from_utf8(output.stdout).unwrap())
}

fn get_network_runner_grpc_endpoint() -> (String, bool) {
    match std::env::var("NETWORK_RUNNER_GRPC_ENDPOINT") {
        Ok(s) => (s, true),
        _ => (String::new(), false),
    }
}
