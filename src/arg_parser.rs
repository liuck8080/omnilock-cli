use anyhow::Result;
use ckb_sdk::util::zeroize_privkey;
use ckb_types::{H160, H256};
use std::str::FromStr;

#[allow(clippy::wrong_self_convention)]
pub trait ArgParser<T> {
    fn parse(input: &str) -> Result<T>;
}
macro_rules! arg_parser {
    ($name:ident) => {
        impl ArgParser<$name> for $name {
            fn parse(s: &str) -> Result<$name> {
                let s = if s.starts_with("0x") || s.starts_with("0X") {
                    &s[2..]
                } else {
                    s
                };
                Ok($name::from_str(s)?)
            }
        }
    };
}
arg_parser!(H160);
arg_parser!(H256);

impl ArgParser<secp256k1::PublicKey> for secp256k1::PublicKey {
    fn parse(s: &str) -> Result<secp256k1::PublicKey> {
        let s = if s.starts_with("0x") || s.starts_with("0X") {
            &s[2..]
        } else {
            s
        };
        Ok(secp256k1::PublicKey::from_str(s)?)
    }
}

#[derive(Clone)]
pub struct PrivkeyWrapper(pub secp256k1::SecretKey);

// For security purpose
impl Drop for PrivkeyWrapper {
    fn drop(&mut self) {
        zeroize_privkey(&mut self.0);
    }
}

impl std::ops::Deref for PrivkeyWrapper {
    type Target = secp256k1::SecretKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct PrivkeyArgParser;

impl ArgParser<PrivkeyWrapper> for PrivkeyArgParser {
    fn parse(s: &str) -> Result<PrivkeyWrapper> {
        let data = H256::parse(s)?;
        let ret = secp256k1::SecretKey::from_slice(data.as_bytes()).map(PrivkeyWrapper)?;
        Ok(ret)
    }
}
