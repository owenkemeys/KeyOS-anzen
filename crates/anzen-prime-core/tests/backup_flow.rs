use anzen_prime_core::{AppSeedSource, ApprovedPackageSink, ReviewedPhoneBackupRequest};

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

#[test]
fn imports_for_review_without_seed_and_failed_authentication_writes_nothing() {
    let reviewed = ReviewedPhoneBackupRequest::import(
        include_bytes!("../../../fixtures/phone-backup-v1/review-only-backup.json"),
        include_bytes!("../../../fixtures/phone-backup-v1/regtest-config.json"),
    )
    .unwrap();
    let mut seed = Seed { calls: 0 };
    let mut sink = Sink { writes: 0 };
    assert_eq!(reviewed.summary().network, "regtest");
    assert_eq!(seed.calls, 0);
    assert!(reviewed.approve_and_export(&mut seed, &mut sink).is_err());
    assert_eq!(seed.calls, 1);
    assert_eq!(sink.writes, 0);
}
