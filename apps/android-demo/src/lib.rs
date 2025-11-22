#[cfg(target_os = "android")]
use android_logger::Config;
#[cfg(target_os = "android")]
use compose_app::{AndroidApp, ComposeAndroidAppWithOptions, ComposeAppOptions};
#[cfg(target_os = "android")]
use desktop_app::app::combined_app;
#[cfg(target_os = "android")]
use log::LevelFilter;

#[cfg(target_os = "android")]
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn android_main(app: AndroidApp) {
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Info)
            .with_tag("rs-compose"),
    );

    ComposeAndroidAppWithOptions(
        app,
        ComposeAppOptions::default().WithTitle("Compose Counter"),
        || {
            combined_app();
        },
    );
}

#[cfg(not(target_os = "android"))]
#[no_mangle]
pub extern "C" fn android_main(_: *mut core::ffi::c_void) {
    panic!("android-app must be built for an Android target");
}
