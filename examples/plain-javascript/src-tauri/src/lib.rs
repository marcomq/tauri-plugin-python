// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet_rust(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet_rust])
        .plugin(tauri_plugin_python::init(vec!["greet_python"]))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
