use anyhow::Result;
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
