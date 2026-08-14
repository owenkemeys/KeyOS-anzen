use anzen_prime_core::{AppSeedSource, ApprovedPackageSink, ReviewedPolicyPackage};
use std::{cell::RefCell, rc::Rc};

slint::include_modules!();

const FIXTURE: &[u8] = include_bytes!("../../fixtures/policy-package-v4/regtest-proposal.json");

struct PreviewSeed;

impl AppSeedSource for PreviewSeed {
    type Error = ();

    fn app_seed(&mut self) -> Result<[u8; 32], ()> {
        Ok([0x42; 32])
    }
}

#[derive(Default)]
struct MemorySink {
    temporary: Option<Vec<u8>>,
    approved: Vec<u8>,
}

impl ApprovedPackageSink for MemorySink {
    type Error = ();

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), ()> {
        self.temporary = Some(bytes.to_vec());
        Ok(())
    }

    fn commit_temporary(&mut self) -> Result<(), ()> {
        self.approved = self.temporary.take().ok_or(())?;
        Ok(())
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.set_environment("WINDOWS PREVIEW · SAME POLICY ENGINE".into());
    let reviewed = Rc::new(RefCell::new(None::<ReviewedPolicyPackage>));

    let ui_weak = ui.as_weak();
    let reviewed_for_action = reviewed.clone();
    ui.on_primary_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match ui.get_stage() {
            0 => match ReviewedPolicyPackage::import(FIXTURE) {
                Ok(package) => {
                    show_review(&ui, &package);
                    *reviewed_for_action.borrow_mut() = Some(package);
                }
                Err(error) => show_error(&ui, format!("Import failed: {error:?}")),
            },
            1 => {
                let mut seed = PreviewSeed;
                let mut sink = MemorySink::default();
                let result = reviewed_for_action
                    .borrow()
                    .as_ref()
                    .ok_or("Import the policy again")
                    .and_then(|package| {
                        package
                            .approve_and_export(&mut seed, &mut sink)
                            .map_err(|_| "The package could not be approved")
                    });
                match result {
                    Ok(receipt) => {
                        ui.set_stage(2);
                        ui.set_success(true);
                        ui.set_status_title("Approved package written".into());
                        ui.set_status_detail(
                            format!(
                                "{} PSBTs independently validated · {} HWW signatures added\nanzen-policy-v4-approved.json",
                                receipt.psbt_count, receipt.hww_signature_count
                            )
                            .into(),
                        );
                    }
                    Err(error) => show_error(&ui, error.to_string()),
                }
            }
            _ => {
                *reviewed_for_action.borrow_mut() = None;
                reset_import(&ui);
            }
        }
    });

    ui.run()
}

fn show_review(ui: &AppWindow, package: &ReviewedPolicyPackage) {
    let summary = package.summary();
    ui.set_stage(1);
    ui.set_success(false);
    ui.set_vault_amount(format!("{} protected", btc(summary.total_input_sats)).into());
    ui.set_monthly_access(
        format!(
            "{} × {} months",
            btc(summary.monthly_limit_sats),
            summary.allowance_count
        )
        .into(),
    );
    ui.set_emergency_access(
        format!(
            "{} after {} days",
            btc(summary.emergency_access_limit_sats),
            summary.emergency_delay_seconds.unwrap_or_default() / 86_400
        )
        .into(),
    );
    ui.set_network_label(summary.network.to_uppercase().into());
    ui.set_fee_label(format!("{} sat/vB", summary.fee_rate_sat_vb).into());
    ui.set_transaction_label(format!("{} PSBTs", package.transaction_count()).into());
    ui.set_status_title("Review before approving".into());
    ui.set_status_detail(
        "The package parsed successfully. Approval will revalidate every transaction and phone signature before the Prime signing key is requested."
            .into(),
    );
}

fn show_error(ui: &AppWindow, error: String) {
    ui.set_success(false);
    ui.set_status_title("Action could not complete".into());
    ui.set_status_detail(error.into());
}

fn reset_import(ui: &AppWindow) {
    ui.set_stage(0);
    ui.set_success(false);
    ui.set_status_title("Ready to import".into());
    ui.set_status_detail(
        "Reads anzen-policy-v4.json from the development USB folder. No signing key is used during import."
            .into(),
    );
}

fn btc(sats: u64) -> String {
    format!("{}.{:08} BTC", sats / 100_000_000, sats % 100_000_000)
}
