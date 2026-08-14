use anzen_prime_core::{run_demo, DemoPresentation};

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.set_environment("WINDOWS PREVIEW · SAME SIGNING CORE".into());

    let result = run_demo([0x42; 32]).expect("deterministic preview benchmark signs");
    show_result(
        &ui,
        DemoPresentation::from_result(result, "Windows preview"),
    );

    let ui_weak = ui.as_weak();
    ui.on_run_requested(move || {
        let Some(ui) = ui_weak.upgrade() else {
            return;
        };
        match run_demo([0x42; 32]) {
            Ok(result) => show_result(
                &ui,
                DemoPresentation::from_result(result, "Windows preview"),
            ),
            Err(error) => {
                ui.set_success(false);
                ui.set_status_title("Signing proof could not run".into());
                ui.set_status_detail(format!("Signing failed: {error:?}").into());
            }
        }
    });

    ui.run()
}

fn show_result(ui: &AppWindow, presentation: DemoPresentation) {
    ui.set_success(true);
    ui.set_status_title(presentation.title.into());
    ui.set_status_detail(
        format!(
            "{}\n{}\n{}",
            presentation.workload, presentation.verification, presentation.transcript
        )
        .into(),
    );
}
