mod add;
mod arg_parser;

mod build_addr;
mod client;
mod config;
mod generate;
mod keystore;
mod sign;
mod signer;
mod txinfo;
mod util;

use ckb_jsonrpc_types as json_types;
use std::{fs, path::PathBuf};
use txinfo::{handle_export_tx_info, ExportTxArgs};

use anyhow::{Context, Result};
use build_addr::BuildAddress;
use ckb_sdk::CkbRpcClient;
use clap::{Parser, Subcommand};
use config::{handle_config_cmds, ConfigCmds, ConfigContext};
use generate::{generate_transfer_tx, GenerateTx};
use sign::{sign_tx, SignCmd};

use crate::{
    add::{handle_add_input, handle_add_output, AddInputArgs, AddOutputArgs},
    build_addr::build_omnilock_addr,
    txinfo::TxInfo,
};

#[derive(Subcommand)]
enum Commands {
    /// Add input
    AddInput(AddInputArgs),
    /// Add output
    AddOutput(AddOutputArgs),
    /// build omni lock address
    #[clap(subcommand)]
    BuildAddress(BuildAddress),
    /// Transform an omnilock tx info file into a ckb-cli compatible format
    ExportTx(ExportTxArgs),
    /// generate a transaction not signed yet
    #[clap(subcommand)]
    GenerateTx(GenerateTx),
    /// Sign the transaction
    #[clap(subcommand)]
    Sign(SignCmd),
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
        Commands::AddInput(args) => {
            let config = ConfigContext::parse(&cli.config)?;
            handle_add_input(&args, &config)?;
        }
        Commands::AddOutput(args) => {
            handle_add_output(&args)?;
        }
        Commands::BuildAddress(cmds) => {
            let config = ConfigContext::parse(&cli.config)?;
            build_omnilock_addr(cmds, &config)?;
        }
        Commands::ExportTx(cmds) => {
            handle_export_tx_info(cmds)?;
        }
        Commands::GenerateTx(cmds) => {
            let config = ConfigContext::parse(&cli.config)?;
            generate_transfer_tx(&cmds, &config)?;
        }
        Commands::Sign(cmds) => {
            let config = ConfigContext::parse(&cli.config)?;
            sign_tx(&cmds, &config)?;
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
