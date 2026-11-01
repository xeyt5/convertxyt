mod job;
mod media;
mod tools;

use job::JobManager;
use media::{BatchSummary, JobResult, Progress};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State};

/// Corre `work` fuera del hilo de la interfaz y reenvía su progreso al frontend.
async fn run_job<T: Send + 'static>(
    app: AppHandle,
    manager: JobManager,
    work: impl FnOnce(&JobManager, &mut dyn FnMut(Progress)) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = manager.begin()?;
        work(&manager, &mut |progress| {
            let _ = app.emit("job-progress", progress);
        })
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
async fn download_video(
    app: AppHandle,
    manager: State<'_, JobManager>,
    url: String,
    quality: String,
    profile: String,
    output_folder: String,
) -> JobResult {
    run_job(app, manager.inner().clone(), move |manager, emit| {
        media::download_video(manager, &url, &quality, &profile, &output_folder, emit)
    })
    .await
}

#[tauri::command]
async fn download_audio(
    app: AppHandle,
    manager: State<'_, JobManager>,
    url: String,
    format: String,
    output_folder: String,
) -> JobResult {
    run_job(app, manager.inner().clone(), move |manager, emit| {
        media::download_audio(manager, &url, &format, &output_folder, emit)
    })
    .await
}

#[tauri::command]
async fn convert_videos(
    app: AppHandle,
    manager: State<'_, JobManager>,
    inputs: Vec<String>,
    format: String,
    output_folder: String,
) -> Result<Option<BatchSummary>, String> {
    run_job(app, manager.inner().clone(), move |manager, emit| {
        media::convert_videos(manager, &inputs, &format, &output_folder, emit)
    })
    .await
}

#[tauri::command]
fn list_videos(folder: String) -> Result<Vec<String>, String> {
    media::list_videos(&folder)
}

#[tauri::command]
async fn extract_audio(
    app: AppHandle,
    manager: State<'_, JobManager>,
    input: String,
    format: String,
    output_folder: String,
) -> JobResult {
    run_job(app, manager.inner().clone(), move |manager, emit| {
        media::extract_audio(manager, &input, &format, &output_folder, emit)
    })
    .await
}

#[tauri::command]
fn cancel_job(manager: State<'_, JobManager>) {
    manager.cancel();
}

#[tauri::command]
fn can_self_update() -> bool {
    tools::is_packaged()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Dentro de un AppImage, el renderizador DMA-BUF de WebKitGTK falla con las bibliotecas
    // gráficas de algunos equipos y puede dejar la ventana en blanco.
    #[cfg(target_os = "linux")]
    if tools::is_packaged() && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    tauri::Builder::default()
        .manage(JobManager::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle()
                    .plugin(tauri_plugin_updater::Builder::new().build())?;
                app.handle().plugin(tauri_plugin_process::init())?;
            }
            tools::init(app.path().app_data_dir().ok());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            download_video,
            download_audio,
            convert_videos,
            list_videos,
            extract_audio,
            cancel_job,
            can_self_update
        ])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app, event| {
            // Cerrar la ventana no debe dejar una descarga corriendo en segundo plano.
            if let RunEvent::Exit = event {
                app.state::<JobManager>().cancel();
            }
        });
}
