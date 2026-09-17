// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn download_video(
    url: String,
    quality: String,
    profile: String,
    output_folder: String,
) -> String {
    println!("URL: {}", url);
    println!("Calidad: {}", quality);
    println!("Perfil: {}", profile);
    println!("Carpeta: {}", output_folder);
    "Datos recibidos correctamente".to_string()
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![download_video])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
