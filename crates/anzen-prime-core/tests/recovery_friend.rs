use anzen_prime_core::{AppSeedSource, ApprovedPackageSink, ReviewedRecoveryFriendRequest};
use pgp::{
    composed::{
        ArmorOptions, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
    },
    crypto::ecc_curve::ECCCurve,
    types::Password,
};
use rand::thread_rng;

struct Seed {
    calls: usize,
}

impl AppSeedSource for Seed {
    type Error = ();

    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        self.calls += 1;
        Ok([0x42; 32])
    }
}

struct Sink {
    writes: usize,
}

impl ApprovedPackageSink for Sink {
    type Error = ();

    fn write_temporary(&mut self, _: &[u8]) -> Result<(), ()> {
        self.writes += 1;
        Ok(())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        self.writes += 1;
        Ok(())
    }
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
        .primary_user_id("Prime flow friend <friend@example.test>".to_owned())
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
fn imports_for_review_without_seed_and_failed_hww_authentication_writes_nothing() {
    let key = friend_public_key();
    let reviewed = ReviewedRecoveryFriendRequest::import(
        include_bytes!("../../../fixtures/phone-backup-v1/review-only-backup.json"),
        include_bytes!("../../../fixtures/phone-backup-v1/regtest-config.json"),
        key.as_bytes(),
    )
    .unwrap();
    let mut seed = Seed { calls: 0 };
    let mut sink = Sink { writes: 0 };

    assert_eq!(reviewed.summary().current_friend_count, 0);
    assert_eq!(seed.calls, 0);
    assert!(reviewed.approve_and_export(&mut seed, &mut sink).is_err());
    assert_eq!(seed.calls, 1);
    assert_eq!(sink.writes, 0);
}

#[test]
fn rejects_invalid_openpgp_before_requesting_seed_material() {
    assert!(ReviewedRecoveryFriendRequest::import(
        include_bytes!("../../../fixtures/phone-backup-v1/review-only-backup.json"),
        include_bytes!("../../../fixtures/phone-backup-v1/regtest-config.json"),
        b"not an OpenPGP public key",
    )
    .is_err());
}
