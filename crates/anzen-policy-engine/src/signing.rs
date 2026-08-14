// Signing follows Luke Childs' Anzen src/core/transactions.rs at
// 01794bb14d34d01413e3b539c7e5496ecbce0c87 under the MIT license.

use crate::{AnzenIdentity, PolicyError, PolicyPackage, ValidatedPolicy};
use bitcoin::{
    hashes::Hash,
    secp256k1::{Message, Secp256k1},
    sighash::{SighashCache, TapSighashType},
    taproot,
};
use miniscript::psbt::{PsbtExt, PsbtSighashMsg};

pub struct ApprovedPolicyPackage {
    package: PolicyPackage,
    hww_signature_count: usize,
}

impl ValidatedPolicy {
    pub fn approve(
        mut self,
        identity: &AnzenIdentity,
    ) -> Result<ApprovedPolicyPackage, PolicyError> {
        if identity.public_key() != self.hww_pubkey {
            return Err(PolicyError::Signing);
        }
        let leaf = self.policy.cooperative_leaf();
        let paths = self
            .package
            .manifest_transactions()
            .into_iter()
            .map(|transaction| transaction.psbt_file.clone())
            .collect::<Vec<_>>();
        let secp = Secp256k1::new();
        let mut hww_signature_count = 0;

        for path in paths {
            let psbt = self
                .psbts
                .get_mut(&path)
                .ok_or(PolicyError::PsbtSetMismatch)?;
            for index in 0..psbt.inputs.len() {
                let origins = psbt.inputs[index]
                    .tap_key_origins
                    .get(&self.hww_pubkey)
                    .ok_or(PolicyError::Signing)?;
                if !origins.0.contains(&leaf.leaf_hash) {
                    return Err(PolicyError::Signing);
                }
                let unsigned_tx = psbt.unsigned_tx.clone();
                let mut cache = SighashCache::new(&unsigned_tx);
                let message = match psbt
                    .sighash_msg(index, &mut cache, Some(leaf.leaf_hash))
                    .map_err(|_| PolicyError::Signing)?
                {
                    PsbtSighashMsg::TapSighash(sighash) => {
                        Message::from_digest(sighash.to_byte_array())
                    }
                    _ => return Err(PolicyError::Signing),
                };
                let signature = secp.sign_schnorr_no_aux_rand(&message, &identity.keypair);
                secp.verify_schnorr(&signature, &message, &self.hww_pubkey)
                    .map_err(|_| PolicyError::Signing)?;
                psbt.inputs[index].tap_script_sigs.insert(
                    (self.hww_pubkey, leaf.leaf_hash),
                    taproot::Signature {
                        signature,
                        sighash_type: TapSighashType::Default,
                    },
                );
                hww_signature_count += 1;
            }
            self.package.psbts.insert(path, psbt.to_string());
        }
        self.package.manifest.hww_approved = true;
        Ok(ApprovedPolicyPackage {
            package: self.package,
            hww_signature_count,
        })
    }
}

impl ApprovedPolicyPackage {
    pub(crate) fn into_package(self) -> PolicyPackage {
        self.package
    }

    pub fn hww_approved(&self) -> bool {
        self.package.manifest.hww_approved
    }

    pub fn hww_signature_count(&self) -> usize {
        self.hww_signature_count
    }

    pub fn to_json(&self) -> Result<Vec<u8>, PolicyError> {
        let mut json =
            serde_json::to_vec_pretty(&self.package).map_err(|_| PolicyError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}
