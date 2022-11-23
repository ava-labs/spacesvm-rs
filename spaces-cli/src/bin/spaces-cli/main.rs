use std::error;

use clap::{Parser, Subcommand};
use jsonrpc_core::futures;
use spacesvm::{
    api::{
        client::{claim_tx, delete_tx, get_or_create_pk, set_tx, Client, Uri},
        DecodeTxArgs, IssueTxArgs, ResolveArgs,
    },
    chain::tx::{decoder, unsigned::TransactionData},
};

#[derive(Subcommand, Debug)]
enum Command {
    Claim {
        space: String,
    },
    Set {
        space: String,
        key: String,
        value: String,
    },
    Delete {
        space: String,
        key: String,
    },
    Get {
        space: String,
        key: String,
    },
    Ping {},
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Endpoint for RPC calls.
    #[clap(long)]
    endpoint: String,

    /// Private key file.
    #[clap(long, default_value = ".spacesvm-cli-pk")]
    private_key_file: String,

    /// Which subcommand to call.
    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    let secret_key = get_or_create_pk(&cli.private_key_file)?;
    let uri = cli.endpoint.parse::<Uri>()?;
    let mut client = Client::new(uri);

    if let Command::Get { space, key } = &cli.command {
        let resp = futures::executor::block_on(client.resolve(ResolveArgs {
            space: space.as_bytes().to_vec(),
            key: key.as_bytes().to_vec(),
        }))
        .map_err(|e| e.to_string())?;
        log::debug!("resolve response: {:?}", resp);

        println!("{}", serde_json::to_string(&resp)?);
        return Ok(());
    }

    if let Command::Ping {} = &cli.command {
        let resp = futures::executor::block_on(client.ping()).map_err(|e| e.to_string())?;

        println!("{}", serde_json::to_string(&resp)?);
        return Ok(());
    }

    // decode tx
    let tx_data = command_to_tx(cli.command)?;
    let resp = futures::executor::block_on(client.decode_tx(DecodeTxArgs { tx_data }))
        .map_err(|e| e.to_string())?;

    let typed_data = &resp.typed_data;

    // create signature
    let dh = decoder::hash_structured_data(typed_data)?;
    let sig = secret_key.sign_digest(&dh.as_bytes())?;

    // issue tx
    let resp = futures::executor::block_on(client.issue_tx(IssueTxArgs {
        typed_data: resp.typed_data,
        signature: sig.to_bytes().to_vec(),
    }))
    .map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string(&resp)?);

    Ok(())
}

/// Takes a TX command and returns transaction data.
fn command_to_tx(command: Command) -> std::io::Result<TransactionData> {
    match command {
        Command::Claim { space } => Ok(claim_tx(space)),
        Command::Set { space, key, value } => Ok(set_tx(space, key, value.as_bytes().to_vec())),
        Command::Delete { space, key } => Ok(delete_tx(space, key)),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "not a supported tx",
        )),
    }
}
