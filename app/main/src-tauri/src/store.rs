use std::{path::PathBuf, sync::Arc};

use rqs_lib::Visibility;
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_store::{Store, StoreExt};

fn get_store(app_handle: &AppHandle) -> Result<Arc<Store<Wry>>, anyhow::Error> {
    app_handle
        .store_builder(".settings.json")
        .disable_auto_save()
        .build()
        .map_err(|error| anyhow::anyhow!("unable to open settings store: {error}"))
}

pub fn init_default(app_handle: &AppHandle) {
    let store = match get_store(app_handle) {
        Ok(store) => store,
        Err(error) => {
            error!("init_default: {error}");
            return;
        }
    };

    let mut changed = false;
    if !store.has("autostart") {
        store.set("autostart", false);
        changed = true;
    }

    if !store.has("realclose") {
        store.set("realclose", false);
        changed = true;
    }

    if !store.has("visibility") {
        store.set("visibility", Visibility::Visible as u8);
        changed = true;
    }

    if !store.has("startminimized") {
        store.set("startminimized", false);
        changed = true;
    }
    if changed && store.save().is_err() {
        warn!("Unable to save initial default settings");
    }
}

pub fn startup_settings(
    app_handle: &AppHandle,
) -> Result<rqs_lib::settings::StartupSettings, anyhow::Error> {
    let path = app_handle.path().app_data_dir()?.join(".settings.json");
    rqs_lib::settings::StartupSettings::read(&path).map_err(Into::into)
}

pub fn get_realclose(app_handle: &AppHandle) -> bool {
    get_store(app_handle)
        .ok()
        .and_then(|store| store.get("realclose"))
        .and_then(|json| json.as_bool())
        .unwrap_or_default()
}

pub fn get_visibility(app_handle: &AppHandle) -> Visibility {
    match get_store(app_handle) {
        Ok(store) => rqs_lib::settings::visibility_setting(store.get("visibility").as_ref()).0,
        Err(_) => Visibility::Invisible,
    }
}

pub fn set_visibility(app_handle: &AppHandle, visibility: Visibility) -> Result<(), anyhow::Error> {
    let store = get_store(app_handle)?;

    let previous = store.get("visibility");
    store.set("visibility", visibility as u8);
    if let Err(error) = store.save() {
        match previous {
            Some(value) => store.set("visibility", value),
            None => {
                store.delete("visibility");
            }
        }
        return Err(error.into());
    }
    if app_handle.emit("visibility_updated", ()).is_err() {
        warn!("Unable to publish receiving visibility update");
    }

    Ok(())
}

pub fn get_download_path(app_handle: &AppHandle) -> Option<PathBuf> {
    get_store(app_handle)
        .ok()
        .and_then(|store| store.get("download_path"))
        .and_then(|json| json.as_str().map(PathBuf::from))
}

pub fn get_logging_level(app_handle: &AppHandle) -> Option<String> {
    get_store(app_handle)
        .ok()
        .and_then(|store| store.get("debug_level"))
        .and_then(|json| json.as_str().map(String::from))
}

pub fn get_startminimized(app_handle: &AppHandle) -> bool {
    get_store(app_handle)
        .ok()
        .and_then(|store| store.get("startminimized"))
        .and_then(|json| json.as_bool())
        .unwrap_or_default()
}
