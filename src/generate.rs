use std::{collections::HashMap, path::PathBuf};

use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient,
    traits::{
        DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver,
        DefaultTransactionDependencyProvider, SecpCkbRawKeySigner,
    },
    tx_builder::{
        balance_tx_capacity, fill_placeholder_witnesses, transfer::CapacityTransferBuilder,
        CapacityBalancer, TxBuilder,
    },
    unlock::{OmniLockConfig, OmniLockScriptSigner},
    unlock::{OmniLockUnlocker, OmniUnlockMode, ScriptUnlocker},
    Address, HumanCapacity, ScriptId, SECP256K1,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, ScriptHashType, TransactionView},
    packed::{CellDep, CellOutput, OutPoint, Script},
    prelude::*,
    H256,
};
use clap::Args;

use crate::{client::build_omnilock_cell_dep_from_client, config::ConfigContext};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
#[derive(Args)]
pub struct GenTxArgs {
    /// The sender private key (hex string)
    #[clap(long, value_name = "KEY")]
    sender_key: H256,
    /// The receiver address
    #[clap(long, value_name = "ADDRESS")]
    receiver: Address,

    /// The capacity to transfer (unit: CKB, example: 102.43)
    #[clap(long, value_name = "CKB")]
    capacity: HumanCapacity,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct TxInfo {
    tx: json_types::TransactionView,
    omnilock_config: OmniLockConfig,
}

pub fn generate_transfer_tx(args: &GenTxArgs, env: &ConfigContext) -> Result<()> {
    let (tx, omnilock_config) = build_transfer_tx(args, env)?;
    let tx_info = TxInfo {
        tx: json_types::TransactionView::from(tx),
        omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn build_transfer_tx(
    args: &GenTxArgs,
    env: &ConfigContext,
) -> Result<(TransactionView, OmniLockConfig)> {
    let sender_key =
        secp256k1::SecretKey::from_slice(args.sender_key.as_bytes()).with_context(|| {
            format!(
                "fail to parse the send_key: `{0}` as private key",
                args.sender_key
            )
        })?;
    let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &sender_key);
    let mut ckb_client = CkbRpcClient::new(env.ckb_rpc.as_str());
    let cell = build_omnilock_cell_dep_from_client(
        &mut ckb_client,
        &env.omnilock_tx_hash,
        env.omnilock_index,
    )?;
    let omnilock_config = OmniLockConfig::new_pubkey_hash(&pubkey.into());
    // Build CapacityBalancer
    let sender = Script::new_builder()
        .code_hash(cell.type_hash.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(omnilock_config.build_args().pack())
        .build();
    let placeholder_witness = omnilock_config.placeholder_witness(OmniUnlockMode::Normal)?;
    let balancer = CapacityBalancer::new_simple(sender, placeholder_witness, 1000);

    // Build:
    //   * CellDepResolver
    //   * HeaderDepResolver
    //   * CellCollector
    //   * TransactionDependencyProvider
    // let mut ckb_client = CkbRpcClient::new(args.ckb_rpc.as_str());
    let genesis_block = ckb_client.get_block_by_number(0.into())?.unwrap();
    let genesis_block = BlockView::from(genesis_block);
    let mut cell_dep_resolver = DefaultCellDepResolver::from_genesis(&genesis_block)?;
    cell_dep_resolver.insert(cell.script_id, cell.cell_dep, "Omni Lock".to_string());
    let header_dep_resolver = DefaultHeaderDepResolver::new(env.ckb_rpc.as_str());
    let mut cell_collector =
        DefaultCellCollector::new(env.ckb_indexer.as_str(), env.ckb_rpc.as_str());
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(env.ckb_rpc.as_str(), 10);

    // Build base transaction
    let unlockers = build_omnilock_unlockers(Vec::new(), omnilock_config.clone(), cell.type_hash);
    let output = CellOutput::new_builder()
        .lock(Script::from(&args.receiver))
        .capacity(args.capacity.0.pack())
        .build();
    let builder = CapacityTransferBuilder::new(vec![(output, Bytes::default())]);

    let base_tx = builder.build_base(
        &mut cell_collector,
        &cell_dep_resolver,
        &header_dep_resolver,
        &tx_dep_provider,
    )?;

    let secp256k1_data_dep = {
        // pub const SECP256K1_DATA_OUTPUT_LOC: (usize, usize) = (0, 3);
        let tx_hash = genesis_block.transactions()[0].hash();
        let out_point = OutPoint::new(tx_hash, 3u32);
        CellDep::new_builder().out_point(out_point).build()
    };

    let base_tx = base_tx
        .as_advanced_builder()
        .cell_dep(secp256k1_data_dep)
        .build();
    let (tx_filled_witnesses, _) =
        fill_placeholder_witnesses(base_tx, &tx_dep_provider, &unlockers)
            .with_context(|| "try to fill placeholder witnesses".to_string())?;

    let tx = balance_tx_capacity(
        &tx_filled_witnesses,
        &balancer,
        &mut cell_collector,
        &tx_dep_provider,
        &cell_dep_resolver,
        &header_dep_resolver,
    )
    .with_context(|| "try to balance capacity".to_string())?;
    Ok((tx, omnilock_config))
}

fn build_omnilock_unlockers(
    keys: Vec<secp256k1::SecretKey>,
    config: OmniLockConfig,
    omni_lock_type_hash: H256,
) -> HashMap<ScriptId, Box<dyn ScriptUnlocker>> {
    let signer = SecpCkbRawKeySigner::new_with_secret_keys(keys);
    let omnilock_signer =
        OmniLockScriptSigner::new(Box::new(signer), config.clone(), OmniUnlockMode::Normal);
    let omnilock_unlocker = OmniLockUnlocker::new(omnilock_signer, config);
    let omnilock_script_id = ScriptId::new_type(omni_lock_type_hash);
    HashMap::from([(
        omnilock_script_id,
        Box::new(omnilock_unlocker) as Box<dyn ScriptUnlocker>,
    )])
}
