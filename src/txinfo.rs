use anyhow::Result;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::unlock::OmniLockConfig;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct TxInfo {
    pub transaction: json_types::Transaction,
    pub omnilock_config: OmniLockConfig,
}

#[derive(Args)]
pub(crate) struct ExportTxArgs {
    /// The input omnilock transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    from_tx_file: PathBuf,
    /// The output ckb-cli transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    to_tx_file: PathBuf,
}
#[derive(Serialize)]
struct CkbCliTxInfo {
    transaction: json_types::Transaction,
    multisig_configs: HashMap<u8, u8>,
    signatures: HashMap<u8, u8>,
}

pub(crate) fn handle_export_tx_info(args: ExportTxArgs) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.from_tx_file)?)?;

    let ckb_tx_info = CkbCliTxInfo {
        transaction: tx_info.transaction,
        multisig_configs: HashMap::new(),
        signatures: HashMap::new(),
    };

    fs::write(args.to_tx_file, serde_json::to_string_pretty(&ckb_tx_info)?)?;
    Ok(())
}
