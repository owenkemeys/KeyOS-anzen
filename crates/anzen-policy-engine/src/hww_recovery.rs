// Delayed HWW recovery follows Luke Childs' Anzen src/core/recovery.rs and
// src/core/transactions.rs at 01794bb14d34d01413e3b539c7e5496ecbce0c87
// under the MIT license. The snapshot envelope is explicitly development-only
// test plumbing; Luke's current command reads chain state directly.

use crate::{
    policy::{
        estimate_hww_recovery_vsize, extract_cooperative_keys, VaultPolicy, HWW_RECOVERY_BLOCKS,
    },
    AnzenIdentity, PolicyError, MAX_PACKAGE_BYTES,
};
use bitcoin::{
    absolute,
    consensus::serialize,
    hashes::Hash,
    psbt::PsbtSighashType,
    secp256k1::{Message, Secp256k1, XOnlyPublicKey},
    sighash::{SighashCache, TapSighashType},
    taproot,
    transaction::Version,
    Address, Amount, Network, OutPoint, Psbt, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Txid,
    Witness,
};
use miniscript::{
    descriptor::DescriptorPublicKey,
    psbt::{PsbtExt, PsbtSighashMsg},
    Descriptor,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, str::FromStr};

const SNAPSHOT_VERSION: u8 = 1;
const SNAPSHOT_KIND: &str = "hww-recovery-snapshot";
const RESULT_KIND: &str = "hww-recovery-transaction";
const DEFAULT_FEE_RATE_SAT_VB: u64 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwwRecoveryUtxo {
    pub txid: String,
    pub vout: u32,
    pub value_sats: u64,
    pub script_pubkey: String,
    pub confirmation_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HwwRecoverySnapshot {
    pub version: u8,
    pub kind: String,
    pub network: String,
    pub vault_descriptor: String,
    pub destination: String,
    pub tip_height: u64,
    pub utxos: Vec<HwwRecoveryUtxo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HwwRecoverySummary {
    pub network: String,
    pub destination: String,
    pub snapshot_tip_height: u64,
    pub delay_blocks: u16,
    pub input_count: usize,
    pub sent_sats: u64,
    pub fee_sats: u64,
}

#[derive(Debug, Clone)]
pub struct ReviewedHwwRecovery {
    summary: HwwRecoverySummary,
    policy: VaultPolicy,
    psbt: Psbt,
    hww_pubkey: XOnlyPublicKey,
}

pub struct ApprovedHwwRecovery {
    summary: HwwRecoverySummary,
    transaction: Transaction,
    hww_signature_count: usize,
}

#[derive(Debug, Serialize)]
struct HwwRecoveryResult<'a> {
    version: u8,
    kind: &'static str,
    network: &'a str,
    destination: &'a str,
    snapshot_tip_height: u64,
    delay_blocks: u16,
    input_count: usize,
    sent_sats: u64,
    fee_sats: u64,
    txid: String,
    transaction_hex: String,
}

impl HwwRecoverySnapshot {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(PolicyError::PackageTooLarge);
        }
        let snapshot: Self = serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)?;
        if snapshot.version != SNAPSHOT_VERSION || snapshot.kind != SNAPSHOT_KIND {
            return Err(PolicyError::UnsupportedPackage);
        }
        Ok(snapshot)
    }

    pub fn review(self) -> Result<ReviewedHwwRecovery, PolicyError> {
        let network = parse_network(&self.network)?;
        let destination = Address::from_str(&self.destination)
            .map_err(|_| PolicyError::InvalidPolicy("invalid recovery destination"))?
            .require_network(network)
            .map_err(|_| PolicyError::InvalidPolicy("recovery destination network mismatch"))?;
        let (phone_pubkey, hww_pubkey) = extract_cooperative_keys(&self.vault_descriptor)
            .ok_or(PolicyError::InvalidPolicy("invalid vault descriptor"))?;
        let policy = VaultPolicy::new(phone_pubkey, hww_pubkey, network);
        if policy.descriptor_string() != self.vault_descriptor {
            return invalid("vault descriptor or recovery paths mismatch");
        }
        let vault_script = policy.address.script_pubkey();
        let next_height = self.tip_height.saturating_add(1);
        let mut seen = HashSet::new();
        let mut mature = Vec::new();
        for utxo in self.utxos {
            let txid = Txid::from_str(&utxo.txid)
                .map_err(|_| PolicyError::InvalidPolicy("invalid recovery outpoint"))?;
            let outpoint = OutPoint::new(txid, utxo.vout);
            if !seen.insert(outpoint) {
                return invalid("duplicate recovery outpoint");
            }
            let script = ScriptBuf::from_bytes(decode_hex(&utxo.script_pubkey)?);
            if script != vault_script {
                return invalid("recovery input is outside the vault policy");
            }
            if utxo
                .confirmation_height
                .saturating_add(u64::from(HWW_RECOVERY_BLOCKS))
                <= next_height
            {
                mature.push((
                    outpoint,
                    TxOut {
                        value: Amount::from_sat(utxo.value_sats),
                        script_pubkey: script,
                    },
                ));
            }
        }
        if mature.is_empty() {
            return invalid("no vault UTXOs are mature for HWW recovery");
        }
        let input_sats = mature.iter().try_fold(0_u64, |total, (_, output)| {
            total
                .checked_add(output.value.to_sat())
                .ok_or(PolicyError::InvalidPolicy("recovery input sum overflowed"))
        })?;
        let mut transaction = Transaction {
            version: Version::TWO,
            lock_time: absolute::LockTime::ZERO,
            input: mature
                .iter()
                .map(|(outpoint, _)| TxIn {
                    previous_output: *outpoint,
                    script_sig: ScriptBuf::new(),
                    sequence: Sequence::from_consensus(u32::from(HWW_RECOVERY_BLOCKS)),
                    witness: Witness::new(),
                })
                .collect(),
            output: vec![TxOut {
                value: Amount::from_sat(input_sats),
                script_pubkey: destination.script_pubkey(),
            }],
        };
        let fee_sats = estimate_hww_recovery_vsize(&transaction, &policy)
            .checked_mul(DEFAULT_FEE_RATE_SAT_VB)
            .ok_or(PolicyError::InvalidPolicy("recovery fee overflowed"))?;
        let sent_sats = input_sats
            .checked_sub(fee_sats)
            .ok_or(PolicyError::InvalidPolicy("recovery fee exceeds inputs"))?;
        if sent_sats
            < transaction.output[0]
                .script_pubkey
                .minimal_non_dust()
                .to_sat()
        {
            return invalid("recovery output would be dust");
        }
        transaction.output[0].value = Amount::from_sat(sent_sats);

        let descriptor = Descriptor::<DescriptorPublicKey>::from_str(&self.vault_descriptor)
            .map_err(|_| PolicyError::InvalidPolicy("invalid vault descriptor"))?
            .at_derivation_index(0)
            .map_err(|_| PolicyError::InvalidPolicy("invalid vault descriptor"))?;
        let mut psbt = Psbt::from_unsigned_tx(transaction).map_err(|_| PolicyError::InvalidPsbt)?;
        for (index, (_, prevout)) in mature.iter().enumerate() {
            psbt.inputs[index].witness_utxo = Some(prevout.clone());
            psbt.inputs[index].sighash_type = Some(PsbtSighashType::from(TapSighashType::Default));
            psbt.update_input_with_descriptor(index, &descriptor)
                .map_err(|_| PolicyError::InvalidPsbt)?;
        }
        let summary = HwwRecoverySummary {
            network: self.network,
            destination: self.destination,
            snapshot_tip_height: self.tip_height,
            delay_blocks: HWW_RECOVERY_BLOCKS,
            input_count: mature.len(),
            sent_sats,
            fee_sats,
        };
        Ok(ReviewedHwwRecovery {
            summary,
            policy,
            psbt,
            hww_pubkey,
        })
    }
}

impl ReviewedHwwRecovery {
    pub fn summary(&self) -> &HwwRecoverySummary {
        &self.summary
    }

    pub fn approve(mut self, identity: &AnzenIdentity) -> Result<ApprovedHwwRecovery, PolicyError> {
        if identity.public_key() != self.hww_pubkey {
            return Err(PolicyError::Signing);
        }
        let leaf = self.policy.hww_recovery_leaf();
        let secp = Secp256k1::new();
        let mut count = 0;
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
            self.psbt.inputs[index].tap_script_sigs.insert(
                (self.hww_pubkey, leaf.leaf_hash),
                taproot::Signature {
                    signature,
                    sighash_type: TapSighashType::Default,
                },
            );
            count += 1;
        }
        self.psbt
            .finalize_mut(&Secp256k1::verification_only())
            .map_err(|_| PolicyError::Signing)?;
        let transaction = self
            .psbt
            .extract(&Secp256k1::verification_only())
            .map_err(|_| PolicyError::Signing)?;
        Ok(ApprovedHwwRecovery {
            summary: self.summary,
            transaction,
            hww_signature_count: count,
        })
    }
}

impl ApprovedHwwRecovery {
    pub fn transaction_bytes(&self) -> Vec<u8> {
        serialize(&self.transaction)
    }

    pub fn hww_signature_count(&self) -> usize {
        self.hww_signature_count
    }

    pub fn to_json(&self) -> Result<Vec<u8>, PolicyError> {
        let bytes = self.transaction_bytes();
        let result = HwwRecoveryResult {
            version: 1,
            kind: RESULT_KIND,
            network: &self.summary.network,
            destination: &self.summary.destination,
            snapshot_tip_height: self.summary.snapshot_tip_height,
            delay_blocks: self.summary.delay_blocks,
            input_count: self.summary.input_count,
            sent_sats: self.summary.sent_sats,
            fee_sats: self.summary.fee_sats,
            txid: self.transaction.compute_txid().to_string(),
            transaction_hex: encode_hex(&bytes),
        };
        let mut json =
            serde_json::to_vec_pretty(&result).map_err(|_| PolicyError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}

fn parse_network(name: &str) -> Result<Network, PolicyError> {
    match name {
        "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
        "regtest" => Ok(Network::Regtest),
        _ => invalid("unsupported Bitcoin network"),
    }
}

fn decode_hex(text: &str) -> Result<Vec<u8>, PolicyError> {
    if !text.len().is_multiple_of(2) {
        return invalid("invalid script pubkey hex");
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = hex_nibble(pair[0])?;
            let low = hex_nibble(pair[1])?;
            Ok((high << 4) | low)
        })
        .collect()
}

fn hex_nibble(value: u8) -> Result<u8, PolicyError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(PolicyError::InvalidPolicy("invalid script pubkey hex")),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn invalid<T>(message: &'static str) -> Result<T, PolicyError> {
    Err(PolicyError::InvalidPolicy(message))
}
