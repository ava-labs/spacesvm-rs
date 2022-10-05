use std::io::Result;

use clap::{crate_version, Arg, Command};
use log::info;
use mini_kvvm::{genesis, vm};
use log4rs::{append::file::FileAppender, encode::pattern::PatternEncoder, Config, config::{Appender, Root}};


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
                .default_value("info"),
        )
        .subcommands(vec![command_genesis()])
        .get_matches();

    let log_level = matches
        .get_one::<String>("LOG_LEVEL")
        .map(String::as_str)
        .unwrap_or("info");

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

    info!("starting mini-kvvm-rs");
    vm::runner::Runner::new()
        .name(APP_NAME)
        .version(crate_version!())
        .log_level(log_level)
        .run()
        .await?;

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
      .build(Root::builder().appender("logfile").build(LevelFilter::Info))
      .unwrap();
    
    // init log4rs
    log4rs::init_config(config).unwrap();
}