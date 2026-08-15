// Phone-key rotation handling follows Luke Childs' Anzen src/core/recovery.rs,
// src/core/crypto.rs, and src/core/social.rs at
// 01794bb14d34d01413e3b539c7e5496ecbce0c87 under the MIT license.

use crate::{
    policy::VaultPolicy, AnzenIdentity, CooperativeSweepPackage, PolicyError, PolicyPackage,
    MAX_PACKAGE_BYTES,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use bip39::{Language, Mnemonic};
use bitcoin::{
    bip32::{DerivationPath, Xpriv},
    secp256k1::{Secp256k1, XOnlyPublicKey},
    Network, OutPoint, Psbt,
};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use hkdf::Hkdf;
use pgp::{
    composed::{ArmorOptions, Deserializable, MessageBuilder, SignedPublicKey},
    crypto::sym::SymmetricKeyAlgorithm,
    types::{KeyDetails, PublicKeyTrait},
};
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use zeroize::{Zeroize, Zeroizing};

const ROTATION_PACKAGE_VERSION: u8 = 1;
const ROTATION_PACKAGE_KIND: &str = "phone-key-rotation";
const PAYLOAD_PURPOSE: &str = "cloud/vault-recovery-payload/v1";
const HWW_KEY_PURPOSE: &str = "cloud/vault-recovery-key/hww/v1";
const FRIEND_MANIFEST_PURPOSE: &str = "cloud/vault-recovery-friends/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub version: u8,
    pub network: String,
    pub phone_vault_pubkey: String,
    pub hww_vault_pubkey: String,
    pub phone_hot_external_descriptor: String,
    pub phone_hot_internal_descriptor: String,
    pub vault_descriptor: String,
    pub vault_address: String,
    pub phone_recovery_blocks: u16,
    pub hww_recovery_blocks: u16,
    #[serde(default, alias = "hard_limit_sats")]
    pub monthly_limit_sats: u64,
    #[serde(default)]
    pub emergency_access_limit_sats: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeviceFile {
    pub kind: String,
    pub network: String,
    pub mnemonic: String,
    #[serde(default)]
    pub vault_key_index: u32,
}

impl Drop for DeviceFile {
    fn drop(&mut self) {
        self.mnemonic.zeroize();
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub version: u8,
    pub purpose: String,
    pub nonce: [u8; 24],
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendKeyWrapper {
    pub fingerprint: String,
    pub public_key_armored: String,
    pub encrypted_symmetric_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CloudRecoveryBackup {
    pub version: u8,
    pub kind: String,
    pub encrypted_payload: EncryptedBlob,
    pub hww_encrypted_symmetric_key: EncryptedBlob,
    pub friends: Vec<FriendKeyWrapper>,
    pub encrypted_friend_manifest: EncryptedBlob,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryPayload {
    pub version: u8,
    pub kind: String,
    pub network: String,
    pub phone_mnemonic: String,
    #[serde(default)]
    pub phone_vault_key_index: u32,
    pub phone_vault_pubkey: String,
    pub vault_descriptor: String,
    pub vault_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhoneRecoveryPackage {
    pub version: u8,
    pub kind: String,
    pub phone_mnemonic: String,
    pub phone_vault_key_index: u32,
    pub phone_vault_pubkey: String,
    pub vault_descriptor: String,
    pub vault_address: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneBackupSummary {
    pub network: String,
    pub vault_address: String,
    pub recovery_friend_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryFriendSummary {
    pub fingerprint: String,
    pub network: String,
    pub vault_address: String,
    pub current_friend_count: usize,
}

#[derive(Clone)]
pub struct ReviewedPhoneBackup {
    backup: CloudRecoveryBackup,
    config: VaultConfig,
    network: Network,
    summary: PhoneBackupSummary,
}

#[derive(Clone)]
pub struct ReviewedRecoveryFriend {
    backup: CloudRecoveryBackup,
    config: VaultConfig,
    public_key: Vec<u8>,
    summary: RecoveryFriendSummary,
}

pub struct ApprovedRecoveryFriend {
    backup: CloudRecoveryBackup,
    fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhoneRotationPackage {
    pub version: u8,
    pub kind: String,
    pub old_vault_descriptor: String,
    pub new_phone_vault_pubkey: String,
    pub new_vault_descriptor: String,
    pub new_vault_address: String,
    pub monthly_limit_sats: u64,
    #[serde(default)]
    pub emergency_access_limit_sats: u64,
    pub sweep: CooperativeSweepPackage,
    pub renewed_policy: Option<PolicyPackage>,
    pub cloud_recovery_backup: Option<CloudRecoveryBackup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhoneRotationSummary {
    pub network: String,
    pub new_phone_vault_pubkey: String,
    pub new_vault_address: String,
    pub input_count: usize,
    pub sent_sats: u64,
    pub fee_sats: u64,
    pub monthly_limit_sats: u64,
    pub emergency_access_limit_sats: u64,
    pub renews_policy: bool,
    pub recovery_friend_count: usize,
    pub policy_psbt_count: usize,
}

#[derive(Clone)]
pub struct ReviewedPhoneRotation {
    package: PhoneRotationPackage,
    current: VaultConfig,
    pending: DeviceFile,
    backup: CloudRecoveryBackup,
    network: Network,
    new_phone_pubkey: XOnlyPublicKey,
}

pub struct ApprovedPhoneRotation {
    package: PhoneRotationPackage,
    sweep_signature_count: usize,
    policy_signature_count: usize,
}

impl VaultConfig {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        parse_bounded(bytes)
    }

    fn network(&self) -> Result<Network, PolicyError> {
        parse_network(&self.network)
    }
}

impl DeviceFile {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        parse_bounded(bytes)
    }
}

impl CloudRecoveryBackup {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        let backup: Self = parse_bounded(bytes)?;
        if backup.version != 1 || backup.kind != "vault-cloud-recovery" {
            return Err(PolicyError::UnsupportedPackage);
        }
        Ok(backup)
    }

    pub fn review_for_recovery(
        self,
        config: VaultConfig,
    ) -> Result<ReviewedPhoneBackup, PolicyError> {
        if config.version != 1 {
            return Err(PolicyError::UnsupportedPackage);
        }
        let network = config.network()?;
        let summary = PhoneBackupSummary {
            network: config.network.clone(),
            vault_address: config.vault_address.clone(),
            recovery_friend_count: self.friends.len(),
        };
        Ok(ReviewedPhoneBackup {
            backup: self,
            config,
            network,
            summary,
        })
    }

    pub fn review_friend_enrollment(
        self,
        config: VaultConfig,
        public_key: &[u8],
    ) -> Result<ReviewedRecoveryFriend, PolicyError> {
        if config.version != 1 {
            return Err(PolicyError::UnsupportedPackage);
        }
        parse_network(&config.network)?;
        let public = parse_friend_public_key(public_key)?;
        let fingerprint = public.fingerprint().to_string();
        Ok(ReviewedRecoveryFriend {
            summary: RecoveryFriendSummary {
                fingerprint,
                network: config.network.clone(),
                vault_address: config.vault_address.clone(),
                current_friend_count: self.friends.len(),
            },
            backup: self,
            config,
            public_key: public_key.to_vec(),
        })
    }
}

impl ReviewedRecoveryFriend {
    pub fn summary(&self) -> &RecoveryFriendSummary {
        &self.summary
    }

    pub fn approve(self, identity: &AnzenIdentity) -> Result<ApprovedRecoveryFriend, PolicyError> {
        if identity.public_key().to_string() != self.config.hww_vault_pubkey {
            return invalid("HWW key does not match the configured vault policy");
        }
        let payload = decrypt_backup(&self.backup, identity.hww_seed())?;
        validate_payload(&payload, &self.config, self.config.network()?)?;
        let symmetric_key = decrypt_blob(
            identity.hww_seed(),
            HWW_KEY_PURPOSE,
            &self.backup.hww_encrypted_symmetric_key,
        )?;
        if symmetric_key.len() != 32 {
            return invalid("cloud backup contains an invalid symmetric key");
        }
        validate_friend_manifest(&self.backup, &symmetric_key)?;
        let wrapper = wrap_for_friend(&self.public_key, &symmetric_key)?;
        if self
            .backup
            .friends
            .iter()
            .any(|friend| friend.fingerprint == wrapper.fingerprint)
        {
            return invalid("recovery friend is already configured");
        }
        let fingerprint = wrapper.fingerprint.clone();
        let mut backup = self.backup;
        backup.friends.push(wrapper);
        backup
            .friends
            .sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
        let friend_bytes =
            serde_json::to_vec(&backup.friends).map_err(|_| PolicyError::Serialization)?;
        backup.encrypted_friend_manifest =
            encrypt_blob(&symmetric_key, FRIEND_MANIFEST_PURPOSE, &friend_bytes)?;
        Ok(ApprovedRecoveryFriend {
            backup,
            fingerprint,
        })
    }
}

impl ApprovedRecoveryFriend {
    pub fn backup(&self) -> &CloudRecoveryBackup {
        &self.backup
    }

    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    pub fn to_json(&self) -> Result<Vec<u8>, PolicyError> {
        let mut json =
            serde_json::to_vec_pretty(&self.backup).map_err(|_| PolicyError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}

impl ReviewedPhoneBackup {
    pub fn summary(&self) -> &PhoneBackupSummary {
        &self.summary
    }

    pub fn approve(self, identity: &AnzenIdentity) -> Result<PhoneRecoveryPackage, PolicyError> {
        if identity.public_key().to_string() != self.config.hww_vault_pubkey {
            return invalid("HWW key does not match the configured vault policy");
        }
        let payload = decrypt_backup(&self.backup, identity.hww_seed())?;
        validate_payload(&payload, &self.config, self.network)?;
        Ok(PhoneRecoveryPackage {
            version: 2,
            kind: "phone-recovery".to_owned(),
            phone_mnemonic: payload.phone_mnemonic,
            phone_vault_key_index: payload.phone_vault_key_index,
            phone_vault_pubkey: payload.phone_vault_pubkey,
            vault_descriptor: payload.vault_descriptor,
            vault_address: payload.vault_address,
        })
    }
}

impl PhoneRecoveryPackage {
    pub fn to_json(&self) -> Result<Vec<u8>, PolicyError> {
        let mut json = serde_json::to_vec_pretty(self).map_err(|_| PolicyError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}

impl PhoneRotationPackage {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        let package: Self = parse_bounded(bytes)?;
        if package.version != ROTATION_PACKAGE_VERSION || package.kind != ROTATION_PACKAGE_KIND {
            return Err(PolicyError::UnsupportedPackage);
        }
        Ok(package)
    }

    pub fn review(
        self,
        current: VaultConfig,
        pending: DeviceFile,
        backup: CloudRecoveryBackup,
    ) -> Result<ReviewedPhoneRotation, PolicyError> {
        if self.cloud_recovery_backup.is_some() {
            return invalid("rotation proposal already contains a renewed backup");
        }
        let network = current.network()?;
        if current.version != 1
            || pending.kind != "pending-phone-rotation"
            || parse_network(&pending.network)? != network
            || self.old_vault_descriptor != current.vault_descriptor
            || self.sweep.vault_descriptor != current.vault_descriptor
            || self.sweep.destination != self.new_vault_address
            || self.monthly_limit_sats != current.monthly_limit_sats
            || self.emergency_access_limit_sats != current.emergency_access_limit_sats
        {
            return invalid("phone rotation package does not match the current vault");
        }
        let new_phone_pubkey = derive_vault_pubkey(&pending, network)?;
        if new_phone_pubkey.to_string() != self.new_phone_vault_pubkey {
            return invalid("pending phone key does not match the rotation proposal");
        }
        let hww_pubkey = XOnlyPublicKey::from_str(&current.hww_vault_pubkey)
            .map_err(|_| PolicyError::InvalidPolicy("invalid configured HWW key"))?;
        let new_policy = VaultPolicy::new(new_phone_pubkey, hww_pubkey, network);
        if new_policy.descriptor_string() != self.new_vault_descriptor
            || new_policy.address.to_string() != self.new_vault_address
        {
            return invalid("phone rotation destination does not match the proposed new keys");
        }
        validate_renewed_policy_binding(&self, &current)?;
        Ok(ReviewedPhoneRotation {
            package: self,
            current,
            pending,
            backup,
            network,
            new_phone_pubkey,
        })
    }

    pub fn summary(&self) -> Result<PhoneRotationSummary, PolicyError> {
        let network = self.sweep.summary()?.network;
        Ok(PhoneRotationSummary {
            network,
            new_phone_vault_pubkey: self.new_phone_vault_pubkey.clone(),
            new_vault_address: self.new_vault_address.clone(),
            input_count: self.sweep.input_count,
            sent_sats: self.sweep.sent_sats,
            fee_sats: self.sweep.fee_sats,
            monthly_limit_sats: self.monthly_limit_sats,
            emergency_access_limit_sats: self.emergency_access_limit_sats,
            renews_policy: self.renewed_policy.is_some(),
            recovery_friend_count: 0,
            policy_psbt_count: self
                .renewed_policy
                .as_ref()
                .map_or(0, |policy| policy.psbts.len()),
        })
    }
}

impl ReviewedPhoneRotation {
    pub fn summary(&self) -> PhoneRotationSummary {
        let mut summary = self.package.summary().expect("reviewed rotation envelope");
        summary.recovery_friend_count = self.backup.friends.len();
        summary
    }

    pub fn approve(
        mut self,
        identity: &AnzenIdentity,
    ) -> Result<ApprovedPhoneRotation, PolicyError> {
        if identity.public_key().to_string() != self.current.hww_vault_pubkey {
            return invalid("HWW key does not match the configured vault policy");
        }
        let current_payload = decrypt_backup(&self.backup, identity.hww_seed())?;
        validate_payload(&current_payload, &self.current, self.network)?;

        let approved_sweep = self
            .package
            .sweep
            .clone()
            .validate(identity.public_key())?
            .approve(identity)?;
        let sweep_signature_count = approved_sweep.hww_signature_count();
        self.package.sweep = approved_sweep.into_package();

        let policy_signature_count = if let Some(renewed) = self.package.renewed_policy.take() {
            let approved = renewed.validate(identity.public_key())?.approve(identity)?;
            let count = approved.hww_signature_count();
            self.package.renewed_policy = Some(approved.into_package());
            count
        } else {
            0
        };

        let new_payload = RecoveryPayload {
            version: 1,
            kind: "vault-recovery-payload".to_owned(),
            network: self.current.network.clone(),
            phone_mnemonic: self.pending.mnemonic.clone(),
            phone_vault_key_index: self.pending.vault_key_index,
            phone_vault_pubkey: self.new_phone_pubkey.to_string(),
            vault_descriptor: self.package.new_vault_descriptor.clone(),
            vault_address: self.package.new_vault_address.clone(),
        };
        self.package.cloud_recovery_backup = Some(rotate_backup(
            &self.backup,
            identity.hww_seed(),
            &new_payload,
        )?);
        Ok(ApprovedPhoneRotation {
            package: self.package,
            sweep_signature_count,
            policy_signature_count,
        })
    }
}

impl ApprovedPhoneRotation {
    pub fn sweep_signature_count(&self) -> usize {
        self.sweep_signature_count
    }

    pub fn policy_signature_count(&self) -> usize {
        self.policy_signature_count
    }

    pub fn to_json(&self) -> Result<Vec<u8>, PolicyError> {
        let mut json =
            serde_json::to_vec_pretty(&self.package).map_err(|_| PolicyError::Serialization)?;
        json.push(b'\n');
        Ok(json)
    }
}

fn validate_renewed_policy_binding(
    package: &PhoneRotationPackage,
    current: &VaultConfig,
) -> Result<(), PolicyError> {
    let enabled = current.monthly_limit_sats > 0 || current.emergency_access_limit_sats > 0;
    match (enabled, package.renewed_policy.as_ref()) {
        (false, None) => return Ok(()),
        (false, Some(_)) => {
            return invalid("disabled vault policy must not create renewed transactions")
        }
        (true, None) => return invalid("phone rotation is missing the renewed vault policy"),
        (true, Some(_)) => {}
    }
    let renewed = package.renewed_policy.as_ref().expect("matched above");
    if !renewed.manifest.phone_approved
        || renewed.manifest.hww_approved
        || renewed.manifest.vault_descriptor != package.new_vault_descriptor
        || renewed.manifest.vault_address != package.new_vault_address
        || renewed.manifest.monthly_limit_sats != current.monthly_limit_sats
        || renewed.manifest.emergency_access_limit_sats != current.emergency_access_limit_sats
    {
        return invalid("renewed vault policy does not preserve the active policy");
    }
    let sweep = Psbt::from_str(&package.sweep.psbt).map_err(|_| PolicyError::InvalidPsbt)?;
    let sweep_output = sweep
        .unsigned_tx
        .output
        .first()
        .ok_or(PolicyError::InvalidPolicy("rotation sweep has no output"))?;
    let rollover_text = renewed
        .psbts
        .get(&renewed.manifest.rollover.psbt_file)
        .ok_or(PolicyError::PsbtSetMismatch)?;
    let rollover = Psbt::from_str(rollover_text).map_err(|_| PolicyError::InvalidPsbt)?;
    if rollover.unsigned_tx.input.len() != 1
        || rollover.inputs.len() != 1
        || rollover.unsigned_tx.input[0].previous_output
            != OutPoint::new(sweep.unsigned_tx.compute_txid(), 0)
        || rollover.inputs[0].witness_utxo.as_ref() != Some(sweep_output)
    {
        return invalid("renewed vault policy is not chained to the rotation sweep");
    }
    Ok(())
}

fn derive_vault_pubkey(file: &DeviceFile, network: Network) -> Result<XOnlyPublicKey, PolicyError> {
    let mnemonic = Mnemonic::parse_in_normalized(Language::English, &file.mnemonic)
        .map_err(|_| PolicyError::KeyDerivation)?;
    let mut seed = mnemonic.to_seed_normalized("");
    let result = (|| {
        let coin_type = match network {
            Network::Bitcoin => 0,
            Network::Regtest => 1,
            _ => return Err(PolicyError::KeyDerivation),
        };
        let secp = Secp256k1::new();
        let master = Xpriv::new_master(network, &seed).map_err(|_| PolicyError::KeyDerivation)?;
        let path = DerivationPath::from_str(&format!(
            "m/86'/{coin_type}'/100'/0/{}",
            file.vault_key_index
        ))
        .map_err(|_| PolicyError::KeyDerivation)?;
        let vault = master
            .derive_priv(&secp, &path)
            .map_err(|_| PolicyError::KeyDerivation)?;
        let keypair = bitcoin::secp256k1::Keypair::from_secret_key(&secp, &vault.private_key);
        Ok(keypair.x_only_public_key().0)
    })();
    seed.zeroize();
    result
}

fn validate_payload(
    payload: &RecoveryPayload,
    config: &VaultConfig,
    network: Network,
) -> Result<(), PolicyError> {
    if payload.version != 1
        || payload.kind != "vault-recovery-payload"
        || payload.network != config.network
        || payload.phone_vault_pubkey != config.phone_vault_pubkey
        || payload.vault_descriptor != config.vault_descriptor
        || payload.vault_address != config.vault_address
    {
        return invalid("recovery payload does not match the configured vault");
    }
    let file = DeviceFile {
        kind: "phone".to_owned(),
        network: config.network.clone(),
        mnemonic: payload.phone_mnemonic.clone(),
        vault_key_index: payload.phone_vault_key_index,
    };
    if derive_vault_pubkey(&file, network)?.to_string() != payload.phone_vault_pubkey {
        return invalid("recovery payload mnemonic does not match its phone public key");
    }
    Ok(())
}

fn rotate_backup(
    backup: &CloudRecoveryBackup,
    hww_seed: &[u8],
    payload: &RecoveryPayload,
) -> Result<CloudRecoveryBackup, PolicyError> {
    let existing_key = decrypt_blob(
        hww_seed,
        HWW_KEY_PURPOSE,
        &backup.hww_encrypted_symmetric_key,
    )?;
    validate_friend_manifest(backup, &existing_key)?;
    let payload_bytes = serde_json::to_vec(payload).map_err(|_| PolicyError::Serialization)?;
    let symmetric_key = derive_rotation_key(hww_seed, backup, &payload_bytes)?;
    let mut friends = Vec::with_capacity(backup.friends.len());
    for friend in &backup.friends {
        friends.push(wrap_for_friend(
            friend.public_key_armored.as_bytes(),
            &symmetric_key,
        )?);
    }
    friends.sort_by(|left, right| left.fingerprint.cmp(&right.fingerprint));
    friends.dedup_by(|left, right| left.fingerprint == right.fingerprint);
    let friend_bytes = serde_json::to_vec(&friends).map_err(|_| PolicyError::Serialization)?;
    Ok(CloudRecoveryBackup {
        version: 1,
        kind: "vault-cloud-recovery".to_owned(),
        encrypted_payload: encrypt_blob(&symmetric_key, PAYLOAD_PURPOSE, &payload_bytes)?,
        hww_encrypted_symmetric_key: encrypt_blob(hww_seed, HWW_KEY_PURPOSE, &symmetric_key)?,
        friends,
        encrypted_friend_manifest: encrypt_blob(
            &symmetric_key,
            FRIEND_MANIFEST_PURPOSE,
            &friend_bytes,
        )?,
    })
}

fn parse_friend_public_key(public_key: &[u8]) -> Result<SignedPublicKey, PolicyError> {
    let (public, _) = SignedPublicKey::from_reader_single(public_key)
        .map_err(|_| PolicyError::InvalidPolicy("invalid recovery-friend OpenPGP public key"))?;
    public.verify().map_err(|_| {
        PolicyError::InvalidPolicy("recovery-friend OpenPGP self-signature is invalid")
    })?;
    if !public
        .public_subkeys
        .iter()
        .any(|subkey| subkey.is_encryption_key())
    {
        return invalid("recovery-friend OpenPGP key has no encryption-capable subkey");
    }
    Ok(public)
}

fn wrap_for_friend(
    public_key: &[u8],
    symmetric_key: &[u8],
) -> Result<FriendKeyWrapper, PolicyError> {
    let public = parse_friend_public_key(public_key)?;
    let encryption_subkey = public
        .public_subkeys
        .iter()
        .find(|subkey| subkey.is_encryption_key())
        .ok_or(PolicyError::InvalidPolicy(
            "recovery-friend OpenPGP key has no encryption-capable subkey",
        ))?;
    let mut builder = MessageBuilder::from_bytes("vault-recovery-key", symmetric_key.to_vec())
        .seipd_v1(thread_rng(), SymmetricKeyAlgorithm::AES256);
    builder
        .encrypt_to_key(thread_rng(), encryption_subkey)
        .map_err(|_| PolicyError::Serialization)?;
    let encrypted = builder
        .to_vec(thread_rng())
        .map_err(|_| PolicyError::Serialization)?;
    Ok(FriendKeyWrapper {
        fingerprint: public.fingerprint().to_string(),
        public_key_armored: public
            .to_armored_string(ArmorOptions::default())
            .map_err(|_| PolicyError::Serialization)?,
        encrypted_symmetric_key: STANDARD.encode(encrypted),
    })
}

fn decrypt_backup(
    backup: &CloudRecoveryBackup,
    hww_seed: &[u8],
) -> Result<RecoveryPayload, PolicyError> {
    if backup.version != 1 || backup.kind != "vault-cloud-recovery" {
        return Err(PolicyError::UnsupportedPackage);
    }
    let key = decrypt_blob(
        hww_seed,
        HWW_KEY_PURPOSE,
        &backup.hww_encrypted_symmetric_key,
    )?;
    validate_friend_manifest(backup, &key)?;
    let payload = decrypt_blob(&key, PAYLOAD_PURPOSE, &backup.encrypted_payload)?;
    serde_json::from_slice(&payload).map_err(|_| PolicyError::InvalidJson)
}

fn validate_friend_manifest(backup: &CloudRecoveryBackup, key: &[u8]) -> Result<(), PolicyError> {
    let authenticated = decrypt_blob(
        key,
        FRIEND_MANIFEST_PURPOSE,
        &backup.encrypted_friend_manifest,
    )?;
    let expected = serde_json::to_vec(&backup.friends).map_err(|_| PolicyError::Serialization)?;
    if authenticated.as_slice() != expected {
        return invalid("cloud recovery friend list failed authentication");
    }
    Ok(())
}

fn encrypt_blob(
    seed: &[u8],
    purpose: &str,
    plaintext: &[u8],
) -> Result<EncryptedBlob, PolicyError> {
    let key = derive_key(seed, purpose)?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let mut nonce = [0_u8; 24];
    let digest = sha2::Sha256::digest(plaintext);
    let hk = Hkdf::<Sha256>::new(Some(b"renewable-bitcoin-vault/mvp/v1/nonce"), seed);
    let mut info = Vec::with_capacity(purpose.len() + digest.len());
    info.extend_from_slice(purpose.as_bytes());
    info.extend_from_slice(&digest);
    hk.expand(&info, &mut nonce)
        .map_err(|_| PolicyError::KeyDerivation)?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), plaintext)
        .map_err(|_| PolicyError::Serialization)?;
    Ok(EncryptedBlob {
        version: 1,
        purpose: purpose.to_owned(),
        nonce,
        ciphertext,
    })
}

fn derive_rotation_key(
    hww_seed: &[u8],
    backup: &CloudRecoveryBackup,
    payload_bytes: &[u8],
) -> Result<Zeroizing<Vec<u8>>, PolicyError> {
    let mut transcript = Sha256::new();
    transcript.update(b"cloud/vault-recovery-rotation-key/v1");
    transcript.update(payload_bytes);
    transcript.update(backup.encrypted_payload.nonce);
    transcript.update(&backup.encrypted_payload.ciphertext);
    let digest = transcript.finalize();
    let hk = Hkdf::<Sha256>::new(
        Some(b"renewable-bitcoin-vault/mvp/v1/rotation-key"),
        hww_seed,
    );
    let mut key = Zeroizing::new(vec![0_u8; 32]);
    hk.expand(&digest, key.as_mut_slice())
        .map_err(|_| PolicyError::KeyDerivation)?;
    Ok(key)
}

fn decrypt_blob(
    seed: &[u8],
    purpose: &str,
    blob: &EncryptedBlob,
) -> Result<Zeroizing<Vec<u8>>, PolicyError> {
    if blob.version != 1 || blob.purpose != purpose {
        return invalid("encrypted blob version or purpose mismatch");
    }
    let key = derive_key(seed, purpose)?;
    let cipher = XChaCha20Poly1305::new((&*key).into());
    let plaintext = cipher
        .decrypt(XNonce::from_slice(&blob.nonce), blob.ciphertext.as_ref())
        .map_err(|_| PolicyError::InvalidPolicy("authentication or decryption failed"))?;
    Ok(Zeroizing::new(plaintext))
}

fn derive_key(seed: &[u8], purpose: &str) -> Result<Zeroizing<[u8; 32]>, PolicyError> {
    let hk = Hkdf::<Sha256>::new(Some(b"renewable-bitcoin-vault/mvp/v1"), seed);
    let mut key = Zeroizing::new([0_u8; 32]);
    hk.expand(purpose.as_bytes(), key.as_mut())
        .map_err(|_| PolicyError::KeyDerivation)?;
    Ok(key)
}

fn parse_bounded<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, PolicyError> {
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(PolicyError::PackageTooLarge);
    }
    serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)
}

fn parse_network(name: &str) -> Result<Network, PolicyError> {
    match name {
        "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
        "regtest" => Ok(Network::Regtest),
        _ => invalid("unsupported Bitcoin network"),
    }
}

fn invalid<T>(message: &'static str) -> Result<T, PolicyError> {
    Err(PolicyError::InvalidPolicy(message))
}

#[cfg(test)]
mod phone_backup_tests {
    use super::*;
    use pgp::{
        composed::{
            ArmorOptions, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
        },
        crypto::ecc_curve::ECCCurve,
        types::Password,
    };
    use rand::thread_rng;

    const PHONE_MNEMONIC: &str = "drastic bamboo mountain loyal category cancel animal embark drastic bamboo mountain loyal category cancel animal embark drastic bamboo mountain loyal category cancel animal embark";

    fn fixture() -> (CloudRecoveryBackup, VaultConfig, AnzenIdentity) {
        let identity = AnzenIdentity::from_app_seed(&[0x42; 32], Network::Regtest).unwrap();
        let phone = DeviceFile {
            kind: "phone".into(),
            network: "regtest".into(),
            mnemonic: PHONE_MNEMONIC.into(),
            vault_key_index: 0,
        };
        let phone_pubkey = derive_vault_pubkey(&phone, Network::Regtest)
            .unwrap()
            .to_string();
        let config = VaultConfig {
            version: 1,
            network: "regtest".into(),
            phone_vault_pubkey: phone_pubkey.clone(),
            hww_vault_pubkey: identity.public_key().to_string(),
            phone_hot_external_descriptor: "external".into(),
            phone_hot_internal_descriptor: "internal".into(),
            vault_descriptor: "descriptor".into(),
            vault_address: "bcrt1ptest".into(),
            phone_recovery_blocks: 61_200,
            hww_recovery_blocks: 65_535,
            monthly_limit_sats: 0,
            emergency_access_limit_sats: 0,
        };
        let payload = RecoveryPayload {
            version: 1,
            kind: "vault-recovery-payload".into(),
            network: "regtest".into(),
            phone_mnemonic: PHONE_MNEMONIC.into(),
            phone_vault_key_index: 0,
            phone_vault_pubkey: phone_pubkey,
            vault_descriptor: config.vault_descriptor.clone(),
            vault_address: config.vault_address.clone(),
        };
        let symmetric_key = [7_u8; 32];
        let friend_bytes = serde_json::to_vec(&Vec::<FriendKeyWrapper>::new()).unwrap();
        let backup = CloudRecoveryBackup {
            version: 1,
            kind: "vault-cloud-recovery".into(),
            encrypted_payload: encrypt_blob(
                &symmetric_key,
                PAYLOAD_PURPOSE,
                &serde_json::to_vec(&payload).unwrap(),
            )
            .unwrap(),
            hww_encrypted_symmetric_key: encrypt_blob(
                identity.hww_seed(),
                HWW_KEY_PURPOSE,
                &symmetric_key,
            )
            .unwrap(),
            friends: vec![],
            encrypted_friend_manifest: encrypt_blob(
                &symmetric_key,
                FRIEND_MANIFEST_PURPOSE,
                &friend_bytes,
            )
            .unwrap(),
        };
        (backup, config, identity)
    }

    fn friend_public_key() -> String {
        let mut encryption_subkey = SubkeyParamsBuilder::default();
        encryption_subkey
            .key_type(KeyType::ECDH(ECCCurve::Curve25519))
            .can_sign(false)
            .can_encrypt(true)
            .can_authenticate(false);
        let mut params = SecretKeyParamsBuilder::default();
        params
            .key_type(KeyType::Ed25519Legacy)
            .can_certify(true)
            .can_sign(false)
            .can_encrypt(false)
            .primary_user_id("Prime parity friend <friend@example.test>".to_owned())
            .subkeys(vec![encryption_subkey.build().unwrap()]);
        let secret = params
            .build()
            .unwrap()
            .generate(thread_rng())
            .unwrap()
            .sign(&mut thread_rng(), &Password::empty())
            .unwrap();
        SignedPublicKey::from(secret)
            .to_armored_string(ArmorOptions::default())
            .unwrap()
    }

    #[test]
    fn authenticated_backup_exports_lukes_version_two_recovery_package() {
        let (backup, config, identity) = fixture();
        let package = backup
            .review_for_recovery(config)
            .unwrap()
            .approve(&identity)
            .unwrap();
        assert_eq!(package.version, 2);
        assert_eq!(package.kind, "phone-recovery");
        assert_eq!(package.phone_mnemonic, PHONE_MNEMONIC);
    }

    #[test]
    fn wrong_hww_identity_and_tampered_manifest_fail_closed() {
        let (backup, config, _) = fixture();
        let wrong = AnzenIdentity::from_app_seed(&[0x43; 32], Network::Regtest).unwrap();
        assert!(backup
            .clone()
            .review_for_recovery(config.clone())
            .unwrap()
            .approve(&wrong)
            .is_err());
        let (mut backup, config, identity) = fixture();
        backup.encrypted_friend_manifest.ciphertext[0] ^= 1;
        assert!(backup
            .review_for_recovery(config)
            .unwrap()
            .approve(&identity)
            .is_err());
    }

    #[test]
    fn recovery_friend_enrollment_matches_lukes_authenticated_set_rules() {
        let public_key = friend_public_key();
        let (backup, config, identity) = fixture();
        let reviewed = backup
            .review_friend_enrollment(config.clone(), public_key.as_bytes())
            .unwrap();
        assert_eq!(reviewed.summary().current_friend_count, 0);
        assert!(!reviewed.summary().fingerprint.is_empty());

        let approved = reviewed.approve(&identity).unwrap();
        assert_eq!(approved.backup().friends.len(), 1);
        assert_eq!(
            approved.backup().friends[0].fingerprint,
            approved.fingerprint()
        );

        assert!(approved
            .backup()
            .clone()
            .review_friend_enrollment(config, public_key.as_bytes())
            .unwrap()
            .approve(&identity)
            .is_err());
    }

    #[test]
    fn invalid_key_wrong_hww_and_tampered_manifest_never_enroll_a_friend() {
        let public_key = friend_public_key();
        let (backup, config, _) = fixture();
        assert!(backup
            .clone()
            .review_friend_enrollment(config.clone(), b"not an OpenPGP key")
            .is_err());

        let wrong = AnzenIdentity::from_app_seed(&[0x43; 32], Network::Regtest).unwrap();
        assert!(backup
            .clone()
            .review_friend_enrollment(config.clone(), public_key.as_bytes())
            .unwrap()
            .approve(&wrong)
            .is_err());

        let (mut backup, config, identity) = fixture();
        backup.encrypted_friend_manifest.ciphertext[0] ^= 1;
        assert!(backup
            .review_friend_enrollment(config, public_key.as_bytes())
            .unwrap()
            .approve(&identity)
            .is_err());
    }

    #[test]
    fn rotation_uses_a_fresh_key_and_rewraps_every_enrolled_friend() {
        let public_key = friend_public_key();
        let (backup, config, identity) = fixture();
        let approved = backup
            .review_friend_enrollment(config, public_key.as_bytes())
            .unwrap()
            .approve(&identity)
            .unwrap();
        let before = approved.backup().clone();
        let mut payload = decrypt_backup(&before, identity.hww_seed()).unwrap();
        payload.phone_vault_key_index = 1;
        let rotated = rotate_backup(&before, identity.hww_seed(), &payload).unwrap();

        assert_eq!(rotated.friends.len(), 1);
        assert_eq!(
            rotated.friends[0].fingerprint,
            before.friends[0].fingerprint
        );
        assert_ne!(
            rotated.friends[0].encrypted_symmetric_key,
            before.friends[0].encrypted_symmetric_key
        );
        assert_ne!(
            rotated.hww_encrypted_symmetric_key.ciphertext,
            before.hww_encrypted_symmetric_key.ciphertext
        );
        assert_eq!(
            decrypt_backup(&rotated, identity.hww_seed()).unwrap(),
            payload
        );
    }
}
