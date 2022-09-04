use std::str::FromStr;

use crate::{client::build_omnilock_cell_dep, config::ConfigContext};
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH,
    unlock::{MultisigConfig, OmniLockConfig},
    Address, NetworkType, SECP256K1,
};
use ckb_types::{core::ScriptHashType, packed::Script, prelude::*, H160, H256};
use clap::{Args, Subcommand};

use anyhow::{anyhow, bail, ensure, Result};
#[derive(Args)]
pub(crate) struct MultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    #[clap(long, value_name = "NUM")]
    require_first_n: u8,

    /// Multisig threshold
    #[clap(long, value_name = "NUM")]
    threshold: u8,

    /// Normal sighash addresses
    #[clap(long,  multiple_values = true, value_name = "ADDRESS")]
    sighash_address: Vec<Address>,
}

#[derive(Subcommand)]
pub(crate) enum BuildAddress {
    /// The auth content represents the blake160 hash of a secp256k1 public key.
    /// The lock script will perform secp256k1 signature verification, the same as the SECP256K1/blake160 lock.
    PubkeyHash {
        /// The receiver address
        #[clap(long, value_name = "ADDRESS")]
        receiver: Address,
    },
    /// It follows the same unlocking methods used by Ethereum.
    Ethereum {
        /// The receiver's private key (hex string)
        #[clap(long, value_name = "PRV_KEY")]
        receiver_privkey: Option<H256>,
        /// The receiver's pub key (hex string)
        #[clap(long, value_name = "PUB_KEY")]
        receiver_pubkey: Option<String>,
    },
    /// It follows the same unlocking methods used by EOS.
    Eos,
    /// It follows the same unlocking methods used by Tron.
    Tron,
    /// It follows the same unlocking methods used by Bitcoin
    Bitcoin,
    ///  It follows the same unlocking methods used by Dogecoin.
    Dogecoin,
    /// It follows the same unlocking method used by CKB MultiSig.
    Multisig(MultiSigArgs),

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

pub(crate) fn build_omnilock_addr(cmds: &BuildAddress, env: &ConfigContext) -> Result<()> {
    match cmds {
        BuildAddress::PubkeyHash { receiver } => {
            build_pubkeyhash_addr(receiver, env)?;
        }
        BuildAddress::Ethereum {
            receiver_privkey,
            receiver_pubkey,
        } => {
            build_ethereum_addr(receiver_privkey, receiver_pubkey, env)?;
        }
        BuildAddress::Multisig(args) => {
            build_multisig_addr(args, env)?;
        }
        _ => unreachable!("the action is not supported yet"),
    };
    Ok(())
}

fn build_pubkeyhash_addr(receiver: &Address, env: &ConfigContext) -> Result<()> {
    let arg = H160::from_slice(&receiver.payload().args()).unwrap();
    let config = OmniLockConfig::new_pubkey_hash_with_lockarg(arg);

    build_addr_with_omnilock_conf(&config, env)
}

fn build_ethereum_addr(
    receiver_privkey: &Option<H256>,
    receiver_pubkey: &Option<String>,
    env: &ConfigContext,
) -> Result<()> {
    let pubkey = if let Some(str) = receiver_pubkey {
        secp256k1::PublicKey::from_str(str)?
    } else if let Some(receiver) = receiver_privkey {
        let privkey = secp256k1::SecretKey::from_slice(receiver.as_bytes()).unwrap();
        secp256k1::PublicKey::from_secret_key(&SECP256K1, &privkey)
    } else {
        bail!("should provide at least one private key or public key of the receiver");
    };
    println!("pubkey:{:?}", hex_string(&pubkey.serialize()));
    println!("pubkey:{:?}", hex_string(&pubkey.serialize_uncompressed()));
    let config = OmniLockConfig::new_ethereum(&pubkey.into());

    build_addr_with_omnilock_conf(&config, env)
}

fn build_multisig_addr(args: &MultiSigArgs, env: &ConfigContext) -> Result<()> {
    let multisig_config =
        build_multisig_config(&args.sighash_address, args.require_first_n, args.threshold)?;

    let config = OmniLockConfig::new_multisig(multisig_config);
    build_addr_with_omnilock_conf(&config, env)
}

fn build_addr_with_omnilock_conf(config: &OmniLockConfig, env: &ConfigContext) -> Result<()> {
    let cell = build_omnilock_cell_dep(
        env.ckb_rpc.as_str(),
        &env.omnilock_tx_hash,
        env.omnilock_index,
    )?;

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

    MultisigConfig::new_with(sighash_addresses, require_first_n, threshold)
        .map_err(|e| anyhow!(e.to_string()))
}
