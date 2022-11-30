use ckb_crypto::secp::Pubkey;
use ckb_hash::blake2b_256;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    traits::DefaultTransactionDependencyProvider,
    tx_builder::unlock_tx,
    types::omni_lock::OmniLockWitnessLock,
    unlock::{OmniLockConfig, OmniUnlockMode},
    util::keccak160,
    ScriptGroup, SECP256K1,
};
use ckb_types::{
    core::TransactionView,
    molecule::hex_string,
    packed::{Transaction, WitnessArgs},
    prelude::*,
    H160,
};
use clap::{Args, Subcommand};
use rpassword::prompt_password_stdout;
use std::fs;
use std::path::PathBuf;

use crate::{
    arg_parser::{ArgParser, PrivkeyArgParser, PrivkeyWrapper},
    client::build_omnilock_cell_dep,
    config::ConfigContext,
    generate::build_omnilock_unlockers,
    keystore::CkbKeyStore,
    txinfo::TxInfo,
};
use anyhow::{bail, Result};

#[derive(Args)]
pub struct SignTxPubkeyHashArgs {
    /// The sender private key (hex string)
    #[clap(long, value_name = "PRIV_KEY", value_parser=PrivkeyArgParser::parse)]
    pub sender_key: Option<PrivkeyWrapper>,
    /// the unlock account
    #[clap(long, value_name = "ACCOUNT", value_parser=H160::parse)]
    pub from_account: Option<H160>,
    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    pub tx_file: PathBuf,
}

#[derive(Args)]
pub struct EthereumArgs {
    /// The sender private key (hex string)
    #[clap(long, value_name = "KEY", value_parser=PrivkeyArgParser::parse)]
    sender_key: PrivkeyWrapper,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

#[derive(Args)]
pub struct SignTxMultisigArgs {
    /// The sender private key (hex string)
    #[clap(long, value_name = "KEY", multiple_values = true, value_parser=PrivkeyArgParser::parse)]
    sender_key: Vec<PrivkeyWrapper>,

    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    tx_file: PathBuf,
}

#[derive(Subcommand)]
pub enum SignCmd {
    /// to sign a transaction from pubkey hash omnilock cell
    PubkeyHash(SignTxPubkeyHashArgs),
    /// to sign a transaction from ethereum omnilock cell
    Ethereum(EthereumArgs),
    /// to sign a transaction from multisig omnilock cell
    Multisig(SignTxMultisigArgs),
}

pub fn sign_tx(cmds: &SignCmd, env: &ConfigContext) -> Result<()> {
    match cmds {
        SignCmd::PubkeyHash(args) => sign_pubkey_hash_tx(args, env),
        SignCmd::Ethereum(args) => sign_ethereum_tx(args, env),
        SignCmd::Multisig(args) => sign_multisig_tx(args, env),
    }
}

fn sign_pubkey_hash_tx(args: &SignTxPubkeyHashArgs, env: &ConfigContext) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    let tx = Transaction::from(tx_info.transaction).into_view();

    let key = if let Some(sender_key) = &args.sender_key {
        sender_key.clone()
    } else if let Some(from_account) = args.from_account.as_ref() {
        let prompt = "Password";
        let pass = prompt_password_stdout(format!("{}: ", prompt).as_str())?;

        CkbKeyStore::load_default()?.export_priv_key(from_account, pass.as_bytes())?
    } else {
        bail!("must provide one of sender_key(private key) or an account!");
    };
    let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &key);
    let hash160 = &blake2b_256(&pubkey.serialize()[..])[0..20];
    if tx_info.omnilock_config.id().auth_content().as_bytes() != hash160 {
        bail!(
            "can not find hash {} in omnilock config",
            hex_string(hash160)
        );
    }
    let (tx, still_locked_groups) = sign_tx_(tx, &tx_info.omnilock_config, vec![key], env)?;
    let witness_args = WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
    let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
    if lock_field != tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)?
        && still_locked_groups.is_empty()
    {
        println!("> transaction signed!");
    } else {
        bail!("Failed to sign the transaction!");
    }
    let tx_info = TxInfo {
        transaction: json_types::Transaction::from(tx.data()),
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn sign_ethereum_tx(args: &EthereumArgs, env: &ConfigContext) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    let tx = Transaction::from(tx_info.transaction).into_view();
    let pubkey = secp256k1::PublicKey::from_secret_key(&SECP256K1, &args.sender_key);
    let pubkey = Pubkey::from(pubkey);
    let hash160 = keccak160(pubkey.as_ref());
    if tx_info.omnilock_config.id().auth_content().as_bytes() != hash160.as_bytes() {
        bail!("can not find hash {:#x} in omnilock config", hash160);
    }
    let (tx, still_locked_groups) = sign_tx_(
        tx,
        &tx_info.omnilock_config,
        vec![args.sender_key.clone()],
        env,
    )?;
    let witness_args = WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
    let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
    if lock_field != tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)?
        && still_locked_groups.is_empty()
    {
        println!("> transaction signed!");
    } else {
        bail!("Failed to sign the transaction!");
    }
    let tx_info = TxInfo {
        transaction: json_types::Transaction::from(tx.data()),
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn sign_multisig_tx(args: &SignTxMultisigArgs, env: &ConfigContext) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    let tx = Transaction::from(tx_info.transaction).into_view();

    let previous_lock_field = {
        let witness_args =
            WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
        witness_args.lock().to_opt().unwrap().raw_data()
    };
    let (tx, still_locked_groups) =
        sign_tx_(tx, &tx_info.omnilock_config, args.sender_key.clone(), env)?;
    let witness_args = WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
    let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
    let zero_lock = tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)?;
    if lock_field.len() == zero_lock.len() && lock_field != previous_lock_field {
        if still_locked_groups.is_empty() {
            let multisig_config = tx_info.omnilock_config.multisig_config().unwrap();
            let n = multisig_config.threshold();
            let omnilock_witnesslock = OmniLockWitnessLock::from_slice(lock_field.as_ref())?;
            let omni_sig = omnilock_witnesslock
                .signature()
                .to_opt()
                .map(|data| data.raw_data().as_ref().to_vec())
                .unwrap();

            let mut idx = multisig_config.to_witness_data().len();
            if tx_info.omnilock_config.is_opentx_mode() {
                if let Some(opentx_wit) = tx_info.omnilock_config.get_opentx_input() {
                    idx += opentx_wit.opentx_sig_data_len();
                }
            }
            let mut empty_n = 0u32; // empty number of slices of signatures.
            while idx < omni_sig.len() {
                if omni_sig[idx..idx + 65] == [0u8; 65] {
                    empty_n += 1;
                }
                idx += 65;
            }
            if empty_n == 0 {
                println!("> transaction signed!");
            } else if empty_n <= n as u32 {
                println!("> {} more signature(s) need !", empty_n);
            } else {
                bail!("{} signatures need, but got {} left to sign!", n, empty_n)
            }
        } else {
            println!("> {} groups left to sign!", still_locked_groups.len());
        }
    } else if lock_field == zero_lock || zero_lock.len() != lock_field.len() {
        bail!("Failed to sign the transaction!");
    } else {
        bail!("You may tried signed the second time with the same private key!");
    }
    let tx_info = TxInfo {
        transaction: json_types::Transaction::from(tx.data()),
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn sign_tx_(
    tx: TransactionView,
    omnilock_config: &OmniLockConfig,
    keys: Vec<PrivkeyWrapper>,
    env: &ConfigContext,
) -> Result<(TransactionView, Vec<ScriptGroup>)> {
    // Unlock transaction
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(env.ckb_rpc.as_str(), 10);

    let cell = build_omnilock_cell_dep(
        env.ckb_rpc.as_str(),
        &env.omnilock_tx_hash,
        env.omnilock_index,
    )?;

    let unlockers = build_omnilock_unlockers(keys, omnilock_config.clone(), cell.type_hash);
    let (new_tx, new_still_locked_groups) = unlock_tx(tx, &tx_dep_provider, &unlockers)?;
    Ok((new_tx, new_still_locked_groups))
}
