use std::io::Result;

use avalanche_types::rpcchainvm;
use clap::{crate_version, Arg, Command};
use log::info;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};
use mini_kvvm::{genesis, vm};

pub const APP_NAME: &str = "mini-kvvm-rs";

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    let matches = Command::new(APP_NAME)
        .version(crate_version!())
        .about("Mini key-value VM for Avalanche in Rust")
        .arg(
            Arg::new("LOG_LEVEL")
                .long("log-level")
                .short('l')
                .help("Sets the log level")
                .required(false)
                .default_value("info"),
        )
        .subcommands(vec![command_genesis()])
        .get_matches();

    // let log_level = matches
    //     .get_one::<String>("LOG_LEVEL")
    //     .map(String::as_str)
    //     .unwrap_or("info");

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    // env_logger::init_from_env(
    //     env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, log_level),
    // );

    init_logger();

    if let Some(("genesis", sub_matches)) = matches.subcommand() {
        let author = sub_matches
            .get_one::<String>("AUTHOR")
            .map(String::as_str)
            .unwrap_or("");
        let msg = sub_matches
            .get_one::<String>("WELCOME_MESSAGE")
            .map(String::as_str)
            .unwrap_or("");
        let p = sub_matches
            .get_one::<String>("GENESIS_FILE_PATH")
            .map(String::as_str)
            .unwrap_or("");
        execute_genesis(author, msg, p).unwrap();
        return Ok(());
    }

    // Initialize broadcast stop channel used to terminate gRPC servers during shutdown.
    let (stop_ch_tx, stop_ch_rx): (
        tokio::sync::broadcast::Sender<()>,
        tokio::sync::broadcast::Receiver<()>,
    ) = tokio::sync::broadcast::channel(1);

    info!("starting mini-kvvm-rs");
    let vm_server = avalanche_types::rpcchainvm::vm::server::Server::new(
        Box::new(vm::ChainVm::new()),
        stop_ch_tx,
    );

    rpcchainvm::plugin::serve(vm_server, stop_ch_rx)
        .await
        .expect("failed to start gRPC server");

    // info!("starting mini-kvvm-rs");
    // vm::runner::Runner::new()
    //     .name(APP_NAME)
    //     .version(crate_version!())
    //     .log_level(log_level)
    //     .run()
    //     .await?;

    Ok(())
}

pub fn command_genesis() -> Command {
    Command::new("genesis")
        .about("Generates the genesis file")
        .arg(
            Arg::new("AUTHOR")
                .long("author")
                .short('a')
                .help("author of the genesis")
                .required(true)
                .default_value("subnet creator"),
        )
        .arg(
            Arg::new("WELCOME_MESSAGE")
                .long("welcome-message")
                .short('m')
                .help("message field in genesis")
                .required(true)
                .default_value("hi"),
        )
        .arg(
            Arg::new("GENESIS_FILE_PATH")
                .long("genesis-file-path")
                .short('p')
                .help("file path to save genesis file")
                .required(true),
        )
}

pub fn execute_genesis(author: &str, msg: &str, p: &str) -> Result<()> {
    let g = genesis::Genesis {
        author: String::from(author),
        welcome_message: String::from(msg),
    };
    g.sync(p)
}

fn init_logger() {
    let date = chrono::Utc::now();

    // create log file appender
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::default()))
        // set the file name based on the current date
        .build(format!("log/{}.log", date.timestamp_subsec_micros()))
        .unwrap();

    // add the logfile appender to the config
    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(
            Root::builder()
                .appender("logfile")
                .build(log::LevelFilter::Info),
        )
        .unwrap();

    // init log4rs
    log4rs::init_config(config).unwrap();
}
