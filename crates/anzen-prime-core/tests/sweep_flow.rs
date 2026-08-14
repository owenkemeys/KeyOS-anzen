use anzen_policy_engine::CooperativeSweepPackage;
use anzen_prime_core::{
    AppSeedSource, ApprovedPackageSink, PolicyFlowError, ReviewedCooperativeSweep,
};
use serde_json::{json, Value};

fn sweep_fixture() -> Vec<u8> {
    let policy: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/policy-package-v4/regtest-proposal.json"
    ))
    .expect("policy fixture");
    let manifest = &policy["manifest"];
    let revocation = &manifest["allowances"][0]["revocation"];
    let psbt_file = revocation["psbt_file"].as_str().expect("PSBT path");
    let chain_value = manifest["allowances"][0]["chain_value_sats"]
        .as_u64()
        .expect("chain value");
    let fee = revocation["fee_sats"].as_u64().expect("fee");
    serde_json::to_vec(&json!({
        "version": 1,
        "kind": "cooperative-sweep",
        "vault_descriptor": manifest["vault_descriptor"],
        "destination": manifest["vault_address"],
        "psbt": policy["psbts"][psbt_file],
        "input_count": 1,
        "sent_sats": chain_value - fee,
        "fee_sats": fee,
        "phone_approved": true,
        "hww_approved": false
    }))
    .expect("sweep fixture")
}

#[derive(Default)]
struct SeedSource {
    calls: usize,
    seed: [u8; 32],
}

impl AppSeedSource for SeedSource {
    type Error = ();

    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        self.calls += 1;
        Ok(self.seed)
    }
}

struct RecordingSink {
    temporary: Option<Vec<u8>>,
    committed: Vec<u8>,
    commits: usize,
}

impl ApprovedPackageSink for RecordingSink {
    type Error = ();

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.temporary = Some(bytes.to_vec());
        Ok(())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        self.committed = self.temporary.take().ok_or(())?;
        self.commits += 1;
        Ok(())
    }
}

#[test]
fn imports_sweep_for_review_without_requesting_seed_material() {
    let reviewed = ReviewedCooperativeSweep::import(&sweep_fixture()).expect("import");
    let seed = SeedSource::default();

    assert_eq!(reviewed.summary().network, "regtest");
    assert_eq!(reviewed.summary().input_count, 1);
    assert_eq!(reviewed.summary().sent_sats, 120_002_255);
    assert_eq!(reviewed.summary().fee_sats, 162);
    assert_eq!(seed.calls, 0);
}

#[test]
fn explicit_approval_requests_seed_once_and_atomically_exports_luke_json() {
    let reviewed = ReviewedCooperativeSweep::import(&sweep_fixture()).expect("import");
    let mut seed = SeedSource {
        seed: [0x42; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink {
        temporary: None,
        committed: b"older approved sweep".to_vec(),
        commits: 0,
    };

    let receipt = reviewed
        .approve_and_export(&mut seed, &mut sink)
        .expect("approve");

    assert_eq!(seed.calls, 1);
    assert_eq!(sink.commits, 1);
    assert_eq!(receipt.psbt_count, 1);
    assert_eq!(receipt.hww_signature_count, 1);
    let package = CooperativeSweepPackage::parse_bounded(&sink.committed).expect("approved JSON");
    assert!(package.hww_approved);
}

#[test]
fn identity_mismatch_preserves_the_previous_approved_sweep() {
    let reviewed = ReviewedCooperativeSweep::import(&sweep_fixture()).expect("import");
    let mut seed = SeedSource {
        seed: [0x43; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink {
        temporary: None,
        committed: b"keep me".to_vec(),
        commits: 0,
    };

    assert!(matches!(
        reviewed.approve_and_export(&mut seed, &mut sink),
        Err(PolicyFlowError::Policy(_))
    ));
    assert_eq!(sink.committed, b"keep me");
    assert!(sink.temporary.is_none());
    assert_eq!(sink.commits, 0);
}
