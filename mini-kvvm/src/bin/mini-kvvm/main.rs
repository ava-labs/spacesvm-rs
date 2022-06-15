use std::io::Result;

use avalanche_types::rpcchainvm::plugin;
use clap::{crate_version, Arg, Command};
use log::info;
use kvvm::{engine, genesis, kvvm};

pub const APP_NAME: &str = "mini-kvvm-rs";

#[tokio::main]
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
                .takes_value(true)
                .possible_value("debug")
                .possible_value("info")
                .allow_invalid_utf8(false)
                .default_value("info"),
        )
        .subcommands(vec![command_genesis()])
        .get_matches();

    let log_level = matches.value_of("LOG_LEVEL").unwrap_or("info").to_string();

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, log_level),
    );

    if let Some(("genesis", sub_matches)) = matches.subcommand() {
        let author = sub_matches.value_of("AUTHOR").unwrap_or("");
        let msg = sub_matches.value_of("WELCOME_MESSAGE").unwrap_or("");
        let p = sub_matches.value_of("GENESIS_FILE_PATH").unwrap_or("");
        execute_genesis(author, msg, p).unwrap();
        return Ok(());
    }

    info!("starting mini-kvvm-rs");
    plugin::serve(engine::VmServer::new(kvvm::ChainVmInterior::new())).await
}

pub fn command_genesis() -> Command<'static> {
    Command::new("genesis")
        .about("Generates the genesis file")
        .arg(
            Arg::new("AUTHOR")
                .long("author")
                .short('a')
                .help("author of the genesis")
                .required(true)
                .takes_value(true)
                .allow_invalid_utf8(false)
                .default_value("subnet creator"),
        )
        .arg(
            Arg::new("WELCOME_MESSAGE")
                .long("welcome-message")
                .short('m')
                .help("message field in genesis")
                .required(true)
                .takes_value(true)
                .allow_invalid_utf8(false)
                .default_value("hi"),
        )
        .arg(
            Arg::new("GENESIS_FILE_PATH")
                .long("genesis-file-path")
                .short('p')
                .help("file path to save genesis file")
                .required(true)
                .takes_value(true)
                .allow_invalid_utf8(false),
        )
}

pub fn execute_genesis(author: &str, msg: &str, p: &str) -> Result<()> {
    let g = genesis::Genesis {
        author: String::from(author),
        welcome_message: String::from(msg),
    };
    g.sync(p)
}
