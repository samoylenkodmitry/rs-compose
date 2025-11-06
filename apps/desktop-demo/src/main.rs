use compose_app::ComposeAppOptions;
use desktop_app::app::combined_app;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    println!("=== Compose-RS Desktop Example ===");
    println!("Click the Increment/Decrement buttons to see:");
    println!("  - Side effect cleanup when switching branches");
    println!("  - Frame clock callbacks firing");
    println!("  - Smart recomposition (only affected parts update)");
    println!("  - Intrinsic measurements in layout");
    println!();
    println!("Press 'D' key to dump debug info about what's on screen");
    println!();

    compose_app::ComposeApp!(
        options: ComposeAppOptions::default()
            .WithTitle("Compose Counter")
            .WithSize(800, 600),
        {
            combined_app();
        }
    );
}
