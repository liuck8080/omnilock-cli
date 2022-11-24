use crate::{config::ConfigContext, txinfo::TxInfo, arg_parser::ArgParser};
use anyhow::Result;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient, traits::DefaultCellDepResolver, Address, HumanCapacity, ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, Capacity, TransactionView},
    packed::{Byte32, CellOutput, OutPoint, Script, Transaction},
    prelude::*,
    H256,
};
use clap::Args;
use std::{fs, path::PathBuf};

#[derive(Args)]
pub struct AddInputArgs {
    /// omnilock script deploy transaction hash
    #[clap(long, value_name = "H256", value_parser=H256::parse)]
    tx_hash: H256,

    /// cell index of omnilock script deploy transaction's outputs
    #[clap(long, value_name = "NUMBER")]
    index: usize,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

#[derive(Args)]
pub struct AddOutputArgs {
    /// The receiver's address
    #[clap(long, value_name = "ADDRESS")]
    to_address: Address,
    /// The capacity to transfer (unit: CKB, example: 102.43)
    #[clap(long, value_name = "CKB")]
    capacity: HumanCapacity,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

fn add_live_cell(
    args: &AddInputArgs,
    tx: TransactionView,
    env: &ConfigContext,
) -> Result<TransactionView> {
    let mut ckb_client = CkbRpcClient::new(env.ckb_rpc.as_str());
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: args.tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(args.index as u32),
    };
    let cell_with_status = ckb_client.get_live_cell(out_point_json, false)?;
    let input_outpoint = OutPoint::new(
        Byte32::from_slice(args.tx_hash.as_bytes())?,
        args.index as u32,
    );
    // since value should be provided in args
    let input = ckb_types::packed::CellInput::new(input_outpoint, 0);
    let cell_dep_resolver = {
        let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
        DefaultCellDepResolver::from_genesis(&BlockView::from(genesis_block))?
    };
    let code_hash = cell_with_status.cell.unwrap().output.lock.code_hash;
    let script_id = ScriptId::new_type(code_hash);
    let dep = cell_dep_resolver
        .get(&script_id)
        .as_ref()
        .unwrap()
        .0
        .clone();

    Ok(tx.as_advanced_builder().input(input).cell_dep(dep).build())
}

pub fn handle_add_input(args: &AddInputArgs, env: &ConfigContext) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    // println!("> tx: {}", serde_json::to_string_pretty(&tx_info.tx)?);
    let tx = Transaction::from(tx_info.transaction).into_view();
    let tx = add_live_cell(args, tx, env)?;
    let tx_info = TxInfo {
        transaction: json_types::TransactionView::from(tx).inner,
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

pub fn handle_add_output(args: &AddOutputArgs) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    // println!("> tx: {}", serde_json::to_string_pretty(&tx_info.tx)?);
    let tx = Transaction::from(tx_info.transaction).into_view();
    let lock_script = Script::from(args.to_address.payload());
    let output = CellOutput::new_builder()
        .capacity(Capacity::shannons(args.capacity.0).pack())
        .lock(lock_script)
        .build();
    let tx = tx
        .as_advanced_builder()
        .output(output)
        .output_data(Bytes::default().pack())
        .build();
    let tx_info = TxInfo {
        transaction: json_types::TransactionView::from(tx).inner,
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}
