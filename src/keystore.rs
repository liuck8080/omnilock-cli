use anyhow::{Context, Result};
use ckb_signer::{KeyStore, ScryptType};
use ckb_types::H160;

use crate::arg_parser::PrivkeyWrapper;

pub struct CkbKeyStore {
    key_store: KeyStore,
}

impl CkbKeyStore {
    pub fn load_default() -> Result<Self> {
        let mut dir = dirs::home_dir().unwrap();
        dir.push(".ckb-cli");
        dir.push("keystore");
        let key_store = KeyStore::from_dir(dir.clone(), ScryptType::default())
            .with_context(|| format!("try to load from directory {}", dir.to_string_lossy()))?;
        Ok(CkbKeyStore { key_store })
    }

    pub fn export_priv_key(&self, hash160: &H160, password: &[u8]) -> Result<PrivkeyWrapper> {
        let master_priv_key = self
            .key_store
            .export_key(hash160, password)
            .with_context(|| format!("try to export key of {:#x}", hash160))?;
        let bytes = master_priv_key.to_bytes();
        // println!("private key:{}", hex_string(&bytes));
        let key = secp256k1::SecretKey::from_slice(&bytes[0..32])?;
        Ok(PrivkeyWrapper(key))
    }
}
