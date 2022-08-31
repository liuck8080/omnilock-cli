mod build_addr;
mod config;

use anyhow::Result;
use build_addr::BuildAddress;
use ckb_types::H256;
use clap::{Args, Parser, Subcommand};
use config::ConfigContext;

use crate::build_addr::build_omnilock_addr;

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
    /// generate a template configuration for later modification.
    InitConfig,
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
        Commands::InitConfig => {
            ConfigContext::write_template(&cli.config).map(|_| {
                println!(
                    "The template file {} generated, please fill it with the correct content.",
                    &cli.config
                );
            })?;
        }
    }
    Ok(())
}
