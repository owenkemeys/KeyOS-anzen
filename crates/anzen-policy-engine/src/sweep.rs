// Cooperative sweep handling follows Luke Childs' Anzen src/core/recovery.rs and
// src/core/transactions.rs at 01794bb14d34d01413e3b539c7e5496ecbce0c87
// under the MIT license.

use crate::{
    policy::{estimate_cooperative_vsize, extract_cooperative_keys, VaultPolicy},
    validation::verify_phone_signature,
    AnzenIdentity, PolicyError, MAX_PACKAGE_BYTES,
};
use bitcoin::{
    hashes::Hash,
    secp256k1::{Message, Secp256k1, XOnlyPublicKey},
    sighash::{SighashCache, TapSighashType},
    taproot, Address, Network, Psbt, Sequence,
};
use miniscript::psbt::{PsbtExt, PsbtSighashMsg};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const SWEEP_PACKAGE_VERSION: u8 = 1;
const SWEEP_PACKAGE_KIND: &str = "cooperative-sweep";
const DEFAULT_FEE_RATE_SAT_VB: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CooperativeSweepPackage {
    pub version: u8,
    pub kind: String,
    pub vault_descriptor: String,
    pub destination: String,
    pub psbt: String,
    pub input_count: usize,
    pub sent_sats: u64,
    pub fee_sats: u64,
    pub phone_approved: bool,
    pub hww_approved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CooperativeSweepSummary {
    pub network: String,
    pub destination: String,
    pub input_count: usize,
    pub sent_sats: u64,
    pub fee_sats: u64,
    pub phone_approved: bool,
    pub hww_approved: bool,
}

pub struct ValidatedCooperativeSweep {
    package: CooperativeSweepPackage,
    policy: VaultPolicy,
    psbt: Psbt,
    hww_pubkey: XOnlyPublicKey,
}

pub struct ApprovedCooperativeSweep {
    package: CooperativeSweepPackage,
    hww_signature_count: usize,
}

impl CooperativeSweepPackage {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(PolicyError::PackageTooLarge);
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)?;
        if package.version != SWEEP_PACKAGE_VERSION || package.kind != SWEEP_PACKAGE_KIND {
            return Err(PolicyError::UnsupportedPackage);
        }
        Ok(package)
    }

    pub fn summary(&self) -> Result<CooperativeSweepSummary, PolicyError> {
        let network = destination_network(&self.destination)?;
        Ok(CooperativeSweepSummary {
            network: network_name(network).to_owned(),
            destination: self.destination.clone(),
            input_count: self.input_count,
            sent_sats: self.sent_sats,
            fee_sats: self.fee_sats,
            phone_approved: self.phone_approved,
            hww_approved: self.hww_approved,
        })
    }

    pub fn validate(
        self,
        expected_hww: XOnlyPublicKey,
    ) -> Result<ValidatedCooperativeSweep, PolicyError> {
        if self.version != SWEEP_PACKAGE_VERSION || self.kind != SWEEP_PACKAGE_KIND {
            return Err(PolicyError::UnsupportedPackage);
        }
        if !self.phone_approved || self.hww_approved {
            return invalid("invalid cooperative sweep approval state");
        }
        let network = destination_network(&self.destination)?;
        let destination = Address::from_str(&self.destination)
            .map_err(|_| PolicyError::InvalidPolicy("invalid sweep destination"))?
            .require_network(network)
            .map_err(|_| PolicyError::InvalidPolicy("sweep destination network mismatch"))?;
        let (phone_pubkey, descriptor_hww) = extract_cooperative_keys(&self.vault_descriptor)
            .ok_or(PolicyError::InvalidPolicy("invalid vault descriptor"))?;
        if descriptor_hww != expected_hww {
            return invalid("HWW key does not match vault descriptor");
        }
        let policy = VaultPolicy::new(phone_pubkey, expected_hww, network);
        if self.vault_descriptor != policy.descriptor_string() {
            return invalid("vault descriptor or recovery paths mismatch");
        }
        let psbt = Psbt::from_str(&self.psbt).map_err(|_| PolicyError::InvalidPsbt)?;
        let transaction = &psbt.unsigned_tx;
        if transaction.input.len() != self.input_count
            || psbt.inputs.len() != self.input_count
            || transaction.input.is_empty()
            || transaction
                .input
                .iter()
                .any(|input| input.sequence != Sequence::MAX)
            || transaction.output.len() != 1
            || transaction.output[0].script_pubkey != destination.script_pubkey()
            || transaction.output[0].value.to_sat() != self.sent_sats
        {
            return invalid("cooperative sweep transaction does not match its package");
        }
        let vault_script = policy.address.script_pubkey();
        let input_sats = psbt.inputs.iter().try_fold(0_u64, |total, input| {
            let prevout = input
                .witness_utxo
                .as_ref()
                .ok_or(PolicyError::InvalidPolicy(
                    "cooperative sweep input lacks witness UTXO",
                ))?;
            if prevout.script_pubkey != vault_script {
                return invalid("cooperative sweep input is outside the vault policy");
            }
            total
                .checked_add(prevout.value.to_sat())
                .ok_or(PolicyError::InvalidPolicy(
                    "cooperative sweep input sum overflowed",
                ))
        })?;
        let fee_sats = input_sats
            .checked_sub(self.sent_sats)
            .ok_or(PolicyError::InvalidPolicy(
                "cooperative sweep outputs exceed its inputs",
            ))?;
        let expected_fee =
            estimate_cooperative_vsize(transaction, &policy) * DEFAULT_FEE_RATE_SAT_VB;
        if fee_sats != self.fee_sats || fee_sats != expected_fee {
            return invalid("cooperative sweep fee does not match its package");
        }
        let leaf = policy.cooperative_leaf();
        verify_phone_signature(&psbt, &leaf, phone_pubkey)?;
        if psbt.inputs.iter().any(|input| {
            input
                .tap_script_sigs
                .contains_key(&(expected_hww, leaf.leaf_hash))
        }) {
            return invalid("proposal already contains an HWW signature");
        }
        Ok(ValidatedCooperativeSweep {
            package: self,
            policy,
            psbt,
            hww_pubkey: expected_hww,
        })
    }
}

impl ValidatedCooperativeSweep {
    pub fn input_count(&self) -> usize {
        self.package.input_count
    }

    pub fn approve(
        mut self,
        identity: &AnzenIdentity,
    ) -> Result<ApprovedCooperativeSweep, PolicyError> {
        if identity.public_key() != self.hww_pubkey {
            return Err(PolicyError::Signing);
        }
        let leaf = self.policy.cooperative_leaf();
        let secp = Secp256k1::new();
        let mut hww_signature_count = 0;
        for index in 0..self.psbt.inputs.len() {
            let origins = self.psbt.inputs[index]
                .tap_key_origins
                .get(&self.hww_pubkey)
                .ok_or(PolicyError::Signing)?;
            if !origins.0.contains(&leaf.leaf_hash) {
                return Err(PolicyError::Signing);
            }
            let unsigned_tx = self.psbt.unsigned_tx.clone();
            let mut cache = SighashCache::new(&unsigned_tx);
            let message = match self
                .psbt
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
            self.psbt.inputs[index].tap_script_sigs.insert(
                (self.hww_pubkey, leaf.leaf_hash),
                taproot::Signature {
                    signature,
                    sighash_type: TapSighashType::Default,
                },
            );
            hww_signature_count += 1;
        }
        self.package.psbt = self.psbt.to_string();
        self.package.hww_approved = true;
        Ok(ApprovedCooperativeSweep {
            package: self.package,
            hww_signature_count,
        })
    }
}

impl ApprovedCooperativeSweep {
    pub(crate) fn into_package(self) -> CooperativeSweepPackage {
        self.package
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

fn destination_network(destination: &str) -> Result<Network, PolicyError> {
    let address = Address::from_str(destination)
        .map_err(|_| PolicyError::InvalidPolicy("invalid sweep destination"))?;
    if address.is_valid_for_network(Network::Regtest) {
        Ok(Network::Regtest)
    } else if address.is_valid_for_network(Network::Bitcoin) {
        Ok(Network::Bitcoin)
    } else {
        invalid("unsupported sweep destination network")
    }
}

fn network_name(network: Network) -> &'static str {
    match network {
        Network::Bitcoin => "bitcoin",
        Network::Regtest => "regtest",
        _ => "unsupported",
    }
}

fn invalid<T>(message: &'static str) -> Result<T, PolicyError> {
    Err(PolicyError::InvalidPolicy(message))
}
