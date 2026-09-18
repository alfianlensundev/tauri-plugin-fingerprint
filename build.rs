const COMMANDS: &[&str] = &[
    "check_device",
    "get_device_info",
    "capture",
    "capture_frames",
    "enroll",
    "verify",
    "identify",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
