use anzen_prime_core::{
    AppSeedSource, ApprovalClock, ApprovalReceipt, ApprovedPackageSink, ReviewedCooperativeSweep,
    ReviewedHwwRecoveryRequest, ReviewedPhoneBackupRequest, ReviewedPhoneRotationRequest,
    ReviewedPolicyPackage, ReviewedRecoveryFriendRequest,
};
use slint_keyos_platform::{app_ui, slint::SharedString};
use std::{cell::RefCell, io::Write, rc::Rc, time::Instant};

#[cfg(keyos)]
use std::io::Read;

security::use_api!();
app_ui!("Anzen Policy Approval");

#[cfg(keyos)]
fn keyos_getrandom(buffer: &mut [u8]) -> Result<(), getrandom::Error> {
    use std::num::NonZeroU32;

    let security = Security::default();
    for chunk in buffer.chunks_mut(32) {
        let random = security.get_random().map_err(|_| {
            let code = NonZeroU32::new(getrandom::Error::CUSTOM_START + 1)
                .expect("custom getrandom error code");
            getrandom::Error::from(code)
        })?;
        chunk.copy_from_slice(&random[..chunk.len()]);
    }
    Ok(())
}

#[cfg(keyos)]
getrandom::register_custom_getrandom!(keyos_getrandom);

#[cfg(keyos)]
const POLICY_IMPORT_FILE: &str = "anzen-policy-v4.json";
const POLICY_APPROVED_FILE: &str = "anzen-policy-v4-approved.json";
const POLICY_TEMPORARY_FILE: &str = ".anzen-policy-v4-approved.tmp";
const SWEEP_IMPORT_FILE: &str = "anzen-sweep-v1.json";
const SWEEP_APPROVED_FILE: &str = "anzen-sweep-v1-approved.json";
const SWEEP_TEMPORARY_FILE: &str = ".anzen-sweep-v1-approved.tmp";
const ROTATION_IMPORT_FILE: &str = "anzen-rotation-v1.json";
const ROTATION_CONFIG_FILE: &str = "anzen-current-config-v1.json";
const ROTATION_PENDING_FILE: &str = "anzen-pending-phone-rotation-v1.json";
const ROTATION_BACKUP_FILE: &str = "anzen-current-phone-backup-v1.json";
const ROTATION_APPROVED_FILE: &str = "anzen-rotation-v1-approved.json";
const ROTATION_TEMPORARY_FILE: &str = ".anzen-rotation-v1-approved.tmp";
const HWW_RECOVERY_IMPORT_FILE: &str = "anzen-hww-recovery-snapshot-v1.json";
const HWW_RECOVERY_APPROVED_FILE: &str = "anzen-hww-recovery-transaction-v1.json";
const HWW_RECOVERY_TEMPORARY_FILE: &str = ".anzen-hww-recovery-transaction-v1.tmp";
const BACKUP_IMPORT_FILE: &str = "anzen-phone-backup-v1.json";
const BACKUP_CONFIG_FILE: &str = "anzen-phone-backup-config-v1.json";
const BACKUP_APPROVED_FILE: &str = "anzen-phone-recovery-v2.json";
const BACKUP_TEMPORARY_FILE: &str = ".anzen-phone-recovery-v2.tmp";
const FRIEND_PUBLIC_KEY_FILE: &str = "anzen-recovery-friend-public.asc";
const FRIEND_APPROVED_FILE: &str = "anzen-phone-backup-friend-added-v1.json";
const FRIEND_TEMPORARY_FILE: &str = ".anzen-phone-backup-friend-added-v1.tmp";

enum ReviewedRequest {
    Policy(ReviewedPolicyPackage),
    Sweep(ReviewedCooperativeSweep),
    Rotation(ReviewedPhoneRotationRequest),
    HwwRecovery(ReviewedHwwRecoveryRequest),
    PhoneBackup(ReviewedPhoneBackupRequest),
    RecoveryFriend(ReviewedRecoveryFriendRequest),
}

impl ReviewedRequest {
    fn approve_and_export_timed(
        &self,
        seed: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
        clock: &mut impl ApprovalClock,
    ) -> Result<ApprovalReceipt, anzen_prime_core::PolicyFlowError> {
        match self {
            Self::Policy(package) => package.approve_and_export_timed(seed, sink, clock),
            Self::Sweep(package) => package.approve_and_export_timed(seed, sink, clock),
            Self::Rotation(package) => package.approve_and_export_timed(seed, sink, clock),
            Self::HwwRecovery(package) => package.approve_and_export_timed(seed, sink, clock),
            Self::PhoneBackup(package) => package.approve_and_export_timed(seed, sink, clock),
            Self::RecoveryFriend(package) => package.approve_and_export_timed(seed, sink, clock),
        }
    }

    fn kind(&self) -> RequestKind {
        match self {
            Self::Policy(_) => RequestKind::Policy,
            Self::Sweep(_) => RequestKind::Sweep,
            Self::Rotation(_) => RequestKind::Rotation,
            Self::HwwRecovery(_) => RequestKind::HwwRecovery,
            Self::PhoneBackup(_) => RequestKind::PhoneBackup,
            Self::RecoveryFriend(_) => RequestKind::RecoveryFriend,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RequestKind {
    Policy,
    Sweep,
    Rotation,
    HwwRecovery,
    PhoneBackup,
    RecoveryFriend,
}

struct KeyOsSeedSource;

struct SystemApprovalClock {
    started: Instant,
}

impl SystemApprovalClock {
    fn start() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl ApprovalClock for SystemApprovalClock {
    fn now_ms(&mut self) -> u64 {
        self.started.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
}

impl AppSeedSource for KeyOsSeedSource {
    type Error = ();

    #[cfg(keyos)]
    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        Security::default().app_seed().map_err(|_| ())
    }

    #[cfg(not(keyos))]
    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        Ok([0x42; 32])
    }
}

struct DevelopmentApprovedSink {
    fs: FileSystem,
    temporary_file: &'static str,
    approved_file: &'static str,
}

impl DevelopmentApprovedSink {
    fn policy() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: POLICY_TEMPORARY_FILE,
            approved_file: POLICY_APPROVED_FILE,
        }
    }

    fn sweep() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: SWEEP_TEMPORARY_FILE,
            approved_file: SWEEP_APPROVED_FILE,
        }
    }

    fn rotation() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: ROTATION_TEMPORARY_FILE,
            approved_file: ROTATION_APPROVED_FILE,
        }
    }

    fn hww_recovery() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: HWW_RECOVERY_TEMPORARY_FILE,
            approved_file: HWW_RECOVERY_APPROVED_FILE,
        }
    }

    fn phone_backup() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: BACKUP_TEMPORARY_FILE,
            approved_file: BACKUP_APPROVED_FILE,
        }
    }

    fn recovery_friend() -> Self {
        Self {
            fs: FileSystem::default(),
            temporary_file: FRIEND_TEMPORARY_FILE,
            approved_file: FRIEND_APPROVED_FILE,
        }
    }
}

impl ApprovedPackageSink for DevelopmentApprovedSink {
    type Error = ();

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let mut file = self
            .fs
            .open_file(
                self.temporary_file,
                export_location(),
                fs::OpenFlags::CREATE,
            )
            .map_err(|_| ())?;
        file.truncate().map_err(|_| ())?;
        file.write_all(bytes).map_err(|_| ())?;
        file.flush().map_err(|_| ())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        self.fs
            .rename(self.temporary_file, self.approved_file, export_location())
            .map_err(|_| ())
    }
}

fn app_main(_cx: AppContext, ui: AppWindow) {
    log_server::init_wait(env!("CARGO_CRATE_NAME")).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    #[cfg(not(keyos))]
    {
        ui.set_environment(SharedString::from(
            "PRIME SIMULATOR · REGTEST FIXTURE + TEST SEED",
        ));
        ui.set_status_detail(SharedString::from(import_prompt()));
    }

    let reviewed = Rc::new(RefCell::new(None::<ReviewedRequest>));
    let ui_weak = ui.as_weak();
    let reviewed_for_action = reviewed.clone();
    ui.on_primary_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match ui.get_stage() {
            0 => match read_policy_import().and_then(|bytes| {
                ReviewedPolicyPackage::import(&bytes)
                    .map_err(|_| "The file is not a valid Anzen PolicyPackage v4".to_string())
            }) {
                Ok(package) => {
                    show_review(&ui, &package);
                    *reviewed_for_action.borrow_mut() = Some(ReviewedRequest::Policy(package));
                }
                Err(error) => show_error(&ui, error),
            },
            1 => {
                let kind = reviewed_for_action
                    .borrow()
                    .as_ref()
                    .map(ReviewedRequest::kind)
                    .unwrap_or(RequestKind::Policy);
                ui.set_status_title(SharedString::from("Validating and signing..."));
                ui.set_status_detail(SharedString::from(match kind {
                    RequestKind::Sweep => "Checking the destination, amount, fee, vault inputs, and phone signatures before adding Prime approval.",
                    RequestKind::Rotation => "Checking the old and new vaults, pending phone key, sweep, renewed policy, and authenticated recovery backup before adding Prime approval.",
                    RequestKind::HwwRecovery => "Checking the chain snapshot, 65,535-block maturity, vault inputs, destination, amount, and fee before signing the HWW-only recovery path.",
                    RequestKind::PhoneBackup => "Authenticating the encrypted backup and friend manifest, then checking the mnemonic-derived phone key and every configured vault binding.",
                    RequestKind::RecoveryFriend => "Authenticating the current backup and friend manifest, validating the OpenPGP recipient, then wrapping the existing recovery key for that fingerprint.",
                    RequestKind::Policy => "Checking every policy PSBT and phone signature before adding Prime approval.",
                }));
                let mut seed = KeyOsSeedSource;
                let mut sink = match kind {
                    RequestKind::Policy => DevelopmentApprovedSink::policy(),
                    RequestKind::Sweep => DevelopmentApprovedSink::sweep(),
                    RequestKind::Rotation => DevelopmentApprovedSink::rotation(),
                    RequestKind::HwwRecovery => DevelopmentApprovedSink::hww_recovery(),
                    RequestKind::PhoneBackup => DevelopmentApprovedSink::phone_backup(),
                    RequestKind::RecoveryFriend => DevelopmentApprovedSink::recovery_friend(),
                };
                let mut clock = SystemApprovalClock::start();
                let result = reviewed_for_action
                    .borrow()
                    .as_ref()
                    .ok_or_else(|| "Import the policy again".to_string())
                    .and_then(|package| {
                        package
                            .approve_and_export_timed(&mut seed, &mut sink, &mut clock)
                            .map_err(|_| {
                                "Approval failed. The existing approved file was not replaced."
                                    .to_string()
                            })
                    });
                match result {
                    Ok(receipt) => {
                        ui.set_stage(2);
                        ui.set_success(true);
                        ui.set_status_title(SharedString::from(match kind {
                            RequestKind::Policy => "Approved package written",
                            RequestKind::Sweep => "Approved sweep written",
                            RequestKind::Rotation => "Approved rotation written",
                            RequestKind::HwwRecovery => "Recovery transaction written",
                            RequestKind::PhoneBackup => "Phone recovery package written",
                            RequestKind::RecoveryFriend => "Recovery friend added",
                        }));
                        ui.set_status_detail(SharedString::from(format!(
                            "{} PSBTs validated · {} HWW signatures added\n{}\n{}",
                            receipt.psbt_count,
                            receipt.hww_signature_count,
                            receipt.timing_summary(timing_context()),
                            approved_destination(kind),
                        )));
                        log::info!(
                            "Anzen package approved: {} PSBTs, {} HWW signatures, validation {} ms, signing {} ms, total approval {} ms",
                            receipt.psbt_count,
                            receipt.hww_signature_count,
                            receipt.validation_ms,
                            receipt.signing_ms,
                            receipt.total_approval_ms,
                        );
                    }
                    Err(error) => show_error(&ui, error),
                }
            }
            _ => {
                *reviewed_for_action.borrow_mut() = None;
                reset_import(&ui);
            }
        }
    });

    let ui_weak = ui.as_weak();
    let reviewed_for_sweep = reviewed.clone();
    ui.on_sweep_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match read_sweep_import().and_then(|bytes| {
            ReviewedCooperativeSweep::import(&bytes)
                .map_err(|_| "The file is not a valid Anzen cooperative sweep v1".to_string())
        }) {
            Ok(package) => {
                show_sweep_review(&ui, &package);
                *reviewed_for_sweep.borrow_mut() = Some(ReviewedRequest::Sweep(package));
            }
            Err(error) => show_error(&ui, error),
        }
    });

    let ui_weak = ui.as_weak();
    let reviewed_for_rotation = reviewed.clone();
    ui.on_rotation_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match read_rotation_import().and_then(|(proposal, config, pending, backup)| {
            ReviewedPhoneRotationRequest::import(&proposal, &config, &pending, &backup)
                .map_err(|_| "The files are not a valid Anzen phone-key rotation v1".to_string())
        }) {
            Ok(package) => {
                show_rotation_review(&ui, &package);
                *reviewed_for_rotation.borrow_mut() = Some(ReviewedRequest::Rotation(package));
            }
            Err(error) => show_error(&ui, error),
        }
    });

    let ui_weak = ui.as_weak();
    let reviewed_for_recovery = reviewed.clone();
    ui.on_hww_recovery_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match read_hww_recovery_import().and_then(|bytes| {
            ReviewedHwwRecoveryRequest::import(&bytes).map_err(|_| {
                "The file is not a valid mature Anzen HWW recovery snapshot v1".to_string()
            })
        }) {
            Ok(package) => {
                show_hww_recovery_review(&ui, &package);
                *reviewed_for_recovery.borrow_mut() = Some(ReviewedRequest::HwwRecovery(package));
            }
            Err(error) => show_error(&ui, error),
        }
    });

    let ui_weak = ui.as_weak();
    let reviewed_for_backup = reviewed.clone();
    ui.on_phone_backup_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match read_phone_backup_import().and_then(|(backup, config)| {
            ReviewedPhoneBackupRequest::import(&backup, &config).map_err(|_| {
                "The files are not a valid descriptor-bound Anzen phone backup".to_string()
            })
        }) {
            Ok(package) => {
                show_phone_backup_review(&ui, &package);
                *reviewed_for_backup.borrow_mut() = Some(ReviewedRequest::PhoneBackup(package));
            }
            Err(error) => show_error(&ui, error),
        }
    });

    let ui_weak = ui.as_weak();
    let reviewed_for_friend = reviewed.clone();
    ui.on_recovery_friend_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match read_recovery_friend_import().and_then(|(backup, config, public_key)| {
            ReviewedRecoveryFriendRequest::import(&backup, &config, &public_key).map_err(|_| {
                "The files are not a valid Anzen recovery-friend enrollment".to_string()
            })
        }) {
            Ok(package) => {
                show_recovery_friend_review(&ui, &package);
                *reviewed_for_friend.borrow_mut() = Some(ReviewedRequest::RecoveryFriend(package));
            }
            Err(error) => show_error(&ui, error),
        }
    });

    ui.run().expect("UI running");
}

#[cfg(keyos)]
fn read_policy_import() -> Result<Vec<u8>, String> {
    let fs = FileSystem::default();
    let file = fs
        .open_file(
            POLICY_IMPORT_FILE,
            fs::Location::Usb,
            fs::OpenFlags::READ_ONLY,
        )
        .map_err(|_| {
            format!("Place {POLICY_IMPORT_FILE} in the development USB folder and try again")
        })?;
    let mut bytes = Vec::new();
    file.take((anzen_policy_engine::MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "The policy file could not be read".to_string())?;
    Ok(bytes)
}

#[cfg(not(keyos))]
fn read_policy_import() -> Result<Vec<u8>, String> {
    Ok(include_bytes!("../../fixtures/policy-package-v4/regtest-proposal.json").to_vec())
}

#[cfg(keyos)]
fn read_sweep_import() -> Result<Vec<u8>, String> {
    let fs = FileSystem::default();
    let file = fs
        .open_file(
            SWEEP_IMPORT_FILE,
            fs::Location::Usb,
            fs::OpenFlags::READ_ONLY,
        )
        .map_err(|_| {
            format!("Place {SWEEP_IMPORT_FILE} in the development USB folder and try again")
        })?;
    let mut bytes = Vec::new();
    file.take((anzen_policy_engine::MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "The sweep file could not be read".to_string())?;
    Ok(bytes)
}

#[cfg(keyos)]
fn read_rotation_import() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>), String> {
    Ok((
        read_usb_file(ROTATION_IMPORT_FILE)?,
        read_usb_file(ROTATION_CONFIG_FILE)?,
        read_usb_file(ROTATION_PENDING_FILE)?,
        read_usb_file(ROTATION_BACKUP_FILE)?,
    ))
}

#[cfg(keyos)]
fn read_hww_recovery_import() -> Result<Vec<u8>, String> {
    read_usb_file(HWW_RECOVERY_IMPORT_FILE)
}

#[cfg(keyos)]
fn read_phone_backup_import() -> Result<(Vec<u8>, Vec<u8>), String> {
    Ok((
        read_usb_file(BACKUP_IMPORT_FILE)?,
        read_usb_file(BACKUP_CONFIG_FILE)?,
    ))
}

#[cfg(keyos)]
fn read_recovery_friend_import() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    Ok((
        read_usb_file(ROTATION_BACKUP_FILE)?,
        read_usb_file(ROTATION_CONFIG_FILE)?,
        read_usb_file(FRIEND_PUBLIC_KEY_FILE)?,
    ))
}

#[cfg(not(keyos))]
fn read_phone_backup_import() -> Result<(Vec<u8>, Vec<u8>), String> {
    Err("Phone-backup decryption is proved by host tests and public unchanged-Luke CI; no recovery secret is bundled in the simulator.".to_string())
}

#[cfg(not(keyos))]
fn read_recovery_friend_import() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), String> {
    Err("Recovery-friend enrollment is proved by host tests and public unchanged-Luke CI; no recovery identity is bundled in the simulator.".to_string())
}

#[cfg(not(keyos))]
fn read_hww_recovery_import() -> Result<Vec<u8>, String> {
    Ok(include_bytes!("../../fixtures/hww-recovery-v1/regtest-snapshot.json").to_vec())
}

#[cfg(keyos)]
fn read_usb_file(name: &str) -> Result<Vec<u8>, String> {
    let fs = FileSystem::default();
    let file = fs
        .open_file(name, fs::Location::Usb, fs::OpenFlags::READ_ONLY)
        .map_err(|_| format!("Place {name} in the development USB folder and try again"))?;
    let mut bytes = Vec::new();
    file.take((anzen_policy_engine::MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| format!("{name} could not be read"))?;
    Ok(bytes)
}

#[cfg(not(keyos))]
fn read_rotation_import() -> Result<(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>), String> {
    Err("The rotation flow is proved by public Luke/regtest CI; no synthetic simulator rotation is bundled.".to_string())
}

#[cfg(not(keyos))]
fn read_sweep_import() -> Result<Vec<u8>, String> {
    Ok(include_bytes!("../../fixtures/cooperative-sweep-v1/regtest-proposal.json").to_vec())
}

#[cfg(keyos)]
fn export_location() -> fs::Location {
    fs::Location::Usb
}

#[cfg(not(keyos))]
fn export_location() -> fs::Location {
    fs::Location::AppData
}

#[cfg(keyos)]
fn import_prompt() -> &'static str {
    "Reads anzen-policy-v4.json from the development USB folder. No signing key is used during import."
}

#[cfg(not(keyos))]
fn import_prompt() -> &'static str {
    "Loads Luke's real regtest PolicyPackage v4 fixture. No signing key is used during import."
}

#[cfg(keyos)]
fn approved_destination(kind: RequestKind) -> &'static str {
    match kind {
        RequestKind::Policy => POLICY_APPROVED_FILE,
        RequestKind::Sweep => SWEEP_APPROVED_FILE,
        RequestKind::Rotation => ROTATION_APPROVED_FILE,
        RequestKind::HwwRecovery => HWW_RECOVERY_APPROVED_FILE,
        RequestKind::PhoneBackup => BACKUP_APPROVED_FILE,
        RequestKind::RecoveryFriend => FRIEND_APPROVED_FILE,
    }
}

#[cfg(keyos)]
fn timing_context() -> &'static str {
    "PHYSICAL PRIME TIMING"
}

#[cfg(not(keyos))]
fn timing_context() -> &'static str {
    "SIMULATOR TIMING · NOT HARDWARE"
}

#[cfg(not(keyos))]
fn approved_destination(_kind: RequestKind) -> &'static str {
    "Approved JSON saved in simulator app storage"
}

fn show_review(ui: &AppWindow, package: &ReviewedPolicyPackage) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review annual policy"));
    ui.set_primary_label(SharedString::from("VAULT"));
    ui.set_secondary_label(SharedString::from("MONTHLY ACCESS"));
    ui.set_tertiary_label(SharedString::from("EMERGENCY"));
    ui.set_vault_amount(SharedString::from(format!(
        "{} protected",
        btc(summary.total_input_sats)
    )));
    ui.set_monthly_access(SharedString::from(format!(
        "{} × {} months",
        btc(summary.monthly_limit_sats),
        summary.allowance_count,
    )));
    ui.set_emergency_access(SharedString::from(format!(
        "{} after {} days",
        btc(summary.emergency_access_limit_sats),
        summary.emergency_delay_seconds.unwrap_or_default() / 86_400,
    )));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from(format!(
        "{} sat/vB",
        summary.fee_rate_sat_vb
    )));
    ui.set_transaction_label(SharedString::from(format!(
        "{} PSBTs",
        package.transaction_count()
    )));
    ui.set_status_title(SharedString::from("Review before approving"));
    ui.set_status_detail(SharedString::from(
        "The package parsed successfully. Approval will revalidate every transaction and phone signature before the Prime signing key is requested.",
    ));
}

fn show_sweep_review(ui: &AppWindow, package: &ReviewedCooperativeSweep) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review cooperative sweep"));
    ui.set_primary_label(SharedString::from("SEND"));
    ui.set_secondary_label(SharedString::from("DESTINATION"));
    ui.set_tertiary_label(SharedString::from("VAULT INPUTS"));
    ui.set_vault_amount(SharedString::from(btc(summary.sent_sats)));
    ui.set_monthly_access(SharedString::from(summary.destination.clone()));
    ui.set_emergency_access(SharedString::from(format!(
        "{} input{}",
        summary.input_count,
        if summary.input_count == 1 { "" } else { "s" }
    )));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from(format!("{} sats fee", summary.fee_sats)));
    ui.set_transaction_label(SharedString::from("1 PSBT"));
    ui.set_status_title(SharedString::from("Review before approving"));
    ui.set_status_detail(SharedString::from(
        "Approval will revalidate the destination, amount, fee, every vault input, and every phone signature before the Prime signing key is requested.",
    ));
}

fn show_rotation_review(ui: &AppWindow, package: &ReviewedPhoneRotationRequest) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review phone-key rotation"));
    ui.set_primary_label(SharedString::from("MOVE TO NEW VAULT"));
    ui.set_secondary_label(SharedString::from("NEW VAULT ADDRESS"));
    ui.set_tertiary_label(SharedString::from("POLICY + RECOVERY"));
    ui.set_vault_amount(SharedString::from(btc(summary.sent_sats)));
    ui.set_monthly_access(SharedString::from(summary.new_vault_address.clone()));
    ui.set_emergency_access(SharedString::from(format!(
        "{} policy PSBTs · {} recovery friends",
        summary.policy_psbt_count, summary.recovery_friend_count
    )));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from(format!("{} sats fee", summary.fee_sats)));
    ui.set_transaction_label(SharedString::from(format!(
        "{} vault input{}",
        summary.input_count,
        if summary.input_count == 1 { "" } else { "s" }
    )));
    ui.set_status_title(SharedString::from("Review before approving"));
    ui.set_status_detail(SharedString::from(
        "Approval will bind the pending phone key to the new vault, revalidate and sign the sweep and renewed policy, then authenticate and renew the recovery backup.",
    ));
}

fn show_hww_recovery_review(ui: &AppWindow, package: &ReviewedHwwRecoveryRequest) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review delayed HWW recovery"));
    ui.set_primary_label(SharedString::from("RECOVER"));
    ui.set_secondary_label(SharedString::from("DESTINATION"));
    ui.set_tertiary_label(SharedString::from("DELAY + INPUTS"));
    ui.set_vault_amount(SharedString::from(btc(summary.sent_sats)));
    ui.set_monthly_access(SharedString::from(summary.destination.clone()));
    ui.set_emergency_access(SharedString::from(format!(
        "{} blocks · {} input{}",
        summary.delay_blocks,
        summary.input_count,
        if summary.input_count == 1 { "" } else { "s" }
    )));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from(format!("{} sats fee", summary.fee_sats)));
    ui.set_transaction_label(SharedString::from(format!(
        "tip {}",
        summary.snapshot_tip_height
    )));
    ui.set_status_title(SharedString::from("Review snapshot before signing"));
    ui.set_status_detail(SharedString::from(
        "This development snapshot is not a chain transport. Approval revalidates every prevout and signs only the mature 65,535-block HWW recovery leaf.",
    ));
}

fn show_phone_backup_review(ui: &AppWindow, package: &ReviewedPhoneBackupRequest) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review phone-backup recovery"));
    ui.set_primary_label(SharedString::from("VAULT"));
    ui.set_secondary_label(SharedString::from("NETWORK"));
    ui.set_tertiary_label(SharedString::from("RECOVERY FRIENDS"));
    ui.set_vault_amount(SharedString::from(summary.vault_address.clone()));
    ui.set_monthly_access(SharedString::from(summary.network.to_uppercase()));
    ui.set_emergency_access(SharedString::from(
        summary.recovery_friend_count.to_string(),
    ));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from("No transaction"));
    ui.set_transaction_label(SharedString::from("Recovery v2"));
    ui.set_status_title(SharedString::from("Review before decrypting"));
    ui.set_status_detail(SharedString::from("Approval authenticates the HWW envelope and friend manifest, then re-derives the phone key and checks every descriptor-bound vault field. Recovery words are never shown in logs."));
}

fn show_recovery_friend_review(ui: &AppWindow, package: &ReviewedRecoveryFriendRequest) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_operation_title(SharedString::from("Review recovery friend"));
    ui.set_primary_label(SharedString::from("OPENPGP FINGERPRINT"));
    ui.set_secondary_label(SharedString::from("VAULT"));
    ui.set_tertiary_label(SharedString::from("RECOVERY FRIENDS AFTER"));
    ui.set_vault_amount(SharedString::from(summary.fingerprint.clone()));
    ui.set_monthly_access(SharedString::from(summary.vault_address.clone()));
    ui.set_emergency_access(SharedString::from(
        (summary.current_friend_count + 1).to_string(),
    ));
    ui.set_network_label(SharedString::from(summary.network.to_uppercase()));
    ui.set_fee_label(SharedString::from("No transaction"));
    ui.set_transaction_label(SharedString::from("1-of-N access"));
    ui.set_status_title(SharedString::from("Review trust expansion"));
    ui.set_status_detail(SharedString::from("Approval authenticates the existing HWW envelope and friend manifest, then grants this exact OpenPGP fingerprint access to the descriptor-bound phone recovery package. The 61,200-block phone recovery delay is unchanged."));
}

fn show_error(ui: &AppWindow, error: String) {
    ui.set_success(false);
    ui.set_status_title(SharedString::from("Action could not complete"));
    ui.set_status_detail(SharedString::from(error));
}

fn reset_import(ui: &AppWindow) {
    ui.set_stage(0);
    ui.set_success(false);
    ui.set_status_title(SharedString::from("Ready to import"));
    ui.set_operation_title(SharedString::from("Import HWW request"));
    ui.set_status_detail(SharedString::from(import_prompt()));
}

fn btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}
