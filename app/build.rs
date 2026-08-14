use slint_keyos_platform_build::{compile_options, CompileOptions};

fn main() {
    println!("cargo::rustc-check-cfg=cfg(keyos)");
    compile_options(CompileOptions {
        module_path: "ui/app.slint",
        include_slint: true,
        include_router: false,
        include_translations: false,
        include_time_localization: false,
    });
}
