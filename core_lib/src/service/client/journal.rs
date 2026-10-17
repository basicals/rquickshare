//! Optional, bounded history reads; unavailable history never invents live authority.
use std::{collections::BTreeSet, sync::Arc};

use super::*;
use crate::service::journal::{Record, View, COMPLETED, CURRENT, INTERFACE};
impl NativeClient {
    pub async fn read_journal(&mut self, previous: Option<&Arc<View>>) -> Result<Arc<View>> {
        let reply = match self.wire(INTERFACE, "Read", &()).await {
            Ok(reply) => reply,
            Err(ClientError::Incompatible) => {
                return Ok(Arc::new(View {
                    error: 3,
                    ..Default::default()
                }))
            }
            Err(error) => return Err(error),
        };
        if !reply.data().fds().is_empty() {
            return Err(ClientError::Protocol);
        }
        let (epoch, token, error, total): (String, String, u8, u32) = reply
            .body()
            .deserialize()
            .map_err(|_| ClientError::Protocol)?;
        if epoch != format!("{:032x}", self.epoch)
            || error > 3
            || total as usize > COMPLETED + CURRENT
            || (!token.is_empty()
                && (token.len() != 64
                    || !token
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))))
            || (token.is_empty() && (error == 0 || total != 0))
        {
            return Err(ClientError::Protocol);
        }
        if let Some(old) = previous.filter(|old| {
            old.token == token && old.error == error && old.records.len() == total as usize
        }) {
            return Ok(Arc::clone(old));
        }
        let mut records = Vec::new();
        let mut ids = BTreeSet::new();
        while records.len() < total as usize {
            let reply = self
                .wire(
                    INTERFACE,
                    "GetPage",
                    &(token.as_str(), records.len() as u32),
                )
                .await?;
            if !reply.data().fds().is_empty() {
                return Err(ClientError::Protocol);
            }
            let (count, page): (u32, Vec<Record>) = reply
                .body()
                .deserialize()
                .map_err(|_| ClientError::Protocol)?;
            if count != total || page.len() != (total as usize - records.len()).min(16) {
                return Err(ClientError::Protocol);
            }
            for row in page {
                if !row.valid() || !ids.insert(row.id.clone()) {
                    return Err(ClientError::Protocol);
                }
                records.push(row);
            }
        }
        Ok(Arc::new(View {
            token,
            error,
            records,
        }))
    }
    pub async fn clear_journal(&mut self, token: &str) -> Result<()> {
        if token.len() != 64 {
            return Err(ClientError::InvalidInput);
        }
        let reply = self.wire(INTERFACE, "Clear", &token).await?;
        if !reply.data().fds().is_empty() {
            return Err(ClientError::Protocol);
        }
        match reply
            .body()
            .deserialize::<u8>()
            .map_err(|_| ClientError::Protocol)?
        {
            0 => Ok(()),
            2 => Err(ClientError::Rejected(crate::service::Error::StaleControl)),
            1 => Err(ClientError::Settings(
                crate::service::native_settings::StoreError::Write,
            )),
            _ => Err(ClientError::Protocol),
        }
    }
}
