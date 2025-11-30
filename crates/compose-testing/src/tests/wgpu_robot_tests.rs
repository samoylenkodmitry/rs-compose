use crate::robot::rect_center;
use crate::wgpu_robot::WgpuRobotApp;
use crate::WgpuRobotError;
use compose_core::useState;
use compose_macros::composable;
use compose_ui::widgets::Button;
use compose_ui::{Column, ColumnSpec, Modifier, Text};

static ROBOTO_REGULAR: &[u8] = include_bytes!("../../../../assets/Roboto-Regular.ttf");
static ROBOTO_FONTS: [&[u8]; 1] = [ROBOTO_REGULAR];

#[composable]
fn CounterApp() {
    let count = useState(|| 0i32);
    Column(Modifier::empty(), ColumnSpec::default(), move || {
        Text(
            format!("Count: {}", count.value()),
            Modifier::empty().padding(4.0),
        );
        let on_click = count;
        Button(
            Modifier::empty().padding(4.0),
            move || {
                let current = on_click.value();
                on_click.set_value(current + 1);
            },
            || {
                Text("Tap".to_string(), Modifier::empty().padding(2.0));
            },
        );
    });
}

#[test]
fn wgpu_robot_drives_real_renderer_and_captures_frames() -> Result<(), WgpuRobotError> {
    let robot = match WgpuRobotApp::launch_with_fonts(640, 480, &ROBOTO_FONTS, || {
        CounterApp();
    }) {
        Ok(robot) => robot,
        Err(WgpuRobotError::NoAdapter) => {
            eprintln!("skipping WGPU robot test: no adapter available");
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    robot.pump_until_idle(20)?;
    let snapshot = robot.snapshot()?;
    assert!(snapshot.text_values().any(|text| text == "Count: 0"));

    let button_rect = snapshot
        .text_rects("Tap")
        .first()
        .cloned()
        .expect("button text rect should exist");
    let (x, y) = rect_center(&button_rect);

    assert!(robot.move_pointer(x, y)?);
    assert!(robot.click(x, y)?);

    robot.pump_until_idle(20)?;
    let updated = robot.snapshot()?;
    assert!(updated.text_values().any(|text| text == "Count: 1"));

    let capture = robot.capture_frame()?;
    assert_eq!(capture.width, 640);
    assert_eq!(capture.height, 480);
    assert!(!capture.rgba().is_empty());

    robot.close()?;
    Ok(())
}
