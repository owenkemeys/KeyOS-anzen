use anzen_prime_core::{run_demo, DemoPresentation};
use slint_keyos_platform::{app_ui, slint::SharedString};

security::use_api!();
app_ui!("Anzen Prime Proof");

fn app_main(_cx: AppContext, ui: AppWindow) {
    log_server::init_wait(env!("CARGO_CRATE_NAME")).unwrap();
    log::set_max_level(log::LevelFilter::Info);

    let ui_weak = ui.as_weak();
    ui.on_run_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };

        ui.set_status_title(SharedString::from("Signing Anzen policy..."));
        ui.set_status_detail(SharedString::from(
            "Rebuilding 28 BIP341 transactions and producing 39 signatures.",
        ));

        let outcome = Security::default()
            .app_seed()
            .map_err(|_| "Prime app seed unavailable. Unlock KeyOS and try again.".to_string())
            .and_then(|seed| run_demo(seed).map_err(|error| format!("Signing failed: {error:?}")));

        match outcome {
            Ok(result) => {
                let presentation = DemoPresentation::from_result(result, "Passport Prime / KeyOS");
                ui.set_success(true);
                ui.set_status_title(SharedString::from("Policy signed on Passport Prime"));
                ui.set_status_detail(SharedString::from(format!(
                    "{}\n{}\n{}",
                    presentation.workload,
                    presentation.verification,
                    presentation.transcript,
                )));
                log::info!(
                    "Anzen benchmark complete: {} / {} / {}",
                    presentation.workload,
                    presentation.verification,
                    presentation.transcript,
                );
            }
            Err(error) => {
                ui.set_success(false);
                ui.set_status_title(SharedString::from("Signing proof could not run"));
                ui.set_status_detail(SharedString::from(error));
            }
        }
    });

    ui.run().expect("UI running");
}
