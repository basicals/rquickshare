//! Flat privacy-free DBusMenu projection. Events route to the existing GTK owner.
use super::*;
use std::collections::HashMap;
use tokio::sync::mpsc;
use zbus::zvariant::{OwnedValue, Str, Value};

pub(super) const PATH: &str = "/Menu";
#[cfg(test)]
pub(super) const INTERFACE: &str = "com.canonical.dbusmenu";
const MAX_PROPERTIES: usize = 16;
const ROWS: [i32; 15] = [1, 12, 2, 3, 4, 5, 13, 6, 7, 14, 8, 9, 10, 15, 11];
pub type Layout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);
type Properties = Vec<(i32, HashMap<String, OwnedValue>)>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Action {
    Open = 1,
    Files,
    Folder,
    Incoming,
    Transfers,
    Compact,
    Full,
    Settings,
    Reconnect,
    Hide,
    Quit,
}
impl Action {
    pub fn from_id(id: i32) -> Option<Self> {
        Some(match id {
            1 => Self::Open,
            2 => Self::Files,
            3 => Self::Folder,
            4 => Self::Incoming,
            5 => Self::Transfers,
            6 => Self::Compact,
            7 => Self::Full,
            8 => Self::Settings,
            9 => Self::Reconnect,
            10 => Self::Hide,
            11 => Self::Quit,
            _ => return None,
        })
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open card",
            Self::Files => "Send files…",
            Self::Folder => "Send a folder…",
            Self::Incoming => "Incoming requests",
            Self::Transfers => "Transfers",
            Self::Compact => "Compact view",
            Self::Full => "Full view",
            Self::Settings => "Settings…",
            Self::Reconnect => "Reconnect service",
            Self::Hide => "Hide card",
            Self::Quit => "Quit panel",
        }
    }
    pub fn name(self) -> Option<&'static str> {
        match self {
            Self::Open | Self::Compact | Self::Full => None,
            Self::Files => Some("files"),
            Self::Folder => Some("folder"),
            Self::Incoming => Some("incoming"),
            Self::Transfers => Some("transfers"),
            Self::Settings => Some("settings"),
            Self::Reconnect => Some("reconnect"),
            Self::Hide => Some("hide"),
            Self::Quit => Some("quit"),
        }
    }
    pub fn bit(self) -> u16 {
        1 << (self as u8 - 1)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Projection {
    pub enabled: u16,
    pub full: bool,
}
#[derive(Clone, Copy)]
pub(super) struct State {
    pub revision: u32,
    pub projection: Projection,
    pub exhausted: bool,
}
impl Default for State {
    fn default() -> Self {
        Self {
            revision: 1,
            projection: Projection {
                enabled: 0,
                full: false,
            },
            exhausted: false,
        }
    }
}
#[derive(Clone)]
pub struct Handle {
    pub(super) state: watch::Sender<State>,
}
impl Handle {
    pub fn publish(&self, projection: Projection) {
        self.state.send_if_modified(|state| {
            if state.exhausted || state.projection == projection {
                return false;
            }
            match state.revision.checked_add(1) {
                Some(revision) => {
                    state.revision = revision;
                    state.projection = projection;
                }
                None => state.exhausted = true,
            }
            true
        });
    }
}
pub(super) struct Menu {
    pub state: watch::Sender<State>,
    pub actions: mpsc::Sender<Vec<Action>>,
}

fn invalid() -> fdo::Error {
    fdo::Error::InvalidArgs("Invalid menu request".into())
}
fn known(id: i32) -> bool {
    (0..=15).contains(&id)
}
fn validate_properties(names: &[String]) -> fdo::Result<()> {
    if names.len() > MAX_PROPERTIES
        || names
            .iter()
            .any(|name| name.is_empty() || name.len() > 64 || name.contains('\0'))
    {
        return Err(invalid());
    }
    Ok(())
}
fn properties(id: i32, state: State, names: &[String]) -> HashMap<String, OwnedValue> {
    let mut properties = HashMap::new();
    let mut put = |name: &str, value| {
        if names.is_empty() || names.iter().any(|selected| selected == name) {
            properties.insert(name.to_owned(), value);
        }
    };
    if id == 0 {
        put("children-display", OwnedValue::from(Str::from("submenu")));
    } else if let Some(action) = Action::from_id(id) {
        put("label", OwnedValue::from(Str::from(action.label())));
        put(
            "enabled",
            OwnedValue::from(state.projection.enabled & action.bit() != 0),
        );
        put("visible", OwnedValue::from(true));
        if matches!(action, Action::Compact | Action::Full) {
            put("toggle-type", OwnedValue::from(Str::from("radio")));
            let selected = (action == Action::Full) == state.projection.full;
            put("toggle-state", OwnedValue::from(i32::from(selected)));
        }
    } else {
        put("type", OwnedValue::from(Str::from("separator")));
    }
    properties
}
fn layout(id: i32, depth: i32, names: &[String], state: State) -> fdo::Result<Layout> {
    if !known(id) || depth < -1 {
        return Err(invalid());
    }
    validate_properties(names)?;
    let children = if id == 0 && depth != 0 {
        ROWS.iter()
            .map(|id| {
                OwnedValue::try_from(Value::from((
                    *id,
                    properties(*id, state, names),
                    Vec::<OwnedValue>::new(),
                )))
                .map_err(|_| invalid())
            })
            .collect::<fdo::Result<_>>()?
    } else {
        Vec::new()
    };
    Ok((id, properties(id, state, names), children))
}
fn checked_event(id: i32, event: &str, state: State) -> fdo::Result<Option<Action>> {
    if !known(id) {
        return Err(invalid());
    }
    match event {
        "clicked" => {
            let action = Action::from_id(id).ok_or_else(invalid)?;
            if state.projection.enabled & action.bit() == 0 || state.exhausted {
                return Err(fdo::Error::Failed("Menu action unavailable".into()));
            }
            Ok(Some(action))
        }
        "opened" | "closed" | "hovered" => Ok(None),
        _ => Err(invalid()),
    }
}
impl Menu {
    fn enqueue(&self, actions: Vec<Action>) -> fdo::Result<()> {
        if actions.is_empty() {
            return Ok(());
        }
        self.actions
            .try_send(actions)
            .map_err(|_| fdo::Error::LimitsExceeded("Menu action queue unavailable".into()))
    }
}

#[zbus::interface(name = "com.canonical.dbusmenu", spawn = false)]
impl Menu {
    async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<(u32, Layout)> {
        authenticate(connection, &header).await?;
        let state = *self.state.borrow();
        Ok((
            state.revision,
            layout(parent_id, recursion_depth, &property_names, state)?,
        ))
    }
    async fn get_group_properties(
        &self,
        ids: Vec<i32>,
        property_names: Vec<String>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<Properties> {
        authenticate(connection, &header).await?;
        if ids.len() > 32 || ids.iter().any(|id| !known(*id)) {
            return Err(invalid());
        }
        validate_properties(&property_names)?;
        let ids = if ids.is_empty() {
            (0..=15).collect()
        } else {
            ids
        };
        let state = *self.state.borrow();
        Ok(ids
            .into_iter()
            .map(|id| (id, properties(id, state, &property_names)))
            .collect())
    }
    async fn get_property(
        &self,
        id: i32,
        name: String,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<OwnedValue> {
        authenticate(connection, &header).await?;
        if !known(id) {
            return Err(invalid());
        }
        validate_properties(std::slice::from_ref(&name))?;
        properties(id, *self.state.borrow(), std::slice::from_ref(&name))
            .remove(&name)
            .ok_or_else(invalid)
    }
    async fn event(
        &self,
        id: i32,
        event_id: String,
        _data: OwnedValue,
        _timestamp: u32,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<()> {
        authenticate(connection, &header).await?;
        self.enqueue(
            checked_event(id, &event_id, *self.state.borrow())?
                .into_iter()
                .collect(),
        )
    }
    async fn event_group(
        &self,
        events: Vec<(i32, String, OwnedValue, u32)>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<Vec<i32>> {
        authenticate(connection, &header).await?;
        if events.is_empty() || events.len() > 16 {
            return Err(invalid());
        }
        let state = *self.state.borrow();
        let mut invalid_ids = Vec::new();
        let mut actions = Vec::new();
        for (id, event, _, _) in events {
            match checked_event(id, &event, state) {
                Ok(Some(action)) => actions.push(action),
                Ok(None) => (),
                Err(_) => invalid_ids.push(id),
            }
        }
        if invalid_ids.is_empty() {
            self.enqueue(actions)?;
        }
        Ok(invalid_ids)
    }
    async fn about_to_show(
        &self,
        id: i32,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<bool> {
        authenticate(connection, &header).await?;
        if !known(id) {
            return Err(invalid());
        }
        Ok(false)
    }
    async fn about_to_show_group(
        &self,
        ids: Vec<i32>,
        #[zbus(connection)] connection: &Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> fdo::Result<(Vec<i32>, Vec<i32>)> {
        authenticate(connection, &header).await?;
        if ids.len() > 32 {
            return Err(invalid());
        }
        Ok((
            Vec::new(),
            ids.into_iter().filter(|id| !known(*id)).collect(),
        ))
    }
    #[zbus(property)]
    fn version(&self) -> u32 {
        2
    }
    #[zbus(property)]
    fn status(&self) -> &str {
        "normal"
    }
    #[zbus(property)]
    fn text_direction(&self) -> &str {
        "ltr"
    }
    #[zbus(property)]
    fn icon_theme_path(&self) -> Vec<String> {
        Vec::new()
    }
    #[zbus(signal)]
    pub(super) async fn layout_updated(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests;
