mod config;
use std::path::PathBuf;

use ckb_sdk::Address;
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, ScriptHashType, TransactionView},
    packed::{Byte32, CellDep, CellOutput, OutPoint, Script, Transaction, WitnessArgs},
    prelude::*,
    H160, H256,
};
use clap::{Args, Parser, Subcommand};
use config::ConfigContext;

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

#[derive(Args)]
struct BuildOmniLockAddrMultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    #[clap(long, value_name = "NUM")]
    require_first_n: u8,

    /// Multisig threshold
    #[clap(long, value_name = "NUM")]
    threshold: u8,

    /// Normal sighash address
    #[clap(long, value_name = "ADDRESS")]
    sighash_address: Vec<Address>,
}

#[derive(Subcommand)]
enum Commands {
    /// build omni lock address
    Build(BuildOmniLockAddrMultiSigArgs),
    /// generate a template configuration for later modification.
    InitConfig,
    /// say hello
    Hi,
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

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build(_args) => {
            println!("build!");
        }
        Commands::InitConfig => {
            match ConfigContext::write_template(&cli.config) {
                Ok(_) => println!("The template file {} generated, please fill it with the correct content.", &cli.config),
                Err(e) => println!("Fail to generate the template file {}, error: {}", cli.config, e)
            }
        }
        Commands::Hi => {
            let config = ConfigContext::parse(&cli.config).unwrap();
            println!("Hello, world!");
        }
    }
}
