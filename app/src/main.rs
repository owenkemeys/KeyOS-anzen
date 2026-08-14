use anzen_prime_core::{
    AppSeedSource, ApprovedPackageSink, ReviewedPolicyPackage,
};
use slint_keyos_platform::{app_ui, slint::SharedString};
use std::{
    cell::RefCell,
    io::Write,
    rc::Rc,
};

#[cfg(keyos)]
use std::io::Read;

security::use_api!();
app_ui!("Anzen Policy Approval");

#[cfg(keyos)]
const IMPORT_FILE: &str = "anzen-policy-v4.json";
const APPROVED_FILE: &str = "anzen-policy-v4-approved.json";
const TEMPORARY_FILE: &str = ".anzen-policy-v4-approved.tmp";

struct KeyOsSeedSource;

impl AppSeedSource for KeyOsSeedSource {
    type Error = ();

    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        Security::default().app_seed().map_err(|_| ())
    }
}

struct DevelopmentApprovedSink {
    fs: FileSystem,
}

impl Default for DevelopmentApprovedSink {
    fn default() -> Self {
        Self {
            fs: FileSystem::default(),
        }
    }
}

impl ApprovedPackageSink for DevelopmentApprovedSink {
    type Error = ();

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), ()> {
        let mut file = self
            .fs
            .open_file(TEMPORARY_FILE, export_location(), fs::OpenFlags::CREATE)
            .map_err(|_| ())?;
        file.truncate().map_err(|_| ())?;
        file.write_all(bytes).map_err(|_| ())?;
        file.flush().map_err(|_| ())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        self.fs
            .rename(TEMPORARY_FILE, APPROVED_FILE, export_location())
            .map_err(|_| ())
    }
}

fn app_main(_cx: AppContext, ui: AppWindow) {
    log_server::init_wait(env!("CARGO_CRATE_NAME")).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    #[cfg(not(keyos))]
    {
        ui.set_environment(SharedString::from("PRIME SIMULATOR · REGTEST FIXTURE"));
        ui.set_status_detail(SharedString::from(import_prompt()));
    }

    let reviewed = Rc::new(RefCell::new(None::<ReviewedPolicyPackage>));
    let ui_weak = ui.as_weak();
    let reviewed_for_action = reviewed.clone();
    ui.on_primary_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match ui.get_stage() {
            0 => match read_import().and_then(|bytes| {
                ReviewedPolicyPackage::import(&bytes)
                    .map_err(|_| "The file is not a valid Anzen PolicyPackage v4".to_string())
            }) {
                Ok(package) => {
                    show_review(&ui, &package);
                    *reviewed_for_action.borrow_mut() = Some(package);
                }
                Err(error) => show_error(&ui, error),
            },
            1 => {
                ui.set_status_title(SharedString::from("Validating and signing..."));
                ui.set_status_detail(SharedString::from(
                    "Checking all 28 PSBTs and phone signatures before adding Prime approval.",
                ));
                let mut seed = KeyOsSeedSource;
                let mut sink = DevelopmentApprovedSink::default();
                let result = reviewed_for_action
                    .borrow()
                    .as_ref()
                    .ok_or_else(|| "Import the policy again".to_string())
                    .and_then(|package| {
                        package
                            .approve_and_export(&mut seed, &mut sink)
                            .map_err(|_| {
                                "Approval failed. The existing approved file was not replaced."
                                    .to_string()
                            })
                    });
                match result {
                    Ok(receipt) => {
                        ui.set_stage(2);
                        ui.set_success(true);
                        ui.set_status_title(SharedString::from("Approved package written"));
                        ui.set_status_detail(SharedString::from(format!(
                            "{} PSBTs validated · {} HWW signatures added\n{}",
                            receipt.psbt_count,
                            receipt.hww_signature_count,
                            approved_destination(),
                        )));
                        log::info!(
                            "Anzen package approved: {} PSBTs, {} HWW signatures",
                            receipt.psbt_count,
                            receipt.hww_signature_count,
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

    ui.run().expect("UI running");
}

#[cfg(keyos)]
fn read_import() -> Result<Vec<u8>, String> {
    let fs = FileSystem::default();
    let file = fs
        .open_file(IMPORT_FILE, fs::Location::Usb, fs::OpenFlags::READ_ONLY)
        .map_err(|_| format!("Place {IMPORT_FILE} in the development USB folder and try again"))?;
    let mut bytes = Vec::new();
    file.take((anzen_policy_engine::MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "The policy file could not be read".to_string())?;
    Ok(bytes)
}

#[cfg(not(keyos))]
fn read_import() -> Result<Vec<u8>, String> {
    Ok(include_bytes!("../../fixtures/policy-package-v4/regtest-proposal.json").to_vec())
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
fn approved_destination() -> &'static str {
    APPROVED_FILE
}

#[cfg(not(keyos))]
fn approved_destination() -> &'static str {
    "Approved JSON saved in simulator app storage"
}

fn show_review(ui: &AppWindow, package: &ReviewedPolicyPackage) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
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

fn show_error(ui: &AppWindow, error: String) {
    ui.set_success(false);
    ui.set_status_title(SharedString::from("Action could not complete"));
    ui.set_status_detail(SharedString::from(error));
}

fn reset_import(ui: &AppWindow) {
    ui.set_stage(0);
    ui.set_success(false);
    ui.set_status_title(SharedString::from("Ready to import"));
    ui.set_status_detail(SharedString::from(
        import_prompt(),
    ));
}

fn btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}
