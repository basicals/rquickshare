//! Owned descriptor-only containing-folder request. Never launches a payload handler.
use futures::StreamExt;
use rand::TryRngCore;
use std::{collections::HashMap, fs::File, os::fd::AsFd, sync::Arc};
use tokio_util::sync::CancellationToken;
use zbus::{
    message::{Flags, Type},
    zvariant::{Fd, OwnedObjectPath, OwnedValue, Value},
    Connection, MatchRule, Message, MessageStream,
};
const SERVICE: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.OpenURI";
const REQUEST: &str = "org.freedesktop.portal.Request";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Unavailable,
    Unsupported,
    Protocol,
    Rejected,
    Cancelled,
}
fn checked(message: &Message, owner: &str) -> Result<(), Error> {
    if message
        .header()
        .sender()
        .is_none_or(|name| name.as_str() != owner)
        || !message.data().fds().is_empty()
    {
        Err(Error::Protocol)
    } else {
        Ok(())
    }
}
pub async fn show(file: Arc<File>, cancel: CancellationToken) -> Result<(), Error> {
    show_mode(file, cancel, false).await
}
pub async fn show_directory_contents(
    file: Arc<File>,
    cancel: CancellationToken,
) -> Result<(), Error> {
    // OpenFile can execute a regular file's handler; this path admits directories only.
    let flags = rustix::fs::fcntl_getfl(&*file).map_err(|_| Error::Protocol)?;
    if !file.metadata().map_err(|_| Error::Protocol)?.is_dir()
        || flags.contains(rustix::fs::OFlags::PATH)
        || flags & rustix::fs::OFlags::ACCMODE != rustix::fs::OFlags::RDONLY
    {
        return Err(Error::Protocol);
    }
    show_mode(file, cancel, true).await
}
async fn show_mode(
    file: Arc<File>,
    cancel: CancellationToken,
    contents: bool,
) -> Result<(), Error> {
    let connect = async {
        let directory =
            rqs_lib::service::process::runtime_directory().map_err(|_| Error::Unavailable)?;
        let address = rqs_lib::service::process::BusAddress::session(&directory)
            .map_err(|_| Error::Unavailable)?;
        rqs_lib::service::bus::connection_builder(
            address.connect().await.map_err(|_| Error::Unavailable)?,
        )
        .build()
        .await
        .map_err(|_| Error::Unavailable)
    };
    let connection = tokio::select! { biased; _ = cancel.cancelled() => return Err(Error::Cancelled), result = connect => result? };
    if contents {
        show_on_mode(connection, file, cancel, true).await
    } else {
        show_on(connection, file, cancel).await
    }
}
async fn show_on(
    connection: Connection,
    file: Arc<File>,
    cancel: CancellationToken,
) -> Result<(), Error> {
    show_on_mode(connection, file, cancel, false).await
}
async fn show_on_mode(
    connection: Connection,
    file: Arc<File>,
    cancel: CancellationToken,
    contents: bool,
) -> Result<(), Error> {
    let mut pending: Option<(String, String)> = None;
    let result = tokio::select! { biased;
        _ = cancel.cancelled() => Err(Error::Cancelled),
        result = interaction(&connection, &file, &mut pending, contents) => result,
    };
    if let Some((owner, path)) = pending {
        // Close has no Response signal; do not wait for an unresponsive Close reply.
        if let Ok(message) = Message::method_call(path.as_str(), "Close")
            .and_then(|builder| builder.destination(owner.as_str()))
            .and_then(|builder| builder.interface(REQUEST))
            .and_then(|builder| builder.with_flags(Flags::NoReplyExpected))
            .and_then(|builder| builder.build(&()))
        {
            let _ = connection.send(&message).await;
        }
    }
    let closed = connection.close().await;
    if closed.is_err() && result.is_ok() {
        return Err(Error::Unavailable);
    }
    result
}
async fn interaction(
    connection: &Connection,
    file: &File,
    pending: &mut Option<(String, String)>,
    contents: bool,
) -> Result<(), Error> {
    let owner_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender("org.freedesktop.DBus")
        .map_err(|_| Error::Protocol)?
        .path("/org/freedesktop/DBus")
        .map_err(|_| Error::Protocol)?
        .interface("org.freedesktop.DBus")
        .map_err(|_| Error::Protocol)?
        .member("NameOwnerChanged")
        .map_err(|_| Error::Protocol)?
        .arg(0, SERVICE)
        .map_err(|_| Error::Protocol)?
        .build();
    let mut owners = MessageStream::for_match_rule(owner_rule, connection, Some(8))
        .await
        .map_err(|_| Error::Unavailable)?;
    let lookup = || {
        connection.call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetNameOwner",
            &SERVICE,
        )
    };
    let reply = match lookup().await {
        Ok(reply) => reply,
        Err(zbus::Error::MethodError(name, _, error_reply))
            if name.as_str() == "org.freedesktop.DBus.Error.NameHasNoOwner" =>
        {
            checked(&error_reply, "org.freedesktop.DBus")?;
            let started = connection
                .call_method(
                    Some("org.freedesktop.DBus"),
                    "/org/freedesktop/DBus",
                    Some("org.freedesktop.DBus"),
                    "StartServiceByName",
                    &(SERVICE, 0u32),
                )
                .await
                .map_err(|_| Error::Unavailable)?;
            checked(&started, "org.freedesktop.DBus")?;
            let code: u32 = started.body().deserialize().map_err(|_| Error::Protocol)?;
            if !(1..=2).contains(&code) {
                return Err(Error::Protocol);
            }
            lookup().await.map_err(|_| Error::Unavailable)?
        }
        Err(_) => return Err(Error::Unavailable),
    };
    checked(&reply, "org.freedesktop.DBus")?;
    let owner: String = reply.body().deserialize().map_err(|_| Error::Protocol)?;
    zbus::names::UniqueName::try_from(owner.as_str()).map_err(|_| Error::Protocol)?;
    let reply = connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetConnectionUnixUser",
            &owner,
        )
        .await
        .map_err(|_| Error::Unavailable)?;
    checked(&reply, "org.freedesktop.DBus")?;
    let uid: u32 = reply.body().deserialize().map_err(|_| Error::Protocol)?;
    if uid != rustix::process::geteuid().as_raw() {
        return Err(Error::Unavailable);
    }
    let reply = connection
        .call_method(
            Some(owner.as_str()),
            PATH,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(INTERFACE, "version"),
        )
        .await
        .map_err(|_| Error::Unavailable)?;
    checked(&reply, &owner)?;
    let version: OwnedValue = reply.body().deserialize().map_err(|_| Error::Protocol)?;
    if u32::try_from(version).map_err(|_| Error::Protocol)? < 3 {
        return Err(Error::Unsupported);
    }
    let mut random = [0; 16];
    rand::rngs::OsRng
        .try_fill_bytes(&mut random)
        .map_err(|_| Error::Unavailable)?;
    let token = format!(
        "rquickshare_{}",
        random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    let sender = connection
        .unique_name()
        .ok_or(Error::Protocol)?
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    let path = format!("{PATH}/request/{sender}/{token}");
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .sender(owner.as_str())
        .map_err(|_| Error::Protocol)?
        .path(path.as_str())
        .map_err(|_| Error::Protocol)?
        .interface(REQUEST)
        .map_err(|_| Error::Protocol)?
        .member("Response")
        .map_err(|_| Error::Protocol)?
        .build();
    let mut responses = MessageStream::for_match_rule(rule, connection, Some(8))
        .await
        .map_err(|_| Error::Unavailable)?;
    *pending = Some((owner.clone(), path.clone()));
    let options = HashMap::from([("handle_token", Value::from(token.as_str()))]);
    let reply = connection
        .call_method(
            Some(owner.as_str()),
            PATH,
            Some(INTERFACE),
            if contents {
                "OpenFile"
            } else {
                "OpenDirectory"
            },
            &("", Fd::from(file.as_fd()), options),
        )
        .await
        .map_err(|_| Error::Rejected)?;
    checked(&reply, &owner)?;
    let returned: OwnedObjectPath = reply.body().deserialize().map_err(|_| Error::Protocol)?;
    if returned.as_str() != path {
        return Err(Error::Protocol);
    }
    loop {
        tokio::select! { biased;
            changed = owners.next() => {
                let changed = changed.ok_or(Error::Unavailable)?.map_err(|_| Error::Unavailable)?;
                checked(&changed,"org.freedesktop.DBus")?;
                let (name,_,new): (String,String,String) = changed.body().deserialize().map_err(|_| Error::Protocol)?;
                if name == SERVICE && new != owner { return Err(Error::Unavailable); }
            }
            response = responses.next() => {
                let response = response.ok_or(Error::Unavailable)?.map_err(|_| Error::Unavailable)?;
                checked(&response,&owner)?;
                let (code,_): (u32, HashMap<String,OwnedValue>) = response.body().deserialize().map_err(|_| Error::Protocol)?;
                if code > 2 { return Err(Error::Protocol); }
                *pending = None; // A valid Response ends the interaction.
                return match code { 0 => Ok(()), 1 => Err(Error::Cancelled), _ => Err(Error::Rejected) };
            }
        }
    }
}
#[cfg(test)]
mod tests;
