use anzen_policy_engine::{PolicyError, PolicyPackage, MAX_PACKAGE_BYTES};
use anzen_prime_core::{
    AppSeedSource, ApprovedPackageSink, PolicyFlowError, ReviewedPolicyPackage,
};

const FIXTURE: &[u8] = include_bytes!("../../../fixtures/policy-package-v4/regtest-proposal.json");

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
    previous: Vec<u8>,
    temporary: Option<Vec<u8>>,
    committed: Vec<u8>,
    fail_write: bool,
    fail_commit: bool,
    commits: usize,
}

impl RecordingSink {
    fn with_previous(previous: &[u8]) -> Self {
        Self {
            previous: previous.to_vec(),
            temporary: None,
            committed: previous.to_vec(),
            fail_write: false,
            fail_commit: false,
            commits: 0,
        }
    }
}

impl ApprovedPackageSink for RecordingSink {
    type Error = ();

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if self.fail_write {
            return Err(());
        }
        self.temporary = Some(bytes.to_vec());
        Ok(())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        if self.fail_commit {
            return Err(());
        }
        self.committed = self.temporary.take().ok_or(())?;
        self.commits += 1;
        Ok(())
    }
}

#[test]
fn imports_real_v4_bytes_for_review_without_requesting_seed_material() {
    let reviewed = ReviewedPolicyPackage::import(FIXTURE).expect("real package should import");
    let seed_source = SeedSource::default();

    assert_eq!(reviewed.summary().network, "regtest");
    assert_eq!(reviewed.summary().total_input_sats, 200_000_000);
    assert_eq!(reviewed.summary().monthly_limit_sats, 10_000_000);
    assert_eq!(reviewed.summary().allowance_count, 12);
    assert_eq!(reviewed.transaction_count(), 28);
    assert_eq!(seed_source.calls, 0);
}

#[test]
fn rejects_invalid_imports_before_any_approval_or_export_work() {
    let mut invalid = FIXTURE.to_vec();
    invalid[0] = b'!';

    assert!(matches!(
        ReviewedPolicyPackage::import(&invalid),
        Err(PolicyFlowError::Policy(_))
    ));
}

#[test]
fn preserves_the_engine_import_limit_at_the_prime_flow_boundary() {
    let oversized = vec![b' '; MAX_PACKAGE_BYTES + 1];

    assert!(matches!(
        ReviewedPolicyPackage::import(&oversized),
        Err(PolicyFlowError::Policy(PolicyError::PackageTooLarge))
    ));
}

#[test]
fn explicit_approval_requests_seed_once_and_commits_luke_compatible_json() {
    let reviewed = ReviewedPolicyPackage::import(FIXTURE).unwrap();
    let mut seed_source = SeedSource {
        seed: [0x42; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink::with_previous(b"older approved package");

    let receipt = reviewed
        .approve_and_export(&mut seed_source, &mut sink)
        .expect("reviewed package should approve");

    assert_eq!(seed_source.calls, 1);
    assert_eq!(sink.commits, 1);
    assert_eq!(receipt.psbt_count, 28);
    assert_eq!(receipt.hww_signature_count, 28);
    let reparsed = PolicyPackage::parse_bounded(&sink.committed).unwrap();
    assert!(reparsed.manifest.hww_approved);
}

#[test]
fn identity_mismatch_never_writes_or_replaces_an_approved_package() {
    let reviewed = ReviewedPolicyPackage::import(FIXTURE).unwrap();
    let mut seed_source = SeedSource {
        seed: [0x43; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink::with_previous(b"keep me");

    assert!(matches!(
        reviewed.approve_and_export(&mut seed_source, &mut sink),
        Err(PolicyFlowError::Policy(_))
    ));
    assert_eq!(sink.committed, b"keep me");
    assert!(sink.temporary.is_none());
    assert_eq!(sink.commits, 0);
}

#[test]
fn temporary_write_failure_preserves_the_previous_approved_package() {
    let reviewed = ReviewedPolicyPackage::import(FIXTURE).unwrap();
    let mut seed_source = SeedSource {
        seed: [0x42; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink::with_previous(b"keep me");
    sink.fail_write = true;

    assert_eq!(
        reviewed
            .approve_and_export(&mut seed_source, &mut sink)
            .unwrap_err(),
        PolicyFlowError::ExportFailed
    );
    assert_eq!(sink.previous, b"keep me");
    assert_eq!(sink.committed, b"keep me");
    assert_eq!(sink.commits, 0);
}

#[test]
fn failed_atomic_commit_preserves_the_previous_approved_package() {
    let reviewed = ReviewedPolicyPackage::import(FIXTURE).unwrap();
    let mut seed_source = SeedSource {
        seed: [0x42; 32],
        ..Default::default()
    };
    let mut sink = RecordingSink::with_previous(b"keep me");
    sink.fail_commit = true;

    assert_eq!(
        reviewed
            .approve_and_export(&mut seed_source, &mut sink)
            .unwrap_err(),
        PolicyFlowError::ExportFailed
    );
    assert_eq!(sink.committed, b"keep me");
    assert_eq!(sink.commits, 0);
}
