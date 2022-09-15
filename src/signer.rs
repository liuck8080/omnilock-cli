use ckb_sdk::traits::{Signer, SignerError};

use ckb_types::{bytes::Bytes, core::TransactionView};
pub struct CommonSigner {
    signers: Vec<Box<dyn Signer>>,
}

impl CommonSigner {
    pub fn new(signers: Vec<Box<dyn Signer>>) -> CommonSigner {
        CommonSigner { signers }
    }

    fn get_signer(&self, id: &[u8]) -> Option<&dyn Signer> {
        for signer in &self.signers {
            if signer.match_id(id) {
                return Some(signer.as_ref());
            }
        }
        None
    }
}

impl Signer for CommonSigner {
    fn match_id(&self, id: &[u8]) -> bool {
        self.get_signer(id).is_some()
    }

    fn sign(
        &self,
        id: &[u8],
        message: &[u8],
        recoverable: bool,
        tx: &TransactionView,
    ) -> Result<Bytes, SignerError> {
        let signer = self.get_signer(id).ok_or(SignerError::IdNotFound)?;
        signer.sign(id, message, recoverable, tx)
    }
}
