use std::collections::BTreeMap;

use crate::{arg_parser::ArgParser, client::build_omnilock_cell_dep, config::ConfigContext};
use ckb_crypto::secp::Pubkey;
use ckb_sdk::{
    constants::SIGHASH_TYPE_HASH,
    unlock::{MultisigConfig, OmniLockConfig},
    util::keccak160,
    Address, NetworkType, SECP256K1,
};
use ckb_types::{core::ScriptHashType, packed::Script, prelude::*, H160, H256};
use clap::{ArgGroup, Args, Subcommand, ValueEnum};

use anyhow::{anyhow, bail, ensure, Result};
use jsonrpc_core::Value;
use serde_json::json;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
#[repr(u8)]
enum OmniLockFlags {
    // administrator mode, flag is 1, affected args:  RC cell type ID, affected field:omni_identity/signature in OmniLockWitnessLock
    //ADMIN = 1,
    // anyone-can-pay mode, flag is 1<<1, affected args: minimum ckb/udt in ACP
    // ACP = 1<<1,
    // time-lock mode, flag is 1<<2, affected args: since for timelock
    // TIMELOCK = 1<<2,
    // supply mode, flag is 1<<3, affected args: type script hash for supply
    // SUPPLY = 1<<3,
    /// open transaction mode.
    OPENTX = 1 << 4,
}
#[derive(Args)]
pub struct ConfigArgs {
    /// The receiver address
    #[clap(long, value_name = "flags", value_enum)]
    flags: Vec<OmniLockFlags>,
}

#[derive(Args)]
pub(crate) struct PubkeyHashArgs {
    /// The receiver address
    #[clap(long, value_name = "ADDRESS")]
    sighash_address: Option<Address>,
    /// The receiver's blake160 hash of a public key, lock-arg
    #[clap(long, value_name = "HASH", value_parser=H160::parse)]
    pubkey_hash: Option<H160>,

    #[clap(flatten)]
    common_args: ConfigArgs,
}

#[derive(Args)]
#[clap(group(
    ArgGroup::new("key")
        .required(true)
        .args(&["ethereum-address", "ethereum-privkey", "ethereum-pubkey"]),
))]
pub(crate) struct EthereumArgs {
    /// The receiver's ethereum address
    #[clap(long, value_name = "ADDRESS", value_parser=H160::parse)]
    ethereum_address: Option<H160>,
    /// The receiver's private key (hex string)
    #[clap(long, value_name = "PRV_KEY", value_parser=H256::parse)]
    ethereum_privkey: Option<H256>,
    /// The receiver's pub key (hex string)
    #[clap(long, value_name = "PUB_KEY")]
    ethereum_pubkey: Option<String>,
    /// Work with "--ethereum-privkey", if this value is set, print the generated public key
    #[clap(
        long,
        value_name = "TO_PRINT_PUBKEY",
        default_value = "false",
        value_parser
    )]
    to_print_pubkey: bool,

    /// Work with "--ethereum-privkey" or "--ethereum-pubkey", this option control if to print the generated ethereum address.
    #[clap(
        long,
        value_name = "TO_PRINT_ADDR",
        default_value = "false",
        value_parser
    )]
    to_print_addr: bool,
    #[clap(flatten)]
    common_args: ConfigArgs,
}

#[derive(Args)]
pub(crate) struct MultiSigArgs {
    /// Require first n signatures of corresponding pubkey
    #[clap(long, value_name = "NUM")]
    require_first_n: u8,

    /// Multisig threshold
    #[clap(long, value_name = "NUM")]
    threshold: u8,

    /// Normal sighash addresses
    #[clap(long, multiple_values = true, value_name = "ADDRESS")]
    sighash_address: Vec<Address>,
    #[clap(flatten)]
    common_args: ConfigArgs,
}

#[derive(Subcommand)]
pub(crate) enum BuildAddress {
    /// The auth content represents the blake160 hash of a secp256k1 public key.
    PubkeyHash(PubkeyHashArgs),
    /// It follows the same unlocking methods used by Ethereum.
    Ethereum(EthereumArgs),
    // /// It follows the same unlocking methods used by EOS.
    // Eos,
    // /// It follows the same unlocking methods used by Tron.
    // Tron,
    // /// It follows the same unlocking methods used by Bitcoin
    // Bitcoin,
    // ///  It follows the same unlocking methods used by Dogecoin.
    // Dogecoin,
    /// It follows the same unlocking method used by CKB MultiSig.
    Multisig(MultiSigArgs),
    // /// The auth content that represents the blake160 hash of a lock script.
    // /// The lock script will check if the current transaction contains an input cell with a matching lock script.
    // /// Otherwise, it would return with an error. It's similar to P2SH in BTC.
    // OwnerLock,
    // /// The auth content that represents the blake160 hash of a preimage.
    // /// The preimage contains exec information that is used to delegate signature verification to another script via exec.
    // Exec,
    // /// The auth content that represents the blake160 hash of a preimage.
    // /// The preimage contains dynamic linking information that is used to delegate signature verification to the dynamic linking script.
    // /// The interface described in Swappable Signature Verification Protocol Spec is used here.
    // Dl,
}

pub(crate) fn build_omnilock_addr(cmds: BuildAddress, env: &ConfigContext) -> Result<()> {
    match cmds {
        BuildAddress::PubkeyHash(args) => {
            build_pubkeyhash_addr(args, env)?;
        }
        BuildAddress::Ethereum(args) => {
            build_ethereum_addr(args, env)?;
        }
        BuildAddress::Multisig(args) => {
            build_multisig_addr(args, env)?;
        }
    };
    Ok(())
}

fn set_omnilock_config_mode(config: &mut OmniLockConfig, args: &ConfigArgs) {
    for v in &args.flags {
        match v {
            OmniLockFlags::OPENTX => config.set_opentx_mode(),
        }
    }
}

fn build_pubkeyhash_addr(args: PubkeyHashArgs, env: &ConfigContext) -> Result<()> {
    let arg = if let Some(pubkey_hash) = args.pubkey_hash {
        pubkey_hash
    } else if let Some(address) = args.sighash_address {
        let to_address_hash_type = address.payload().hash_type();
        let to_address_code_hash: H256 = address
            .payload()
            .code_hash(Some(address.network()))
            .unpack();
        ensure!(
            to_address_hash_type == ScriptHashType::Type
                && to_address_code_hash == ckb_sdk::constants::SIGHASH_TYPE_HASH,
            "The receiver's address must be a sighash address!"
        );
        H160::from_slice(&address.payload().args())?
    } else {
        bail!("The receiver's pubkey hash or address must be provided!");
    };
    let mut config = OmniLockConfig::new_pubkey_hash(arg);
    set_omnilock_config_mode(&mut config, &args.common_args);
    build_addr_with_omnilock_conf(&config, env, BTreeMap::default())
}

fn build_ethereum_addr(args: EthereumArgs, env: &ConfigContext) -> Result<()> {
    let mut extra_json = BTreeMap::new();
    let address = if let Some(address) = args.ethereum_address {
        address
    } else {
        let pubkey = if let Some(str) = args.ethereum_pubkey {
            secp256k1::PublicKey::parse(&str)?
        } else if let Some(receiver) = args.ethereum_privkey {
            let privkey = secp256k1::SecretKey::from_slice(receiver.as_bytes()).unwrap();
            let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &privkey);

            if args.to_print_pubkey {
                extra_json.insert(
                    "ethereum-pubkey".to_owned(),
                    json!(format!("0x{:#x}", pubkey)),
                );
            }
            pubkey
        } else {
            bail!("should provide at least one private key or public key of the receiver");
        };
        let addr = keccak160(Pubkey::from(pubkey).as_ref());
        if args.to_print_addr {
            extra_json.insert(
                "ethereum-address".to_owned(),
                json!(json!(format!("0x{:#x}", addr))),
            );
        }
        addr
    };
    let mut config = OmniLockConfig::new_ethereum(address);

    set_omnilock_config_mode(&mut config, &args.common_args);
    build_addr_with_omnilock_conf(&config, env, extra_json)
}

fn build_multisig_addr(args: MultiSigArgs, env: &ConfigContext) -> Result<()> {
    let multisig_config =
        build_multisig_config(&args.sighash_address, args.require_first_n, args.threshold)?;

    let mut config = OmniLockConfig::new_multisig(multisig_config);
    set_omnilock_config_mode(&mut config, &args.common_args);
    build_addr_with_omnilock_conf(&config, env, BTreeMap::default())
}

fn build_addr_with_omnilock_conf(
    config: &OmniLockConfig,
    env: &ConfigContext,
    extra_json: BTreeMap<String, Value>,
) -> Result<()> {
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
    let mut resp = serde_json::json!({
        "mainnet": Address::new(NetworkType::Mainnet, address_payload.clone(), true).to_string(),
        "testnet": Address::new(NetworkType::Testnet, address_payload.clone(), true).to_string(),
        "lock-arg": format!("0x{}", hex_string(address_payload.args().as_ref())),
        "lock-hash": format!("{:#x}", lock_script.calc_script_hash())
    });

    if !extra_json.is_empty() {
        if let &mut Value::Object(ref mut map) = &mut resp {
            map.extend(extra_json);
        }
    }

    println!("{}", serde_json::to_string_pretty(&resp)?);
    Ok(())
}

pub fn build_multisig_config(
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
