use ckb_hash::blake2b_256;
use ckb_jsonrpc_types as json_types;
use ckb_sdk::{
    traits::DefaultTransactionDependencyProvider,
    tx_builder::unlock_tx,
    unlock::{OmniLockConfig, OmniUnlockMode},
    ScriptGroup, SECP256K1,
};
use ckb_types::{
    core::TransactionView,
    molecule::hex_string,
    packed::{Transaction, WitnessArgs},
    prelude::*,
    H160, H256,
};
use clap::Args;
use rpassword::prompt_password_stdout;
use std::fs;
use std::path::PathBuf;

use crate::{
    client::build_omnilock_cell_dep, config::ConfigContext, generate::build_omnilock_unlockers,
    keystore::CkbKeyStore, txinfo::TxInfo,
};
use anyhow::{bail, Context, Result};

#[derive(Args)]
pub struct SignTxArgs {
    /// The sender private key (hex string)
    #[clap(long, value_name = "PRIV_KEY")]
    pub sender_key: Option<H256>,
    /// the unlock account
    #[clap(long, value_name = "ACCOUNT")]
    pub from_account: Option<H160>,
    /// The output transaction info file (.json)
    #[clap(long, value_name = "PATH")]
    pub tx_file: PathBuf,
}

pub fn sign_tx(args: &SignTxArgs, env: &ConfigContext) -> Result<()> {
    let tx_info: TxInfo = serde_json::from_slice(&fs::read(&args.tx_file)?)?;
    let tx = Transaction::from(tx_info.transaction).into_view();

    let key = if let Some(sender_key) = args.sender_key.as_ref() {
        secp256k1::SecretKey::from_slice(sender_key.as_bytes())
            .with_context(|| format!("invalid sender secret key: {}", sender_key))?
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
    let (tx, _) = sign_tx_(tx, &tx_info.omnilock_config, key, env)?;
    let witness_args = WitnessArgs::from_slice(tx.witnesses().get(0).unwrap().raw_data().as_ref())?;
    let lock_field = witness_args.lock().to_opt().unwrap().raw_data();
    if lock_field != tx_info.omnilock_config.zero_lock(OmniUnlockMode::Normal)? {
        println!("> transaction ready to send!");
    } else {
        bail!("failed to sign tx");
    }
    let tx_info = TxInfo {
        transaction: json_types::Transaction::from(tx.data()),
        omnilock_config: tx_info.omnilock_config,
    };
    fs::write(&args.tx_file, serde_json::to_string_pretty(&tx_info)?)?;
    Ok(())
}

fn sign_tx_(
    mut tx: TransactionView,
    omnilock_config: &OmniLockConfig,
    key: secp256k1::SecretKey,
    env: &ConfigContext,
) -> Result<(TransactionView, Vec<ScriptGroup>)> {
    // Unlock transaction
    let tx_dep_provider = DefaultTransactionDependencyProvider::new(env.ckb_rpc.as_str(), 10);

    let cell = build_omnilock_cell_dep(
        env.ckb_rpc.as_str(),
        &env.omnilock_tx_hash,
        env.omnilock_index,
    )?;

    let mut _still_locked_groups = None;
    let unlockers = build_omnilock_unlockers(vec![key], omnilock_config.clone(), cell.type_hash);
    let (new_tx, new_still_locked_groups) = unlock_tx(tx.clone(), &tx_dep_provider, &unlockers)?;
    tx = new_tx;
    _still_locked_groups = Some(new_still_locked_groups);
    Ok((tx, _still_locked_groups.unwrap_or_default()))
}
