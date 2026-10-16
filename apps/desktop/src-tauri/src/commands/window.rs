use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

/// Open a new application window.
#[tauri::command]
pub async fn open_new_window(app: AppHandle) -> Result<String, String> {
    let label = format!("window-{}", uuid::Uuid::new_v4());
    let url = WebviewUrl::App("index.html".into());

    WebviewWindowBuilder::new(&app, &label, url)
        .title("ApiArk Internal")
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .traffic_light_position(tauri::LogicalPosition::new(16.0, 20.0))
        .inner_size(1280.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .build()
        .map_err(|e| format!("Failed to create window: {e}"))?;

    Ok(label)
}
