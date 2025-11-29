pub mod app;
pub mod fonts;

use crate::fonts::DEMO_FONTS;
use compose_app::AppLauncher;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

fn create_app() -> AppLauncher {
    AppLauncher::new()
        .with_title("Compose Demo")
        .with_size(800, 600)
        .with_fonts(&DEMO_FONTS)
}

/// Shared entry point for desktop
#[cfg(not(target_os = "android"))]
pub fn entry_point() {
    let _ = env_logger::try_init();
    create_app().run(app::combined_app);
}

/// Android entry point
#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: android_activity::AndroidApp) {
    create_app().run(app, app::combined_app);
}

/// Web entry point
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[cfg(target_arch = "wasm32")]
pub fn wasm_entry_point() {
    create_app().run(app::combined_app);
}
