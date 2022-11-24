use std::{collections::HashMap, path::PathBuf};

use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    rpc::CkbRpcClient,
    traits::{
        DefaultCellCollector, DefaultCellDepResolver, DefaultHeaderDepResolver,
        DefaultTransactionDependencyProvider, SecpCkbRawKeySigner,
    },
    tx_builder::{
        balance_tx_capacity, fill_placeholder_witnesses, omni_lock::OmniLockTransferBuilder,
        transfer::CapacityTransferBuilder, CapacityBalancer, TxBuilder,
    },
    unlock::{opentx::OpentxWitness, OmniLockConfig, OmniLockScriptSigner},
    unlock::{OmniLockUnlocker, OmniUnlockMode, ScriptUnlocker},
    Address, HumanCapacity, ScriptId,
};
use ckb_types::{
    bytes::Bytes,
    core::{BlockView, ScriptHashType, TransactionView},
    packed::{CellDep, CellOutput, OutPoint, Script},
    prelude::*,
    H160, H256,
};
use clap::{Args, Subcommand};

use crate::{
    arg_parser::{ArgParser, PrivkeyWrapper},
    build_addr::build_multisig_config,
    client::build_omnilock_cell_dep_from_client,
    config::ConfigContext,
    signer::CommonSigner,
    txinfo::TxInfo,
};
use anyhow::{Context, Result};

use rand::Rng;
use std::fs;
#[derive(Args)]
pub struct GeneratePubkeyHashArgs {
    /// The sender's pubkey hash, lock-arg
    #[clap(long, value_name = "KEY", value_parser=H160::parse)]
    pubkey_hash: H160,

    #[clap(flatten)]
    common_args: CommonArgs,
}

#[derive(Args)]
pub struct CommonArgs {
    /// The receiver address
    #[clap(long, value_name = "ADDRESS")]
    receiver: Address,

    /// The capacity to transfer (unit: CKB, example: 102.43)
    #[clap(long, value_name = "CKB")]
    capacity: HumanCapacity,

    /// The capacity to transfer (unit: CKB, example: 102.43), according receiver is open, so it will be an open transaction.
    #[clap(long, value_name = "CKB")]
    open_capacity: Option<HumanCapacity>,
    /// The fee_rate
    #[clap(long, value_name = "NUMBER", default_value = "1000")]
    fee_rate: u64,
    /// salt value for open transaction input command list, if not specified, a random salt value will be generated.
    #[clap(long, value_name = "NUMBER")]
    open_salt: Option<u32>,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

#[derive(Args)]
pub struct GenerateMultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    #[clap(long, value_name = "NUM")]
    require_first_n: u8,

    /// Multisig threshold
    #[clap(long, value_name = "NUM")]
    threshold: u8,

    /// Normal sighash address
    #[clap(long, value_name = "ADDRESS", multiple_values = true)]
    sighash_address: Vec<Address>,

    #[clap(flatten)]
    common_args: CommonArgs,
}

#[derive(Args)]
pub struct GenerateEthereumArgs {
    /// The sender's ethereum address
    #[clap(long, value_name = "ADDRESS", value_parser=H160::parse)]
    sender_address: H160,

    #[clap(flatten)]
    common_args: CommonArgs,
}
#[derive(Subcommand)]
pub enum GenerateTx {
    /// to generate a transaction from pubkey hash omnilock cell
    PubkeyHash(GeneratePubkeyHashArgs),
    /// to generate a transaction from ethereum omnilock cell
    Ethereum(GenerateEthereumArgs),
    /// to generate a transaction from multisig omnilock cell
    Multisig(GenerateMultiSigArgs),
}

pub fn generate_transfer_tx(cmds: &GenerateTx, env: &ConfigContext) -> Result<()> {
    let (tx, omnilock_config, tx_file) = match cmds {
        GenerateTx::PubkeyHash(args) => build_pubkeyhash_transfer_tx(args, env)?,
        GenerateTx::Ethereum(args) => build_ethereum_transfer_tx(args, env)?,
        GenerateTx::Multisig(args) => build_multisig_transfer_tx(args, env)?,
    };

    let tx_info = TxInfo {
        transaction: json_types::TransactionView::from(tx).inner,
        omnilock_config,
    };
    fs::write(tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn build_pubkeyhash_transfer_tx(
    args: &GeneratePubkeyHashArgs,
    env: &ConfigContext,
) -> Result<(TransactionView, OmniLockConfig, PathBuf)> {
    let omnilock_config = OmniLockConfig::new_pubkey_hash(args.pubkey_hash.clone());
    build_transfer_tx_(&args.common_args, env, omnilock_config)
}

fn build_transfer_tx_(
    args: &CommonArgs,
    env: &ConfigContext,
    mut omnilock_config: OmniLockConfig,
) -> Result<(TransactionView, OmniLockConfig, PathBuf)> {
    let mut ckb_client = CkbRpcClient::new(env.ckb_rpc.as_str());
    let cell = build_omnilock_cell_dep_from_client(
        &mut ckb_client,
        &env.omnilock_tx_hash,
        env.omnilock_index,
    )?;
    if args.open_capacity.is_some() {
        omnilock_config.set_opentx_mode();
    }
    // Build CapacityBalancer
    let sender = Script::new_builder()
        .code_hash(cell.type_hash.pack())
        .hash_type(ScriptHashType::Type.into())
        .args(omnilock_config.build_args().pack())
        .build();
    let placeholder_witness = omnilock_config.placeholder_witness(OmniUnlockMode::Normal)?;
    let balancer = CapacityBalancer::new_simple(sender.clone(), placeholder_witness, args.fee_rate);

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
    let mut cell_collector = DefaultCellCollector::new(env.ckb_rpc.as_str());
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(env.ckb_rpc.as_str(), 10);

    // Build base transaction
    let unlockers = build_omnilock_unlockers(Vec::new(), omnilock_config.clone(), cell.type_hash);
    let output = CellOutput::new_builder()
        .lock(Script::from(&args.receiver))
        .capacity(args.capacity.0.pack())
        .build();
    let builder: Box<dyn TxBuilder> = if args.open_capacity.is_none() {
        Box::new(CapacityTransferBuilder::new(vec![(
            output,
            Bytes::default(),
        )]))
    } else {
        Box::new(OmniLockTransferBuilder::new_open(
            args.open_capacity.unwrap(),
            vec![(output, Bytes::default())],
            omnilock_config.clone(),
            None,
        ))
    };

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

    let mut tx = balance_tx_capacity(
        &tx_filled_witnesses,
        &balancer,
        &mut cell_collector,
        &tx_dep_provider,
        &cell_dep_resolver,
        &header_dep_resolver,
    )
    .with_context(|| "try to balance capacity".to_string())?;
    if omnilock_config.is_opentx_mode() {
        tx = OmniLockTransferBuilder::remove_open_out(tx);
        let salt = if args.open_salt.is_some() {
            args.open_salt
        } else {
            let mut rng = rand::thread_rng();
            Some(rng.gen())
        };
        let wit = OpentxWitness::new_sig_all_relative(&tx, salt).unwrap();
        omnilock_config.set_opentx_input(wit);
        tx = OmniLockTransferBuilder::update_opentx_witness(
            tx,
            &omnilock_config,
            OmniUnlockMode::Normal,
            &tx_dep_provider,
            &sender,
        )?;
    }
    Ok((tx, omnilock_config, args.tx_file.clone()))
}

fn build_ethereum_transfer_tx(
    args: &GenerateEthereumArgs,
    env: &ConfigContext,
) -> Result<(TransactionView, OmniLockConfig, PathBuf)> {
    let omnilock_config = OmniLockConfig::new_ethereum(args.sender_address.clone());
    build_transfer_tx_(&args.common_args, env, omnilock_config)
}

fn build_multisig_transfer_tx(
    args: &GenerateMultiSigArgs,
    env: &ConfigContext,
) -> Result<(TransactionView, OmniLockConfig, PathBuf)> {
    let multisig_config =
        build_multisig_config(&args.sighash_address, args.require_first_n, args.threshold)?;

    let omnilock_config = OmniLockConfig::new_multisig(multisig_config);
    build_transfer_tx_(&args.common_args, env, omnilock_config)
}

pub fn build_omnilock_unlockers(
    keys: Vec<PrivkeyWrapper>,
    config: OmniLockConfig,
    omni_lock_type_hash: H256,
) -> HashMap<ScriptId, Box<dyn ScriptUnlocker>> {
    let keys: Vec<secp256k1::SecretKey> = keys.iter().map(|k| k.0).collect();
    let signer = if config.is_ethereum() {
        SecpCkbRawKeySigner::new_with_ethereum_secret_keys(keys)
    } else {
        SecpCkbRawKeySigner::new_with_secret_keys(keys)
    };
    let omnilock_signer = Box::new(CommonSigner::new(vec![Box::new(signer)]));
    let omnilock_signer =
        OmniLockScriptSigner::new(omnilock_signer, config.clone(), OmniUnlockMode::Normal);
    let omnilock_unlocker = OmniLockUnlocker::new(omnilock_signer, config);
    let omnilock_script_id = ScriptId::new_type(omni_lock_type_hash);
    HashMap::from([(
        omnilock_script_id,
        Box::new(omnilock_unlocker) as Box<dyn ScriptUnlocker>,
    )])
}
