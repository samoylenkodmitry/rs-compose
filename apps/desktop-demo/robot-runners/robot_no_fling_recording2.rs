//! Auto-generated robot test from recording with a fling assertion
//! Generated at: timestamp
//! Events: 164

use compose_app::AppLauncher;
use compose_testing::{find_button, find_in_semantics};
use std::time::Duration;

fn main() {
    AppLauncher::new()
        .with_headless(true)
        .with_test_driver(|robot| {
            std::thread::sleep(Duration::from_millis(500));
            let _ = robot.wait_for_idle();

            fn tab_x(robot: &compose_app::Robot, label: &str) -> Option<f32> {
                find_in_semantics(robot, |elem| find_button(elem, label)).map(|(x, _, _, _)| x)
            }

            std::thread::sleep(Duration::from_millis(408));
            let _ = robot.mouse_move(399.5, 288.7);
            let _ = robot.mouse_move(400.1, 288.2);
            let _ = robot.mouse_move(400.1, 287.4);
            let _ = robot.mouse_move(399.6, 286.0);
            let _ = robot.mouse_move(398.3, 285.5);
            let _ = robot.mouse_move(399.6, 284.2);
            let _ = robot.mouse_move(400.5, 283.3);
            let _ = robot.mouse_move(401.5, 282.4);
            let _ = robot.mouse_move(402.9, 281.5);
            let _ = robot.mouse_move(403.3, 280.5);
            let _ = robot.mouse_move(404.2, 279.6);
            let _ = robot.mouse_move(404.7, 278.7);
            let _ = robot.mouse_move(405.2, 277.8);
            let _ = robot.mouse_move(406.5, 276.4);
            let _ = robot.mouse_move(407.9, 274.5);
            let _ = robot.mouse_move(409.9, 272.1);
            let _ = robot.mouse_move(411.9, 269.6);
            let _ = robot.mouse_move(414.0, 267.0);
            let _ = robot.mouse_move(416.0, 264.4);
            let _ = robot.mouse_move(418.1, 261.9);
            let _ = robot.mouse_move(420.8, 258.6);
            let _ = robot.mouse_move(424.3, 255.1);
            std::thread::sleep(Duration::from_millis(65));
            let _ = robot.mouse_move(427.2, 251.6);
            let _ = robot.mouse_move(430.7, 247.5);
            let _ = robot.mouse_move(433.7, 243.9);
            let _ = robot.mouse_move(438.0, 239.6);
            let _ = robot.mouse_move(441.9, 234.4);
            let _ = robot.mouse_move(446.6, 229.1);
            let _ = robot.mouse_move(452.2, 222.7);
            let _ = robot.mouse_move(457.9, 217.7);
            let _ = robot.mouse_move(462.7, 212.3);
            std::thread::sleep(Duration::from_millis(7));
            let _ = robot.mouse_move(468.1, 207.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(472.0, 203.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(476.5, 198.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(481.0, 194.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(485.7, 188.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(492.1, 183.0);
            std::thread::sleep(Duration::from_millis(14));
            let _ = robot.mouse_move(497.7, 178.0);
            std::thread::sleep(Duration::from_millis(17));
            let _ = robot.mouse_move(503.2, 173.2);
            let _ = robot.mouse_move(508.6, 168.5);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(514.1, 163.7);
            let _ = robot.mouse_move(519.5, 159.0);
            std::thread::sleep(Duration::from_millis(84));
            let _ = robot.mouse_move(524.2, 154.3);
            let _ = robot.mouse_move(528.7, 149.8);
            let _ = robot.mouse_move(532.6, 144.5);
            let _ = robot.mouse_move(535.8, 140.1);
            let _ = robot.mouse_move(538.1, 136.7);
            let _ = robot.mouse_move(540.2, 134.1);
            let _ = robot.mouse_move(541.7, 131.6);
            let _ = robot.mouse_move(542.6, 129.2);
            let _ = robot.mouse_move(544.0, 127.3);
            let _ = robot.mouse_move(545.4, 125.4);
            let _ = robot.mouse_move(546.3, 123.1);
            let _ = robot.mouse_move(547.7, 121.7);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(548.7, 120.3);
            std::thread::sleep(Duration::from_millis(53));
            let _ = robot.mouse_move(549.1, 119.4);
            let _ = robot.mouse_move(549.6, 118.5);
            let _ = robot.mouse_move(550.0, 117.6);
            let _ = robot.mouse_move(550.5, 116.7);
            let _ = robot.mouse_move(550.5, 115.3);
            std::thread::sleep(Duration::from_millis(11));
            let _ = robot.mouse_move(551.4, 113.9);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(551.9, 113.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(552.8, 112.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(553.7, 110.7);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(554.7, 109.3);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(555.6, 107.4);
            std::thread::sleep(Duration::from_millis(66));
            let _ = robot.mouse_move(556.5, 106.0);
            let _ = robot.mouse_move(557.4, 104.2);
            let _ = robot.mouse_move(558.8, 102.3);
            let _ = robot.mouse_move(560.2, 100.5);
            let _ = robot.mouse_move(561.7, 97.6);
            let _ = robot.mouse_move(563.1, 95.6);
            let _ = robot.mouse_move(564.6, 93.2);
            let _ = robot.mouse_move(566.0, 91.3);
            let _ = robot.mouse_move(567.4, 89.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(568.8, 87.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(569.7, 85.8);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(571.1, 83.9);
            std::thread::sleep(Duration::from_millis(70));
            let _ = robot.mouse_move(572.5, 82.1);
            let _ = robot.mouse_move(573.4, 80.2);
            let _ = robot.mouse_move(573.8, 78.4);
            let _ = robot.mouse_move(574.8, 77.0);
            let _ = robot.mouse_move(574.8, 75.6);
            let _ = robot.mouse_move(575.2, 74.2);
            let _ = robot.mouse_move(575.7, 73.3);
            let _ = robot.mouse_move(576.6, 71.9);
            let _ = robot.mouse_move(577.1, 70.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(577.1, 69.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(577.5, 68.7);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(577.5, 67.8);
            std::thread::sleep(Duration::from_millis(58));
            let _ = robot.mouse_move(577.5, 66.4);
            let _ = robot.mouse_move(577.5, 65.5);
            let _ = robot.mouse_move(577.5, 64.5);
            std::thread::sleep(Duration::from_millis(6));
            let _ = robot.mouse_move(577.5, 63.6);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(578.0, 62.7);
            std::thread::sleep(Duration::from_millis(77));
            let _ = robot.mouse_move(578.0, 62.0);
            std::thread::sleep(Duration::from_millis(91));
            let _ = robot.mouse_move(577.8, 61.2);
            std::thread::sleep(Duration::from_millis(7));
            let _ = robot.mouse_down();
            std::thread::sleep(Duration::from_millis(9));
            let _ = robot.mouse_move(576.8, 61.2);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(575.0, 61.2);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(573.6, 60.8);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(571.8, 60.8);
            std::thread::sleep(Duration::from_millis(9));
            let _ = robot.mouse_move(569.5, 60.8);
            std::thread::sleep(Duration::from_millis(70));
            let _ = robot.mouse_move(566.0, 60.8);
            let _ = robot.mouse_move(560.0, 60.8);
            let _ = robot.mouse_move(551.7, 60.8);
            let _ = robot.mouse_move(540.9, 60.8);
            let _ = robot.mouse_move(529.5, 60.8);
            let _ = robot.mouse_move(518.1, 61.6);
            let _ = robot.mouse_move(506.7, 61.6);
            let _ = robot.mouse_move(494.1, 62.4);
            let _ = robot.mouse_up();
            std::thread::sleep(Duration::from_millis(40));
            let tab_label = "Counter App";
            let before_fling_x = tab_x(&robot, tab_label);
            std::thread::sleep(Duration::from_millis(200));
            let after_fling_x = tab_x(&robot, tab_label);

            match (before_fling_x, after_fling_x) {
                (Some(before), Some(after)) => {
                    let delta = (after - before).abs();
                    if delta < 8.0 {
                        eprintln!(
                            "✗ No fling after drag: '{}' moved {:.1}px (expected > 8px)",
                            tab_label, delta
                        );
                        std::process::exit(1);
                    }
                }
                _ => {
                    eprintln!("✗ No fling assert: could not locate '{}' tab", tab_label);
                    std::process::exit(1);
                }
            }
            let _ = robot.mouse_move(481.3, 63.3);
            let _ = robot.mouse_move(468.5, 64.1);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(454.5, 65.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(440.2, 65.9);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(428.2, 66.8);
            std::thread::sleep(Duration::from_millis(67));
            let _ = robot.mouse_move(417.9, 67.6);
            let _ = robot.mouse_move(411.6, 68.3);
            let _ = robot.mouse_move(406.9, 68.8);
            let _ = robot.mouse_move(404.3, 69.4);
            let _ = robot.mouse_move(402.4, 69.8);
            let _ = robot.mouse_move(401.5, 70.3);
            let _ = robot.mouse_move(401.0, 71.2);
            std::thread::sleep(Duration::from_millis(21));
            let _ = robot.mouse_move(401.0, 72.0);
            std::thread::sleep(Duration::from_millis(61));
            let _ = robot.mouse_move(401.0, 72.9);
            let _ = robot.mouse_move(400.2, 73.3);
            let _ = robot.mouse_move(399.3, 73.8);
            std::thread::sleep(Duration::from_millis(40));
            let _ = robot.mouse_move(398.6, 74.2);
            std::thread::sleep(Duration::from_millis(24));
            let _ = robot.mouse_move(397.6, 75.0);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(396.7, 75.5);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(395.8, 75.9);
            std::thread::sleep(Duration::from_millis(24));
            let _ = robot.mouse_move(394.9, 75.9);
            std::thread::sleep(Duration::from_millis(16));
            let _ = robot.mouse_move(394.0, 76.4);
            std::thread::sleep(Duration::from_millis(95));
            let _ = robot.mouse_move(393.2, 76.4);
            std::thread::sleep(Duration::from_millis(201));
            let _ = robot.mouse_move(395.0, 76.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(395.9, 76.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(397.3, 75.7);
            std::thread::sleep(Duration::from_millis(33));
            let _ = robot.mouse_move(398.7, 75.7);
            std::thread::sleep(Duration::from_millis(224));
            let _ = robot.mouse_move(399.5, 76.5);
            std::thread::sleep(Duration::from_millis(7));
            let _ = robot.mouse_move(402.0, 76.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(408.8, 74.1);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(420.1, 67.6);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(428.9, 66.0);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(444.9, 64.3);
            std::thread::sleep(Duration::from_millis(53));
            let _ = robot.mouse_move(465.2, 64.3);
            let _ = robot.mouse_move(489.2, 64.3);
            let _ = robot.mouse_move(516.0, 62.5);
            let _ = robot.mouse_move(542.8, 59.7);
            let _ = robot.mouse_move(568.6, 56.9);
            let _ = robot.mouse_move(594.5, 55.1);
            let _ = robot.mouse_move(620.3, 53.2);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(644.3, 50.5);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(668.3, 47.7);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(689.5, 44.9);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(709.8, 40.3);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(727.4, 34.8);
            std::thread::sleep(Duration::from_millis(8));
            let _ = robot.mouse_move(743.1, 29.2);
            std::thread::sleep(Duration::from_millis(67));
            let _ = robot.mouse_move(755.9, 22.8);
            let _ = robot.mouse_move(764.9, 17.9);
            let _ = robot.mouse_move(771.4, 13.6);
            let _ = robot.mouse_move(776.7, 9.6);
            let _ = robot.mouse_move(780.5, 5.8);
            let _ = robot.mouse_move(783.4, 2.3);

            std::thread::sleep(Duration::from_secs(1));
            let _ = robot.exit();
        })
        .run(|| {
            desktop_app::app::combined_app();
        });
}
