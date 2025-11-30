# Robot Testing Framework

The Robot Testing Framework provides automated UI testing for Compose-RS applications. Write tests that launch your app, perform user interactions, and validate UI state.

## Overview

The framework supports two testing modes:

1. **Headless Testing** - Fast unit tests with mock rendering (no window)
2. **Real App Testing** - Full E2E tests with actual windows and GPU rendering

Both modes test THE SAME application code, ensuring your tests validate real behavior.

---

## Quick Start - Real App Testing

### Run Examples

```bash
# Watch the robot interact with your app
cargo run --package desktop-app --example robot_demo --features robot-app

# Interactive demo with detailed output
cargo run --package desktop-app --example robot_interactive --features robot-app
```

### Write Your Own

```rust
use compose_app::AppLauncher;

fn main() {
    AppLauncher::new()
        .with_title("My Robot Test")
        .with_size(800, 600)
        .with_test_driver(|robot| {
            // Wait for app to be ready
            robot.wait_for_idle().unwrap();

            // Click a button
            robot.click(150.0, 560.0).unwrap();

            // Move cursor
            robot.move_to(400.0, 50.0).unwrap();

            // Exit when done
            robot.exit().unwrap();
        })
        .run(|| {
            // Your app content here
            my_app_ui();
        });
}
```

### How It Works

- **Event loop runs on main thread** (required on Linux)
- **Test driver runs in separate thread**
- **Communication via channels** (RobotCommand/RobotResponse)
- **Robot commands processed during idle time**
- **You see the window** and watch the robot interact with it!

---

## Quick Start - Headless Testing

Headless tests run fast without creating windows - perfect for unit tests and CI/CD.

### Basic Test

```rust
use compose_testing::robot::create_headless_robot_test;
use compose_ui::prelude::*;

#[test]
fn test_button_click() {
    let mut robot = create_headless_robot_test(800, 600, || {
        Column(|| {
            Text("Click the button");

            Button("Click Me", || {
                println!("Button clicked!");
            });
        });
    });

    robot.wait_for_idle();

    // Find button by text
    let button = robot.find_by_text("Click Me").unwrap();

    // Click it
    robot.click_at(button.bounds.center_x(), button.bounds.center_y());
    robot.wait_for_idle();

    assert!(/* verify state changed */);
}
```

### Running Headless Tests

```bash
# Run all robot tests
cargo test -p desktop-app robot

# Run specific test
cargo test -p desktop-app test_button_click
```

---

## Robot API Reference

### Real App Robot (with_test_driver)

```rust
// Click at coordinates (logical pixels)
robot.click(x: f32, y: f32) -> Result<(), String>

// Move cursor
robot.move_to(x: f32, y: f32) -> Result<(), String>

// Wait for app to be idle (no redraws, no animations)
robot.wait_for_idle() -> Result<(), String>

// Exit the application
robot.exit() -> Result<(), String>
```

### Headless Robot

```rust
// Click at coordinates
robot.click_at(x: f32, y: f32)

// Drag from one point to another
robot.drag(from_x: f32, from_y: f32, to_x: f32, to_y: f32)

// Move cursor without clicking
robot.move_to(x: f32, y: f32)

// Wait for all updates to process
robot.wait_for_idle()

// Find elements
robot.find_by_text(text: &str) -> Option<Element>
robot.find_clickable_at(x: f32, y: f32) -> Option<Element>

// Get all elements
robot.get_all_text() -> Vec<String>
robot.get_all_rects() -> Vec<Rect>
robot.get_all_clickable() -> Vec<Element>

// Debug
robot.dump_screen()  // Print UI tree
```

---

## Testing Patterns

### Pattern 1: Tab Navigation Test

```rust
AppLauncher::new()
    .with_title("Tab Test")
    .with_test_driver(|robot| {
        robot.wait_for_idle().unwrap();

        // Click first tab
        robot.click(70.0, 50.0).unwrap();
        robot.wait_for_idle().unwrap();

        // Verify content changed (add assertions)

        // Click second tab
        robot.click(400.0, 50.0).unwrap();
        robot.wait_for_idle().unwrap();
    })
    .run(|| { app() });
```

### Pattern 2: Counter Increment Test

```rust
#[test]
fn test_counter_increment() {
    let mut robot = create_headless_robot_test(800, 600, || {
        counter_app();
    });

    robot.wait_for_idle();

    // Find and click increment button
    let button = robot.find_by_text("+").unwrap();
    robot.click_at(button.bounds.center_x(), button.bounds.center_y());
    robot.wait_for_idle();

    // Verify count increased
    let texts = robot.get_all_text();
    assert!(texts.iter().any(|t| t.contains("1")));
}
```

### Pattern 3: Drag Gesture Test

```rust
#[test]
fn test_drag_slider() {
    let mut robot = create_headless_robot_test(800, 600, || {
        slider_app();
    });

    robot.wait_for_idle();

    // Drag slider from left to right
    robot.drag(100.0, 300.0, 500.0, 300.0);
    robot.wait_for_idle();

    // Verify slider value changed
    assert!(/* check slider value */);
}
```

---

## Setup

### Enable Robot Testing

In your app's `Cargo.toml`:

```toml
[dev-dependencies]
compose-testing = { path = "../../crates/compose-testing" }

[features]
robot-app = ["compose-app/robot"]
```

### Project Structure

```
your-app/
├── examples/
│   └── robot_demo.rs          # Runnable robot demos
├── src/
│   └── tests/
│       └── robot_test.rs      # Headless unit tests
└── Cargo.toml
```

---

## Best Practices

### ✅ Do

- **Use headless tests** for fast feedback during development
- **Use real app tests** for visual regression and E2E validation
- **Test actual app code** - don't mock UI components
- **Wait for idle** before assertions to ensure updates complete
- **Use logical pixels** for coordinates (they're scale-independent)
- **Test user workflows** not individual functions

### ❌ Don't

- Don't use `#[ignore]` tests - use runnable examples instead
- Don't duplicate app code for testing - test the real thing
- Don't hardcode physical pixel coordinates (breaks on HiDPI)
- Don't forget to wait for animations to complete

---

## Troubleshooting

### "Event loop must be on main thread" Error

✅ **Fixed!** Use `with_test_driver()` which runs the event loop on the main thread.

### Robot Commands Not Working

Make sure you:
1. Call `robot.wait_for_idle()` after commands
2. Use logical pixel coordinates (not physical)
3. Enable the `robot-app` feature

### Tests Flaky

Add delays between actions:
```rust
robot.click(x, y).unwrap();
std::thread::sleep(Duration::from_millis(100));
robot.wait_for_idle().unwrap();
```

---

## Examples

See working examples:
- `apps/desktop-demo/examples/robot_demo.rs`
- `apps/desktop-demo/examples/robot_interactive.rs`
- `apps/desktop-demo/src/tests/robot_test.rs`

Run them:
```bash
cargo run --example robot_demo --features robot-app
cargo test robot_test
```

---

## Architecture

### Real App Mode

```
┌─────────────────┐
│   Main Thread   │
│  Event Loop     │ ← Runs THE actual app
│  WGPU Rendering │
│  Window         │
└────────┬────────┘
         │ Channels (RobotCommand/Response)
         │
┌────────▼────────┐
│ Test Thread     │
│ Robot Driver    │ ← Your test code
└─────────────────┘
```

### Headless Mode

```
┌─────────────────┐
│   Test Thread   │
│  AppShell       │ ← Same app logic
│  TestRenderer   │ ← Mock rendering
│  Robot          │ ← Direct control
└─────────────────┘
```

Both modes test THE SAME application code!

---

## FAQ

**Q: Can I run tests in CI/CD?**
A: Yes! Headless tests work everywhere. Real app tests need a display (use Xvfb on Linux).

**Q: How do I test animations?**
A: Use `robot.wait_for_idle()` which waits until animations complete.

**Q: Can I take screenshots?**
A: Not yet implemented, but the infrastructure is ready (see RobotCommand::Screenshot).

**Q: Why two testing modes?**
A: Headless is fast for development. Real app validates actual rendering and catches visual bugs.

**Q: How do I test my real app, not examples?**
A: Use `with_test_driver()` with your actual app code:
```rust
.run(|| { my_actual_app::ui() });
```

---

## Contributing

To add new robot capabilities:

1. Add command to `RobotCommand` enum (desktop.rs)
2. Handle it in `Event::AboutToWait` branch
3. Add method to `Robot` struct
4. Update documentation
5. Add example/test

---

For more details, see the examples and test files in the repository.
