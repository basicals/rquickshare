//! Constant-size latest invalidation; subscribed streams are always drained.

use futures::StreamExt;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use zbus::message::Type as MessageType;
use zbus::{MatchRule, MessageStream};

use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Hint {
    pub epoch: Option<u128>,
    pub revision: u64,
    pub settings_generation: u64,
    pub failure: Option<ClientError>,
}

pub(super) struct WatchOwner {
    pub receiver: watch::Receiver<Hint>,
    pub stop: CancellationToken,
    pub task: Option<JoinHandle<()>>,
}

fn hint(message: &Message, owner: &str) -> Result<(u128, u64)> {
    reply_sender(message, owner)?;
    let header = message.header();
    if message.message_type() != MessageType::Signal
        || header.path().is_none_or(|v| v.as_str() != PATH)
        || header.interface().is_none_or(|v| v.as_str() != INTERFACE)
        || header.member().is_none_or(|v| v.as_str() != "StateChanged")
    {
        return Err(ClientError::Protocol);
    }
    let (value, revision): (String, u64) = message
        .body()
        .deserialize()
        .map_err(|_| ClientError::Protocol)?;
    if revision == 0 {
        return Err(ClientError::Protocol);
    }
    Ok((epoch(&value)?, revision))
}
fn changed_owner(message: &Message, owner: &str) -> Result<bool> {
    reply_sender(message, "org.freedesktop.DBus")?;
    let header = message.header();
    if message.message_type() != MessageType::Signal
        || header
            .path()
            .is_none_or(|v| v.as_str() != "/org/freedesktop/DBus")
        || header
            .interface()
            .is_none_or(|v| v.as_str() != "org.freedesktop.DBus")
        || header
            .member()
            .is_none_or(|v| v.as_str() != "NameOwnerChanged")
    {
        return Err(ClientError::Protocol);
    }
    let (name, _, new): (String, String, String) = message
        .body()
        .deserialize()
        .map_err(|_| ClientError::Protocol)?;
    if name != SERVICE {
        return Err(ClientError::Protocol);
    }
    Ok(new != owner)
}

impl WatchOwner {
    pub async fn subscribe(connection: &Connection, owner: &str) -> Result<Self> {
        let state_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(owner)
            .map_err(|_| ClientError::Protocol)?
            .path(PATH)
            .map_err(|_| ClientError::Protocol)?
            .interface(INTERFACE)
            .map_err(|_| ClientError::Protocol)?
            .member("StateChanged")
            .map_err(|_| ClientError::Protocol)?
            .build();
        let owner_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender("org.freedesktop.DBus")
            .map_err(|_| ClientError::Protocol)?
            .path("/org/freedesktop/DBus")
            .map_err(|_| ClientError::Protocol)?
            .interface("org.freedesktop.DBus")
            .map_err(|_| ClientError::Protocol)?
            .member("NameOwnerChanged")
            .map_err(|_| ClientError::Protocol)?
            .arg(0, SERVICE)
            .map_err(|_| ClientError::Protocol)?
            .build();
        let mut states = MessageStream::for_match_rule(state_rule, connection, Some(8))
            .await
            .map_err(|_| ClientError::Transport)?;
        let settings_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(owner)
            .map_err(|_| ClientError::Protocol)?
            .path(PATH)
            .map_err(|_| ClientError::Protocol)?
            .interface(crate::service::settings_service::INTERFACE)
            .map_err(|_| ClientError::Protocol)?
            .member("SettingsChanged")
            .map_err(|_| ClientError::Protocol)?
            .build();
        let mut settings = MessageStream::for_match_rule(settings_rule, connection, Some(8))
            .await
            .map_err(|_| ClientError::Transport)?;
        let visibility_rule = MatchRule::builder()
            .msg_type(MessageType::Signal)
            .sender(owner)
            .map_err(|_| ClientError::Protocol)?
            .path(PATH)
            .map_err(|_| ClientError::Protocol)?
            .interface(crate::service::visibility_service::INTERFACE)
            .map_err(|_| ClientError::Protocol)?
            .member("Changed")
            .map_err(|_| ClientError::Protocol)?
            .build();
        let mut visibility = MessageStream::for_match_rule(visibility_rule, connection, Some(8))
            .await
            .map_err(|_| ClientError::Transport)?;
        let mut owners = MessageStream::for_match_rule(owner_rule, connection, Some(8))
            .await
            .map_err(|_| ClientError::Transport)?;
        let (sender, receiver) = watch::channel(Hint::default());
        let stop = CancellationToken::new();
        let end = stop.clone();
        let owner = owner.to_owned();
        let task = tokio::spawn(async move {
            loop {
                let update = tokio::select! {
                    biased;
                    _=end.cancelled()=>break,
                    message=owners.next()=>match message {
                        Some(Ok(message))=>changed_owner(&message,&owner).map(|changed|if changed {Some((None,0,Some(ClientError::DaemonChanged),false))} else {None}),
                        _=>Err(ClientError::Transport),
                    },
                    message=states.next()=>match message {
                        Some(Ok(message))=>hint(&message,&owner).map(|(epoch,revision)|Some((Some(epoch),revision,None,false))),
                        _=>Err(ClientError::Transport),
                    },
                    message=visibility.next()=>match message {
                        Some(Ok(message))=>policy_hint(&message,&owner).map(|epoch|Some((Some(epoch),0,None,true))),
                        _=>Err(ClientError::Transport),
                    },
                    message=settings.next()=>match message {
                        Some(Ok(message))=>settings_hint(&message,&owner).map(|epoch|Some((Some(epoch),0,None,true))),
                        _=>Err(ClientError::Transport),
                    },
                };
                let update = match update {
                    Ok(None) => continue,
                    Ok(Some(update)) => update,
                    Err(error) => (None, 0, Some(error), false),
                };
                sender.send_modify(|hint| {
                    if hint.failure.is_some() {
                        return;
                    }
                    if let Some(error) = update.2 {
                        hint.failure = Some(error);
                        return;
                    }
                    if hint.epoch.is_some() && hint.epoch != update.0 {
                        hint.failure = Some(ClientError::DaemonChanged);
                        return;
                    }
                    hint.epoch = update.0;
                    hint.revision = hint.revision.max(update.1);
                    if update.3 {
                        match hint.settings_generation.checked_add(1) {
                            Some(generation) => hint.settings_generation = generation,
                            None => hint.failure = Some(ClientError::Exhausted),
                        }
                    }
                });
                if sender.borrow().failure.is_some() {
                    break;
                }
            }
        });
        Ok(Self {
            receiver,
            stop,
            task: Some(task),
        })
    }
}

fn settings_hint(message: &Message, owner: &str) -> Result<u128> {
    invalidation_hint(
        message,
        owner,
        crate::service::settings_service::INTERFACE,
        "SettingsChanged",
    )
}
fn policy_hint(message: &Message, owner: &str) -> Result<u128> {
    invalidation_hint(
        message,
        owner,
        crate::service::visibility_service::INTERFACE,
        "Changed",
    )
}
fn invalidation_hint(
    message: &Message,
    owner: &str,
    interface: &str,
    member: &str,
) -> Result<u128> {
    reply_sender(message, owner)?;
    let header = message.header();
    if message.message_type() != MessageType::Signal
        || header.path().is_none_or(|value| value.as_str() != PATH)
        || header
            .interface()
            .is_none_or(|value| value.as_str() != interface)
        || header.member().is_none_or(|value| value.as_str() != member)
        || !message.data().fds().is_empty()
    {
        return Err(ClientError::Protocol);
    }
    let (value, _): (String, u64) = message
        .body()
        .deserialize()
        .map_err(|_| ClientError::Protocol)?;
    epoch(&value)
}

#[cfg(test)]
mod tests;
