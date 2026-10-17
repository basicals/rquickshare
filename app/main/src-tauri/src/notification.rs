#[cfg(target_os = "linux")]
use notify_rust::Notification;
use rqs_lib::channel::ChannelMessage;
#[cfg(target_os = "linux")]
use rqs_lib::{
    channel::{ChannelAction, ChannelDirection},
    Visibility,
};
use tauri::AppHandle;
#[cfg(target_os = "linux")]
use tauri::Manager;
#[cfg(target_os = "linux")]
use tauri_plugin_clipboard_manager::ClipboardExt;
#[cfg(not(target_os = "linux"))]
use tauri_plugin_notification::NotificationExt;

#[cfg(target_os = "linux")]
use crate::cmds;

const MAX_NOTIFICATION_PREVIEW_CHARS: usize = 180;

fn bounded_preview(value: &str) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = compact.chars();
    let preview: String = chars
        .by_ref()
        .take(MAX_NOTIFICATION_PREVIEW_CHARS)
        .collect();

    if chars.next().is_some() {
        format!("{preview}…")
    } else {
        preview
    }
}

fn sender_name(message: &ChannelMessage) -> String {
    message
        .meta
        .as_ref()
        .and_then(|meta| meta.source.as_ref())
        .map(|source| source.name.clone())
        .unwrap_or_else(|| "Unknown device".to_owned())
}

fn request_body(message: &ChannelMessage) -> String {
    let name = sender_name(message);
    let Some(meta) = message.meta.as_ref() else {
        return format!("{name} wants to initiate a transfer");
    };

    if let Some(files) = meta.files.as_ref().filter(|files| !files.is_empty()) {
        if files.len() == 1 {
            return format!("{name} wants to send:\n{}", bounded_preview(&files[0]));
        }
        return format!("{name} wants to send {} files", files.len());
    }

    if let Some(description) = meta
        .text_description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
    {
        return format!("{name} wants to share:\n{}", bounded_preview(description));
    }

    format!("{name} wants to initiate a transfer")
}

fn finished_body(message: &ChannelMessage) -> String {
    let name = sender_name(message);
    let Some(meta) = message.meta.as_ref() else {
        return format!("Transfer from {name} completed");
    };

    if let Some(text) = meta
        .text_payload
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        return format!("Text received from {name}:\n{}", bounded_preview(text));
    }

    if let Some(files) = meta.files.as_ref().filter(|files| !files.is_empty()) {
        if files.len() == 1 {
            return format!("Received from {name}:\n{}", bounded_preview(&files[0]));
        }
        return format!("Received {} files from {name}", files.len());
    }

    format!("Transfer from {name} completed")
}

pub fn send_request_notification(message: &ChannelMessage, app_handle: &AppHandle) {
    let body = request_body(message);

    #[cfg(not(target_os = "linux"))]
    let _ = app_handle
        .notification()
        .builder()
        .title("RQuickShare")
        .body(&body)
        .show();

    #[cfg(target_os = "linux")]
    match Notification::new()
        .summary("RQuickShare")
        .body(&body)
        .action("accept", "Accept")
        .action("reject", "Reject")
        .show()
    {
        Ok(notification) => {
            let capp_handle = app_handle.clone();
            let id = message.id.clone();
            tokio::task::spawn_blocking(move || {
                notification.wait_for_action(|action| match action {
                    "accept" => {
                        let _ = cmds::send_to_rs(
                            ChannelMessage {
                                id: id.clone(),
                                direction: ChannelDirection::FrontToLib,
                                action: Some(ChannelAction::AcceptTransfer),
                                ..Default::default()
                            },
                            capp_handle.state(),
                        );
                    }
                    "reject" => {
                        let _ = cmds::send_to_rs(
                            ChannelMessage {
                                id: id.clone(),
                                direction: ChannelDirection::FrontToLib,
                                action: Some(ChannelAction::RejectTransfer),
                                ..Default::default()
                            },
                            capp_handle.state(),
                        );
                    }
                    _ => (),
                });
            });
        }
        Err(error) => {
            error!("Couldn't show request notification: {error}");
        }
    }
}

pub fn send_finished_notification(message: &ChannelMessage, app_handle: &AppHandle) {
    let body = finished_body(message);

    #[cfg(not(target_os = "linux"))]
    let _ = app_handle
        .notification()
        .builder()
        .title("RQuickShare")
        .body(&body)
        .show();

    #[cfg(target_os = "linux")]
    {
        let text_payload = message
            .meta
            .as_ref()
            .and_then(|meta| meta.text_payload.clone());
        let first_file = message
            .meta
            .as_ref()
            .and_then(|meta| meta.files.as_ref())
            .and_then(|files| files.first())
            .cloned();

        let mut builder = Notification::new();
        builder.summary("RQuickShare").body(&body);

        if text_payload.is_some() {
            builder.action("copy", "Copy").action("dismiss", "Dismiss");
        } else if first_file.is_some() {
            builder
                .action("show-file", "Show file")
                .action("dismiss", "Dismiss");
        }

        match builder.show() {
            Ok(notification) => {
                let capp_handle = app_handle.clone();
                tokio::task::spawn_blocking(move || {
                    notification.wait_for_action(|action| match action {
                        "copy" => {
                            if let Some(text) = text_payload.as_ref() {
                                if let Err(error) = capp_handle.clipboard().write_text(text.clone())
                                {
                                    error!(
                                        "Couldn't copy received text from notification: {error}"
                                    );
                                }
                            }
                        }
                        "show-file" => {
                            if let Some(file_name) = first_file.as_ref() {
                                if let Err(error) = cmds::reveal_download_item(
                                    capp_handle.clone(),
                                    file_name.clone(),
                                ) {
                                    error!(
                                        "Couldn't reveal received file from notification: {error}"
                                    );
                                }
                            }
                        }
                        _ => (),
                    });
                });
            }
            Err(error) => {
                error!("Couldn't show completion notification: {error}");
            }
        }
    }
}

pub fn send_temporarily_notification(app_handle: &AppHandle) {
    let body = "RQuickShare is temporarily hidden".to_string();

    #[cfg(not(target_os = "linux"))]
    let _ = app_handle
        .notification()
        .builder()
        .title("RQuickShare")
        .body(&body)
        .show();

    #[cfg(target_os = "linux")]
    match Notification::new()
        .summary("RQuickShare")
        .body(&body)
        .action("visible", "Be visible (1m)")
        .action("ignore", "Ignore")
        .id(1919)
        .show()
    {
        Ok(notification) => {
            let capp_handle = app_handle.clone();
            tokio::task::spawn_blocking(move || {
                notification.wait_for_action(|action| match action {
                    "visible" => {
                        if let Err(error) = cmds::change_visibility(
                            Visibility::Temporarily,
                            capp_handle.state(),
                            capp_handle.clone(),
                        ) {
                            error!("Couldn't change visibility from notification: {error}");
                        }
                    }
                    "ignore" => {}
                    _ => (),
                });
            });
        }
        Err(error) => {
            error!("Couldn't show notification: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_preview_is_utf8_safe_and_bounded() {
        let input = "é".repeat(MAX_NOTIFICATION_PREVIEW_CHARS + 5);
        let preview = bounded_preview(&input);

        assert_eq!(preview.chars().count(), MAX_NOTIFICATION_PREVIEW_CHARS + 1);
        assert!(preview.ends_with('…'));
    }

    #[test]
    fn bounded_preview_compacts_whitespace() {
        assert_eq!(
            bounded_preview("hello\n\tworld   again"),
            "hello world again"
        );
    }
}
