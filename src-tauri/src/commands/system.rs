#[tauri::command]
pub fn ping() -> &'static str {
    "pong from Rust"
}