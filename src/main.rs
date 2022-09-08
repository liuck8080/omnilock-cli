mod build_addr;
mod client;
mod config;
mod generate;
mod keystore;
mod sign;
mod txinfo;

use ckb_jsonrpc_types as json_types;
use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use build_addr::BuildAddress;
use ckb_sdk::CkbRpcClient;
use ckb_types::H256;
use clap::{Args, Parser, Subcommand};
use config::{handle_config_cmds, ConfigCmds, ConfigContext};
use generate::{generate_transfer_tx, GenTxArgs};
use sign::{sign_tx, SignTxArgs};

use crate::{build_addr::build_omnilock_addr, txinfo::TxInfo};

#[derive(Args)]
struct EnvArgs {
    /// omnilock script deploy transaction hash
    #[clap(long, value_name = "H256")]
    omnilock_tx_hash: H256,

    /// cell index of omnilock script deploy transaction's outputs
    #[clap(long, value_name = "NUMBER")]
    omnilock_index: usize,

    /// CKB rpc url
    #[clap(long, value_name = "URL", default_value = "http://127.0.0.1:8114")]
    ckb_rpc: String,

    /// CKB indexer rpc url
    #[clap(long, value_name = "URL", default_value = "http://127.0.0.1:8116")]
    ckb_indexer: String,

    /// omnilock config file path
    #[clap(long, value_name = "FILE", default_value = "~")]
    env_config_file: String,
}

#[derive(Subcommand)]
enum Commands {
    /// build omni lock address
    #[clap(subcommand)]
    BuildAddress(BuildAddress),
    /// generate a transaction not signed yet
    GenerateTx(GenTxArgs),
    /// Sign the transaction
    Sign(SignTxArgs),
    /// Send the transaction
    Send {
        /// The transaction info file (.json)
        #[clap(long, value_name = "PATH")]
        tx_file: PathBuf,
    },
    /// generate a template configuration for later modification.
    #[clap(subcommand)]
    Config(ConfigCmds),
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    /// omnilock config file path
    #[clap(
        short,
        long,
        value_parser,
        value_name = "FILE",
        default_value = "~/.omnilock.yaml"
    )]
    config: String,
    #[clap(subcommand)]
    command: Commands,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildAddress(cmds) => {
            let config = ConfigContext::parse(&cli.config)?;
            build_omnilock_addr(&cmds, &config)?;
        }
        Commands::GenerateTx(args) => {
            let config = ConfigContext::parse(&cli.config)?;
            generate_transfer_tx(&args, &config)?;
        }
        Commands::Sign(args) => {
            let config = ConfigContext::parse(&cli.config)?;
            sign_tx(&args, &config)?;
        }
        Commands::Send { tx_file } => {
            let config = ConfigContext::parse(&cli.config)?;
            send_tx(&tx_file, &config)?;
        }
        Commands::Config(cmds) => {
            handle_config_cmds(&cmds, &cli.config)?;
        }
    }
    Ok(())
}

fn send_tx(tx_file: &PathBuf, env: &ConfigContext) -> Result<()> {
    // Send transaction
    let read = fs::read(&tx_file)
        .with_context(|| format!("try to read file {}", tx_file.to_string_lossy()))?;
    let tx_info: TxInfo = serde_json::from_slice(&read)
        .with_context(|| format!("try to parse file {}", tx_file.to_string_lossy()))?;
    // println!("> tx: {}", serde_json::to_string_pretty(&tx_info.transaction)?);
    let outputs_validator = Some(json_types::OutputsValidator::Passthrough);
    let tx_hash = CkbRpcClient::new(env.ckb_rpc.as_str())
        .send_transaction(tx_info.transaction, outputs_validator)
        .expect("send transaction");
    println!(">>> tx {} sent! <<<", tx_hash);
    Ok(())
}
