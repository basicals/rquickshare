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
                        Some(Ok(message))=>changed_owner(&message,&owner).map(|changed|if changed {Some((None,0,Some(ClientError::DaemonChanged)))} else {None}),
                        _=>Err(ClientError::Transport),
                    },
                    message=states.next()=>match message {
                        Some(Ok(message))=>hint(&message,&owner).map(|(epoch,revision)|Some((Some(epoch),revision,None))),
                        _=>Err(ClientError::Transport),
                    },
                };
                let update = match update {
                    Ok(None) => continue,
                    Ok(Some(update)) => update,
                    Err(error) => (None, 0, Some(error)),
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

#[cfg(test)]
mod tests;
