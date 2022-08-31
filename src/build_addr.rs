use crate::config::ConfigContext;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH,
    unlock::{MultisigConfig, OmniLockConfig},
    Address, CkbRpcClient, NetworkType, ScriptId,
};
use ckb_types::{
    core::ScriptHashType,
    packed::{Byte32, CellDep, OutPoint, Script},
    prelude::*,
    H160, H256,
};
use clap::{Args, Subcommand};

use anyhow::{anyhow, bail, ensure, Result};
#[derive(Args)]
pub(crate) struct BuildOmniLockAddrMultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    #[clap(long, value_name = "NUM")]
    require_first_n: u8,

    /// Multisig threshold
    #[clap(long, value_name = "NUM")]
    threshold: u8,

    /// Normal sighash addresses
    #[clap(long, value_name = "ADDRESS")]
    sighash_address: Vec<Address>,
}

#[derive(Subcommand)]
pub(crate) enum BuildAddress {
    /// The auth content represents the blake160 hash of a secp256k1 public key.
    /// The lock script will perform secp256k1 signature verification, the same as the SECP256K1/blake160 lock.
    PubkeyHash,
    /// It follows the same unlocking methods used by Ethereum.
    Ethereum,
    /// It follows the same unlocking methods used by EOS.
    Eos,
    /// It follows the same unlocking methods used by Tron.
    Tron,
    /// It follows the same unlocking methods used by Bitcoin
    Bitcoin,
    ///  It follows the same unlocking methods used by Dogecoin.
    Dogecoin,
    /// It follows the same unlocking method used by CKB MultiSig.
    Multisig(BuildOmniLockAddrMultiSigArgs),

    /// The auth content that represents the blake160 hash of a lock script.
    /// The lock script will check if the current transaction contains an input cell with a matching lock script.
    /// Otherwise, it would return with an error. It's similar to P2SH in BTC.
    OwnerLock,
    /// The auth content that represents the blake160 hash of a preimage.
    /// The preimage contains exec information that is used to delegate signature verification to another script via exec.
    Exec,
    /// The auth content that represents the blake160 hash of a preimage.
    /// The preimage contains dynamic linking information that is used to delegate signature verification to the dynamic linking script.
    /// The interface described in Swappable Signature Verification Protocol Spec is used here.
    Dl,
}

pub(crate) fn build_omnilock_addr(cmds: &BuildAddress, conf: &ConfigContext) -> Result<()> {
    match cmds {
        BuildAddress::Multisig(args) => {
            build_multisig_addr(args, conf)?;
        }
        _ => println!("the action is not supported yet"),
    };
    Ok(())
}

fn build_multisig_addr(args: &BuildOmniLockAddrMultiSigArgs, conf: &ConfigContext) -> Result<()> {
    let mut ckb_client = CkbRpcClient::new(conf.ckb_rpc.as_str());
    let cell =
        build_omnilock_cell_dep(&mut ckb_client, &conf.omnilock_tx_hash, conf.omnilock_index)?;

    let multisig_config =
        build_multisig_config(&args.sighash_address, args.require_first_n, args.threshold)?;

    let config = OmniLockConfig::new_multisig(multisig_config);
    let address_payload = {
        let args = config.build_args();
        ckb_sdk::AddressPayload::new_full(ScriptHashType::Type, cell.type_hash.pack(), args)
    };
    let lock_script = Script::from(&address_payload);
    let resp = serde_json::json!({
        "mainnet": Address::new(NetworkType::Mainnet, address_payload.clone(), true).to_string(),
        "testnet": Address::new(NetworkType::Testnet, address_payload.clone(), true).to_string(),
        "lock-arg": format!("0x{}", hex_string(address_payload.args().as_ref())),
        "lock-hash": format!("{:#x}", lock_script.calc_script_hash())
    });
    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

struct OmniLockInfo {
    type_hash: H256,
    script_id: ScriptId,
    cell_dep: CellDep,
}

fn build_omnilock_cell_dep(
    ckb_client: &mut CkbRpcClient,
    tx_hash: &H256,
    index: u32,
) -> Result<OmniLockInfo> {
    let out_point_json = ckb_jsonrpc_types::OutPoint {
        tx_hash: tx_hash.clone(),
        index: ckb_jsonrpc_types::Uint32::from(index as u32),
    };
    let cell_status = ckb_client.get_live_cell(out_point_json, false)?;
    let script = Script::from(cell_status.cell.unwrap().output.type_.unwrap());

    let type_hash = script.calc_script_hash();
    let out_point = OutPoint::new(Byte32::from_slice(tx_hash.as_bytes())?, index as u32);

    let cell_dep = CellDep::new_builder().out_point(out_point).build();
    Ok(OmniLockInfo {
        type_hash: H256::from_slice(type_hash.as_slice())?,
        script_id: ScriptId::new_type(type_hash.unpack()),
        cell_dep,
    })
}

fn build_multisig_config(
    sighash_address: &[Address],
    require_first_n: u8,
    threshold: u8,
) -> Result<MultisigConfig> {
    ensure!(
        !sighash_address.is_empty(),
        "Must have at least one sighash_address"
    );
    let mut sighash_addresses = Vec::with_capacity(sighash_address.len());
    for addr in sighash_address {
        let lock_args = addr.payload().args();
        if addr.payload().code_hash(None).as_slice() != SIGHASH_TYPE_HASH.as_bytes()
            || addr.payload().hash_type() != ScriptHashType::Type
            || lock_args.len() != 20
        {
            bail!("sighash_address {} is not a valid sighash address", addr);
        }
        sighash_addresses.push(H160::from_slice(lock_args.as_ref()).unwrap());
    }

    Ok(
        MultisigConfig::new_with(sighash_addresses, require_first_n, threshold)
            .map_err(|e| anyhow!(e.to_string()))?,
    )
}
