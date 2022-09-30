use clap::{Parser, Subcommand};
use secp256k1::{rand, SecretKey};
use std::error;
use std::fs::File;
use std::io::{Result, Write};
use std::path::Path;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Endpoint for RPC calls.
    #[clap(long)]
    endpoint: String,

    /// Private key file.
    #[clap(long, default_value = ".mini-kvvm-cli-pk")]
    private_key_file: String,

    /// Which subcommand to call.
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Bucket {
        bucket: String,
    },
    Set {
        bucket: String,
        key: String,
        value: String,
    },
    Delete {
        bucket: String,
        key: String,
    },
}

fn main() -> std::result::Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    let secret_key = get_or_create_pk(&cli.private_key_file)?;
    dbg!(hex::encode(secret_key.secret_bytes()));

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Command::Bucket { bucket } => {
            println!("'bucket' was used: {:?}", bucket);
        }
        Command::Set { bucket, key, value } => {
            println!("'set' was used: {:?} {:?} {:?}", bucket, key, value);
        }
        Command::Delete { bucket, key } => {
            println!("'delete' was used: {:?} {:?}", bucket, key);
        }
    }
    Ok(())
}

fn get_or_create_pk(path: &str) -> Result<SecretKey> {
    if !Path::new(path).try_exists()? {
        let secret_key = SecretKey::new(&mut rand::thread_rng());
        let mut f = File::create(path)?;
        let hex = hex::encode(&secret_key.secret_bytes());
        f.write_all(hex.as_bytes())?;
        return Ok(secret_key);
    }
    let contents = std::fs::read_to_string(path)?;
    let parsed = hex::decode(contents)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    Ok(SecretKey::from_slice(&parsed)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?)
}
