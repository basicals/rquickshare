use std::path::PathBuf;

use crate::AppState;

#[tauri::command]
pub fn change_download_path(
    message: Option<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    info!("Download directory update requested");

    let rqs = state
        .rqs
        .lock()
        .map_err(|_| "RQuickShare state lock is poisoned".to_owned())?;
    rqs.set_download_path(message.map(PathBuf::from));
    Ok(())
}
