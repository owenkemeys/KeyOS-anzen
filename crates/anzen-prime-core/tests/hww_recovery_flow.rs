use anzen_prime_core::{
    AppSeedSource, ApprovedPackageSink, PolicyFlowError, ReviewedHwwRecoveryRequest,
};

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

fn fixture() -> &'static [u8] {
    include_bytes!("../../../fixtures/hww-recovery-v1/regtest-snapshot.json")
}

#[test]
fn imports_mature_recovery_for_review_without_requesting_seed_material() {
    let reviewed = ReviewedHwwRecoveryRequest::import(fixture()).expect("import");
    let seed = SeedSource::default();

    assert_eq!(reviewed.summary().network, "regtest");
    assert_eq!(reviewed.summary().delay_blocks, 65_535);
    assert_eq!(reviewed.summary().input_count, 1);
    assert_eq!(seed.calls, 0);
}

#[test]
fn explicit_approval_requests_seed_once_and_atomically_exports_raw_transaction() {
    let reviewed = ReviewedHwwRecoveryRequest::import(fixture()).expect("import");
    let mut seed = SeedSource {
        seed: [0x42; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink {
        temporary: None,
        committed: b"older recovery".to_vec(),
        commits: 0,
    };

    let receipt = reviewed
        .approve_and_export(&mut seed, &mut sink)
        .expect("approve");
    let result: serde_json::Value = serde_json::from_slice(&sink.committed).unwrap();

    assert_eq!(seed.calls, 1);
    assert_eq!(sink.commits, 1);
    assert_eq!(receipt.psbt_count, 1);
    assert_eq!(receipt.hww_signature_count, 1);
    assert_eq!(result["kind"], "hww-recovery-transaction");
    assert!(result["transaction_hex"].as_str().unwrap().len() > 100);
}

#[test]
fn identity_mismatch_preserves_the_previous_recovery_result() {
    let reviewed = ReviewedHwwRecoveryRequest::import(fixture()).expect("import");
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
