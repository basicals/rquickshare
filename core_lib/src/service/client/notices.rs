//! Validated presentation identities tied to the current dedicated client.
use super::state::StateView;
use super::*;
pub use crate::service::notices::NoticeKind as Kind;
use crate::service::{notices::NoticeKind, Phase};
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Notice {
    pub transfer: Transfer,
    pub kind: Kind,
    pub round: u64,
    pub revision: u64,
}
type Row = (String, String, u64, u64);
impl NativeClient {
    pub(super) async fn read_notices(&mut self, capture: &str, view: &mut StateView) -> Result<()> {
        let count: u32 = self
            .wire(bus::NOTIFICATIONS_INTERFACE, "GetNoticeCount", &capture)
            .await?
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if count > 256 || count as usize > view.transfers.len() {
            return Err(ClientError::Protocol);
        }
        let rows: Vec<Row> = self
            .pages_for_on(
                bus::NOTIFICATIONS_INTERFACE,
                "GetNoticePage",
                capture,
                count,
                None,
            )
            .await?;
        let mut previous = None;
        for (id, kind, round, revision) in rows {
            if previous.as_ref().is_some_and(|prior: &String| prior >= &id) {
                return Err(ClientError::Protocol);
            }
            let transfer = view
                .transfers
                .iter()
                .find(|row| row.transfer.id() == id)
                .ok_or(ClientError::Protocol)?;
            let kind = NoticeKind::parse(&kind).ok_or(ClientError::Protocol)?;
            let valid = match kind {
                Kind::Request => {
                    transfer.phase == Phase::AwaitingConsent
                        && transfer.consent_revision == Some(round)
                }
                Kind::Finished => transfer.phase == Phase::Finished && round == 0,
                Kind::Failed => transfer.phase == Phase::Failed && round == 0,
                Kind::Disconnected => transfer.phase == Phase::Disconnected && round == 0,
            };
            if !valid || revision == 0 || revision > view.revision || round > revision {
                return Err(ClientError::Protocol);
            }
            previous = Some(id);
            view.notices.push(Notice {
                transfer: transfer.transfer.clone(),
                kind,
                round,
                revision,
            });
        }
        Ok(())
    }
    pub async fn dismiss_notices(&mut self, notices: &[Notice]) -> Result<()> {
        if notices.is_empty() || notices.len() > 16 {
            return Err(ClientError::InvalidInput);
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut rows = Vec::with_capacity(notices.len());
        for notice in notices {
            self.owned(notice.transfer.id(), &notice.transfer.connection)?;
            if notice.revision == 0
                || (notice.kind == Kind::Request) != (notice.round > 0)
                || notice.round > notice.revision
                || !ids.insert(notice.transfer.id())
            {
                return Err(ClientError::InvalidInput);
            }
            rows.push((
                notice.transfer.id(),
                notice.kind.wire(),
                notice.round,
                notice.revision,
            ));
        }
        self.wire(bus::NOTIFICATIONS_INTERFACE, "Dismiss", &rows)
            .await?
            .body()
            .deserialize::<()>()
            .map_err(|_| ClientError::Protocol)
    }
}
