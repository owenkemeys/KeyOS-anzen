// Validation rules are adapted from Luke Childs' Anzen src/core/ceremony.rs and
// src/core/transactions.rs at 01794bb14d34d01413e3b539c7e5496ecbce0c87
// under the MIT license. This version operates only on the in-memory package.

use crate::{
    policy::{estimate_cooperative_vsize, extract_cooperative_keys, VaultPolicy},
    BatchManifest, BatchTransaction, PolicyError, PolicyPackage,
};
use bitcoin::{
    absolute,
    hashes::Hash,
    secp256k1::{Message, Secp256k1, XOnlyPublicKey},
    sighash::{SighashCache, TapSighashType},
    transaction::Version,
    Address, Network, OutPoint, Psbt, Sequence, Transaction, TxOut,
};
use miniscript::psbt::{PsbtExt, PsbtSighashMsg};
use std::{collections::BTreeMap, str::FromStr};

const MONTHS_PER_ROLLOVER: usize = 12;
const DEFAULT_FEE_RATE_SAT_VB: u64 = 1;
const MONTHLY_ALLOWANCE_DELAY_SECONDS: u32 = 2_592_256;
const EMERGENCY_ACCESS_DELAY_SECONDS: u32 = 605_184;
const MONTHLY_DELAY_SEQUENCE: u32 = 4_199_367;
const EMERGENCY_DELAY_SEQUENCE: u32 = 4_195_486;

pub struct ValidatedPolicy {
    pub(crate) package: PolicyPackage,
    pub(crate) policy: VaultPolicy,
    pub(crate) psbts: BTreeMap<String, Psbt>,
    pub(crate) hww_pubkey: XOnlyPublicKey,
}

impl ValidatedPolicy {
    pub fn psbt_count(&self) -> usize {
        self.psbts.len()
    }

    pub fn summary(&self) -> Result<crate::PolicySummary, PolicyError> {
        self.package.summary()
    }
}

impl PolicyPackage {
    pub fn validate(self, expected_hww: XOnlyPublicKey) -> Result<ValidatedPolicy, PolicyError> {
        let network = parse_network(&self.manifest.network)?;
        if !self.manifest.phone_approved || self.manifest.hww_approved {
            return Err(PolicyError::InvalidPolicy("invalid approval state"));
        }
        let (phone_pubkey, descriptor_hww) =
            extract_cooperative_keys(&self.manifest.vault_descriptor)
                .ok_or(PolicyError::InvalidPolicy("invalid vault descriptor"))?;
        if descriptor_hww != expected_hww {
            return Err(PolicyError::InvalidPolicy(
                "HWW key does not match vault descriptor",
            ));
        }
        let policy = VaultPolicy::new(phone_pubkey, expected_hww, network);
        if self.manifest.vault_descriptor != policy.descriptor_string()
            || self.manifest.vault_address != policy.address.to_string()
        {
            return Err(PolicyError::InvalidPolicy(
                "descriptor, recovery paths, or vault address mismatch",
            ));
        }

        let mut psbts = BTreeMap::new();
        for (path, encoded) in &self.psbts {
            let psbt = Psbt::from_str(encoded).map_err(|_| PolicyError::InvalidPsbt)?;
            psbts.insert(path.clone(), psbt);
        }
        validate_batch(&self.manifest, &psbts, &policy, network)?;
        let leaf = policy.cooperative_leaf();
        for transaction in self.manifest_transactions() {
            let psbt = psbts
                .get(&transaction.psbt_file)
                .ok_or(PolicyError::PsbtSetMismatch)?;
            verify_phone_signature(psbt, &leaf, phone_pubkey)?;
            if psbt.inputs.iter().any(|input| {
                input
                    .tap_script_sigs
                    .contains_key(&(expected_hww, leaf.leaf_hash))
            }) {
                return Err(PolicyError::InvalidPolicy(
                    "proposal already contains an HWW signature",
                ));
            }
        }

        Ok(ValidatedPolicy {
            package: self,
            policy,
            psbts,
            hww_pubkey: expected_hww,
        })
    }
}

fn validate_batch(
    manifest: &BatchManifest,
    psbts: &BTreeMap<String, Psbt>,
    policy: &VaultPolicy,
    network: Network,
) -> Result<(), PolicyError> {
    let monthly_disabled = manifest.monthly_limit_sats == 0;
    if (monthly_disabled && manifest.allowance_count != 0)
        || manifest.allowance_count > MONTHS_PER_ROLLOVER
        || manifest.allowances.len() != manifest.allowance_count
    {
        return invalid("invalid allowance count");
    }
    if manifest.fee_rate_sat_vb != DEFAULT_FEE_RATE_SAT_VB {
        return invalid("fee rate is not one sat/vB");
    }
    let vault_script = policy.address.script_pubkey();
    let rollover = get_psbt(psbts, &manifest.rollover.psbt_file)?;
    let rollover_tx = &rollover.unsigned_tx;
    if rollover_tx.compute_txid().to_string() != manifest.rollover.unsigned_txid
        || rollover_tx.version != Version::TWO
        || rollover_tx.lock_time != absolute::LockTime::ZERO
        || rollover_tx.input.is_empty()
        || rollover_tx
            .input
            .iter()
            .any(|input| input.sequence != Sequence::MAX)
        || rollover_tx.output.len() != usize::from(manifest.allowance_count > 0) + 1
        || rollover_tx.input.len() != rollover.inputs.len()
        || rollover_tx
            .output
            .iter()
            .any(|output| output.script_pubkey != vault_script)
    {
        return invalid("rollover does not match manifest");
    }
    let rollover_input = psbt_input_sum(rollover)?;
    let rollover_output = output_sum(rollover_tx)?;
    if rollover_input != manifest.total_input_sats
        || rollover_input
            .checked_sub(rollover_output)
            .ok_or(PolicyError::InvalidPolicy("rollover outputs exceed inputs"))?
            != manifest.rollover.fee_sats
        || manifest.rollover.fee_sats
            != estimate_cooperative_vsize(rollover_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
    {
        return invalid("rollover amount or fee mismatch");
    }

    let has_allowances = manifest.allowance_count > 0;
    let expected_remainder_vout = u32::from(has_allowances);
    if manifest.allowance_vout != has_allowances.then_some(0)
        || (!has_allowances && manifest.allowance_value_sats != 0)
        || (has_allowances && rollover_tx.output[0].value.to_sat() != manifest.allowance_value_sats)
        || manifest.remainder_vout != expected_remainder_vout
        || rollover_tx.output[expected_remainder_vout as usize]
            .value
            .to_sat()
            != manifest.remainder_value_sats
        || manifest.remainder_value_sats < vault_script.minimal_non_dust().to_sat()
    {
        return invalid("rollover output metadata mismatch");
    }

    let mut expected_outpoint =
        has_allowances.then(|| OutPoint::new(rollover_tx.compute_txid(), 0));
    let mut expected_prevout = has_allowances.then(|| rollover_tx.output[0].clone());
    for (index, allowance) in manifest.allowances.iter().enumerate() {
        let expected_step = u8::try_from(index + 1)
            .map_err(|_| PolicyError::InvalidPolicy("allowance step overflow"))?;
        let has_next = index + 1 < manifest.allowance_count;
        let source_outpoint =
            expected_outpoint.ok_or(PolicyError::InvalidPolicy("allowance source missing"))?;
        let source_txout = expected_prevout
            .take()
            .ok_or(PolicyError::InvalidPolicy("allowance prevout missing"))?;
        if allowance.step != expected_step
            || allowance.delay_seconds != MONTHLY_ALLOWANCE_DELAY_SECONDS
            || allowance.delay_sequence != MONTHLY_DELAY_SEQUENCE
            || allowance.chain_value_sats != source_txout.value.to_sat()
        {
            return invalid("allowance metadata mismatch");
        }
        let hot_script = Address::from_str(&allowance.hot_address)
            .map_err(|_| PolicyError::InvalidPolicy("invalid allowance address"))?
            .require_network(network)
            .map_err(|_| PolicyError::InvalidPolicy("allowance network mismatch"))?
            .script_pubkey();
        let authorization = get_psbt(psbts, &allowance.authorization.psbt_file)?;
        validate_child_common(
            authorization,
            &source_txout,
            source_outpoint,
            &allowance.authorization,
        )?;
        let auth_tx = &authorization.unsigned_tx;
        if auth_tx.version != Version::TWO
            || auth_tx.lock_time != absolute::LockTime::ZERO
            || auth_tx.input[0].sequence.to_consensus_u32() != MONTHLY_DELAY_SEQUENCE
            || auth_tx.output.len() != if has_next { 2 } else { 1 }
            || auth_tx.output[0].value.to_sat() != manifest.monthly_limit_sats
            || auth_tx.output[0].script_pubkey != hot_script
        {
            return invalid("allowance authorization mismatch");
        }
        let remaining = source_txout
            .value
            .to_sat()
            .checked_sub(manifest.monthly_limit_sats)
            .and_then(|value| value.checked_sub(allowance.authorization.fee_sats))
            .ok_or(PolicyError::InvalidPolicy("allowance exceeds chain input"))?;
        if has_next {
            if auth_tx.output[1].script_pubkey != vault_script
                || auth_tx.output[1].value.to_sat() != remaining
                || remaining < vault_script.minimal_non_dust().to_sat()
            {
                return invalid("allowance next hop mismatch");
            }
        } else if remaining != 0 {
            return invalid("final allowance does not exhaust chain");
        }

        let revocation = get_psbt(psbts, &allowance.revocation.psbt_file)?;
        validate_child_common(
            revocation,
            &source_txout,
            source_outpoint,
            &allowance.revocation,
        )?;
        let revoke_tx = &revocation.unsigned_tx;
        if revoke_tx.version != Version::TWO
            || revoke_tx.lock_time != absolute::LockTime::ZERO
            || revoke_tx.input[0].sequence != Sequence::MAX
            || revoke_tx.output.len() != 1
            || revoke_tx.output[0].script_pubkey != vault_script
            || revoke_tx.output[0].value.to_sat()
                != source_txout
                    .value
                    .to_sat()
                    .checked_sub(allowance.revocation.fee_sats)
                    .ok_or(PolicyError::InvalidPolicy("revocation fee exceeds input"))?
            || revoke_tx.output[0].value.to_sat() < vault_script.minimal_non_dust().to_sat()
            || allowance.authorization.fee_sats
                != estimate_cooperative_vsize(auth_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
            || allowance.revocation.fee_sats
                != estimate_cooperative_vsize(revoke_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
        {
            return invalid("allowance revocation or fee mismatch");
        }
        if has_next {
            expected_outpoint = Some(OutPoint::new(auth_tx.compute_txid(), 1));
            expected_prevout = Some(auth_tx.output[1].clone());
        } else {
            expected_outpoint = None;
        }
    }
    validate_emergency(manifest, psbts, policy, network, rollover)
}

fn validate_emergency(
    manifest: &BatchManifest,
    psbts: &BTreeMap<String, Psbt>,
    policy: &VaultPolicy,
    network: Network,
    rollover: &Psbt,
) -> Result<(), PolicyError> {
    let emergency = match (
        manifest.emergency_access_limit_sats,
        &manifest.emergency_access,
    ) {
        (0, None) => return Ok(()),
        (0, Some(_)) => return invalid("disabled emergency has transactions"),
        (_, None) => return invalid("emergency transactions missing"),
        (_, Some(emergency)) => emergency,
    };
    if emergency.amount_sats != manifest.emergency_access_limit_sats
        || emergency.delay_seconds != EMERGENCY_ACCESS_DELAY_SECONDS
        || emergency.delay_sequence != EMERGENCY_DELAY_SEQUENCE
        || emergency.staging_vout != 0
        || emergency.vault_change_vout != 1
    {
        return invalid("emergency metadata mismatch");
    }
    let hot_script = Address::from_str(&emergency.hot_address)
        .map_err(|_| PolicyError::InvalidPolicy("invalid emergency address"))?
        .require_network(network)
        .map_err(|_| PolicyError::InvalidPolicy("emergency network mismatch"))?
        .script_pubkey();
    let vault_script = policy.address.script_pubkey();
    let source_outpoint =
        OutPoint::new(rollover.unsigned_tx.compute_txid(), manifest.remainder_vout);
    let source_txout = rollover
        .unsigned_tx
        .output
        .get(manifest.remainder_vout as usize)
        .ok_or(PolicyError::InvalidPolicy("remainder output missing"))?;
    let trigger = get_psbt(psbts, &emergency.trigger.psbt_file)?;
    validate_child_common(trigger, source_txout, source_outpoint, &emergency.trigger)?;
    let trigger_tx = &trigger.unsigned_tx;
    if trigger_tx.version != Version::TWO
        || trigger_tx.lock_time != absolute::LockTime::ZERO
        || trigger_tx.input[0].sequence != Sequence::MAX
        || trigger_tx.output.len() != 2
        || trigger_tx
            .output
            .iter()
            .any(|output| output.script_pubkey != vault_script)
        || trigger_tx.output[0].value.to_sat() != emergency.staging_value_sats
        || trigger_tx.output[1].value.to_sat() != emergency.vault_change_value_sats
        || emergency.vault_change_value_sats < vault_script.minimal_non_dust().to_sat()
        || emergency.trigger.fee_sats
            != estimate_cooperative_vsize(trigger_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
    {
        return invalid("emergency trigger mismatch");
    }

    let staging_outpoint = OutPoint::new(trigger_tx.compute_txid(), emergency.staging_vout);
    let staging_txout = &trigger_tx.output[emergency.staging_vout as usize];
    let withdrawal = get_psbt(psbts, &emergency.withdrawal.psbt_file)?;
    validate_child_common(
        withdrawal,
        staging_txout,
        staging_outpoint,
        &emergency.withdrawal,
    )?;
    let withdrawal_tx = &withdrawal.unsigned_tx;
    if withdrawal_tx.version != Version::TWO
        || withdrawal_tx.lock_time != absolute::LockTime::ZERO
        || withdrawal_tx.input[0].sequence.to_consensus_u32() != EMERGENCY_DELAY_SEQUENCE
        || withdrawal_tx.output.len() != 1
        || withdrawal_tx.output[0].value.to_sat() != manifest.emergency_access_limit_sats
        || withdrawal_tx.output[0].script_pubkey != hot_script
        || Some(emergency.staging_value_sats)
            != manifest
                .emergency_access_limit_sats
                .checked_add(emergency.withdrawal.fee_sats)
        || emergency.withdrawal.fee_sats
            != estimate_cooperative_vsize(withdrawal_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
    {
        return invalid("emergency withdrawal mismatch");
    }

    let cancellation = get_psbt(psbts, &emergency.cancellation.psbt_file)?;
    validate_child_common(
        cancellation,
        staging_txout,
        staging_outpoint,
        &emergency.cancellation,
    )?;
    let cancellation_tx = &cancellation.unsigned_tx;
    if cancellation_tx.version != Version::TWO
        || cancellation_tx.lock_time != absolute::LockTime::ZERO
        || cancellation_tx.input[0].sequence != Sequence::MAX
        || cancellation_tx.output.len() != 1
        || cancellation_tx.output[0].script_pubkey != vault_script
        || cancellation_tx.output[0].value.to_sat() < vault_script.minimal_non_dust().to_sat()
        || cancellation_tx.output[0].value.to_sat()
            != emergency
                .staging_value_sats
                .checked_sub(emergency.cancellation.fee_sats)
                .ok_or(PolicyError::InvalidPolicy(
                    "emergency cancellation fee exceeds input",
                ))?
        || emergency.cancellation.fee_sats
            != estimate_cooperative_vsize(cancellation_tx, policy) * DEFAULT_FEE_RATE_SAT_VB
    {
        return invalid("emergency cancellation mismatch");
    }
    Ok(())
}

fn validate_child_common(
    psbt: &Psbt,
    expected_prevout: &TxOut,
    expected_outpoint: OutPoint,
    manifest_tx: &BatchTransaction,
) -> Result<(), PolicyError> {
    if psbt.unsigned_tx.compute_txid().to_string() != manifest_tx.unsigned_txid
        || psbt.unsigned_tx.input.len() != 1
        || psbt.inputs.len() != 1
        || psbt.unsigned_tx.input[0].previous_output != expected_outpoint
        || psbt.inputs[0].witness_utxo.as_ref() != Some(expected_prevout)
    {
        return invalid("PSBT does not spend assigned vault output");
    }
    let fee = expected_prevout
        .value
        .to_sat()
        .checked_sub(output_sum(&psbt.unsigned_tx)?)
        .ok_or(PolicyError::InvalidPolicy(
            "transaction outputs exceed input",
        ))?;
    if fee != manifest_tx.fee_sats {
        return invalid("transaction fee does not match manifest");
    }
    Ok(())
}

pub(crate) fn verify_phone_signature(
    psbt: &Psbt,
    leaf: &crate::policy::VaultLeaf,
    phone: XOnlyPublicKey,
) -> Result<(), PolicyError> {
    let secp = Secp256k1::verification_only();
    for index in 0..psbt.inputs.len() {
        let signature = psbt.inputs[index]
            .tap_script_sigs
            .get(&(phone, leaf.leaf_hash))
            .ok_or(PolicyError::InvalidPolicy("phone signature missing"))?;
        if signature.sighash_type != TapSighashType::Default {
            return invalid("phone signature uses non-default sighash");
        }
        let unsigned_tx = psbt.unsigned_tx.clone();
        let mut cache = SighashCache::new(&unsigned_tx);
        let message = match psbt
            .sighash_msg(index, &mut cache, Some(leaf.leaf_hash))
            .map_err(|_| PolicyError::InvalidPolicy("unable to compute Taproot sighash"))?
        {
            PsbtSighashMsg::TapSighash(sighash) => Message::from_digest(sighash.to_byte_array()),
            _ => return invalid("PSBT did not produce Taproot sighash"),
        };
        secp.verify_schnorr(&signature.signature, &message, &phone)
            .map_err(|_| PolicyError::InvalidPolicy("invalid phone signature"))?;
    }
    Ok(())
}

fn psbt_input_sum(psbt: &Psbt) -> Result<u64, PolicyError> {
    if psbt.inputs.len() != psbt.unsigned_tx.input.len() {
        return invalid("PSBT input metadata count mismatch");
    }
    psbt.inputs.iter().try_fold(0_u64, |sum, input| {
        let value = input
            .witness_utxo
            .as_ref()
            .ok_or(PolicyError::InvalidPolicy("PSBT witness UTXO missing"))?
            .value
            .to_sat();
        sum.checked_add(value)
            .ok_or(PolicyError::InvalidPolicy("input amount overflow"))
    })
}

fn output_sum(transaction: &Transaction) -> Result<u64, PolicyError> {
    transaction.output.iter().try_fold(0_u64, |sum, output| {
        sum.checked_add(output.value.to_sat())
            .ok_or(PolicyError::InvalidPolicy("output amount overflow"))
    })
}

fn get_psbt<'a>(psbts: &'a BTreeMap<String, Psbt>, path: &str) -> Result<&'a Psbt, PolicyError> {
    psbts.get(path).ok_or(PolicyError::PsbtSetMismatch)
}

fn parse_network(network: &str) -> Result<Network, PolicyError> {
    match network {
        "regtest" => Ok(Network::Regtest),
        "bitcoin" => Ok(Network::Bitcoin),
        _ => invalid("unsupported network"),
    }
}

fn invalid<T>(message: &'static str) -> Result<T, PolicyError> {
    Err(PolicyError::InvalidPolicy(message))
}
