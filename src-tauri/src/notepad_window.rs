use tauri::{AppHandle, Manager, WebviewWindowBuilder};

/// Shows the notepad window, creating it the first time and focusing it on
/// every call after. Unlike the recording overlay this is an ordinary
/// decorated, resizable window — it's a workspace the user keeps around
/// alongside other apps, not a transient HUD. Shared by the explicit
/// "Open Notepad" command and the auto-open-on-capture path in actions.rs.
pub fn show_notepad_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("notepad") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    let mut builder = WebviewWindowBuilder::new(
        app,
        "notepad",
        tauri::WebviewUrl::App("src/notepad/index.html".into()),
    )
    .title("Notepad")
    .inner_size(1100.0, 720.0)
    .min_inner_size(760.0, 480.0)
    .resizable(true)
    .maximizable(true);

    if let Some(data_dir) = crate::portable::data_dir() {
        builder = builder.data_directory(data_dir.join("webview"));
    }

    builder.build().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub fn open_notepad_window(app: AppHandle) -> Result<(), String> {
    show_notepad_window(&app)
}
