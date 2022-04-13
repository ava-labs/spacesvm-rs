use clap::{crate_version, Arg, Command};
use log::info;

pub const APP_NAME: &str = "mini-kvvm-rs";

fn main() {
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
        .get_matches();

    let log_level = matches.value_of("LOG_LEVEL").unwrap_or("info").to_string();

    // ref. https://github.com/env-logger-rs/env_logger/issues/47
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, log_level),
    );

    info!("starting mini-kvvm-rs");
}
