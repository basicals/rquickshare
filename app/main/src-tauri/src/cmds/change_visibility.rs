use rqs_lib::{Visibility, RQS};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use crate::AppState;

#[tauri::command]
pub fn change_visibility(
    message: Visibility,
    state: tauri::State<'_, AppState>,
    app: AppHandle,
) -> Result<(), String> {
    info!("change_visibility: {message:?}");

    let mut rqs = state
        .rqs
        .lock()
        .map_err(|_| "RQuickShare state lock is poisoned".to_owned())?;
    apply_visibility(&mut rqs, &state.visibility_recovered, message, || {
        crate::store::set_visibility(&app, message)
            .map_err(|_| "Unable to save receiving visibility".to_owned())
    })
}

fn apply_visibility(
    rqs: &mut RQS,
    recovered: &AtomicBool,
    visibility: Visibility,
    persist: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    persist()?;
    rqs.change_visibility(visibility);
    recovered.store(false, Ordering::Release);
    Ok(())
}

#[tauri::command]
pub fn get_receiving_status(
    state: tauri::State<'_, AppState>,
) -> Result<(Visibility, bool), String> {
    let sender = state
        .visibility_sender
        .lock()
        .map_err(|_| "RQuickShare state lock is poisoned".to_owned())?;
    let visibility = *sender.borrow();
    Ok((
        visibility,
        state.visibility_recovered.load(Ordering::Acquire),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn given_hidden_recovery_when_save_fails_then_core_and_warning_remain_unchanged() {
        // Arrange
        let mut core = RQS::new(Visibility::Invisible, None, None);
        let warning = AtomicBool::new(true);
        // Act
        let result = apply_visibility(&mut core, &warning, Visibility::Visible, || {
            Err("write failed".into())
        });
        // Assert
        assert_eq!(result, Err("write failed".into()));
        assert_eq!(
            *core.visibility_sender.lock().unwrap().borrow(),
            Visibility::Invisible
        );
        assert!(warning.load(Ordering::Acquire));
    }

    #[test]
    fn given_hidden_recovery_when_choice_is_saved_then_core_changes_and_warning_clears_after_persistence(
    ) {
        for visibility in [
            Visibility::Invisible,
            Visibility::Temporarily,
            Visibility::Visible,
        ] {
            // Arrange
            let mut core = RQS::new(Visibility::Invisible, None, None);
            let warning = AtomicBool::new(true);
            let sender = core.visibility_sender.clone();
            // Act
            apply_visibility(&mut core, &warning, visibility, || {
                assert_eq!(*sender.lock().unwrap().borrow(), Visibility::Invisible);
                assert!(warning.load(Ordering::Acquire));
                Ok(())
            })
            .unwrap();
            // Assert
            assert_eq!(*core.visibility_sender.lock().unwrap().borrow(), visibility);
            assert!(!warning.load(Ordering::Acquire));
        }
    }
}
