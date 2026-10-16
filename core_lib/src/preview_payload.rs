//! Announced auxiliary IDs have no image/control interpretation until a codec is established.
use std::collections::HashSet;
use std::fmt;

use anyhow::{anyhow, Result};

use crate::sharing_nearby::IntroductionFrame;

// Defensive per-request metadata budget, not a negotiated wire capability.
const MAX_PREVIEW_IDS: usize = 256;

#[derive(Default)]
pub(crate) struct PreviewIds(HashSet<i64>);
impl fmt::Debug for PreviewIds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PreviewIds")
            .field("count", &self.0.len())
            .finish()
    }
}
impl PreviewIds {
    pub(crate) fn from_introduction(introduction: &IntroductionFrame) -> Result<Self> {
        if introduction.preview_payload_ids.len() > MAX_PREVIEW_IDS {
            return Err(anyhow!("Too many preview payload announcements"));
        }
        let mut ids = HashSet::new();
        ids.try_reserve(introduction.preview_payload_ids.len())
            .map_err(|_| anyhow!("Cannot reserve preview payload announcements"))?;
        ids.extend(introduction.preview_payload_ids.iter().copied());
        let mut content = introduction
            .file_metadata
            .iter()
            .map(|meta| meta.payload_id())
            .chain(
                introduction
                    .text_metadata
                    .iter()
                    .map(|meta| meta.payload_id()),
            )
            .chain(
                introduction
                    .wifi_credentials_metadata
                    .iter()
                    .map(|meta| meta.payload_id()),
            )
            .chain(
                introduction
                    .app_metadata
                    .iter()
                    .flat_map(|meta| meta.payload_id.iter().copied()),
            )
            .chain(
                introduction
                    .stream_metadata
                    .iter()
                    .map(|meta| meta.payload_id()),
            );
        if content.any(|id| ids.contains(&id)) {
            return Err(anyhow!("Preview payload overlaps attachment content"));
        }
        Ok(Self(ids))
    }
    pub(crate) fn contains(&self, id: i64) -> bool {
        self.0.contains(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharing_nearby::*;
    #[test]
    fn given_preview_announcements_when_admitted_then_limits_duplicates_signed_ids_and_content_separation_are_explicit(
    ) {
        // Arrange / Act / Assert
        for count in [0, 1, 255, 256, 257] {
            let frame = IntroductionFrame {
                preview_payload_ids: (0..count).map(i64::from).collect(),
                ..Default::default()
            };
            assert_eq!(PreviewIds::from_introduction(&frame).is_ok(), count <= 256);
        }
        let frame = IntroductionFrame {
            preview_payload_ids: vec![i64::MIN, 0, i64::MAX, 0, 42424242],
            ..Default::default()
        };
        let ids = PreviewIds::from_introduction(&frame).unwrap();
        assert_eq!(ids.0.len(), 4);
        for id in frame.preview_payload_ids {
            assert!(ids.contains(id));
        }
        assert!(!ids.contains(1));
        assert!(!format!("{ids:?}").contains("42424242"));
        assert!(!PreviewIds::default().contains(0));
        for family in 0..5 {
            for id in [i64::MIN, 0, i64::MAX] {
                let mut frame = IntroductionFrame {
                    preview_payload_ids: vec![id],
                    ..Default::default()
                };
                match family {
                    0 => frame.file_metadata.push(FileMetadata {
                        payload_id: Some(id),
                        ..Default::default()
                    }),
                    1 => frame.text_metadata.push(TextMetadata {
                        payload_id: Some(id),
                        ..Default::default()
                    }),
                    2 => frame
                        .wifi_credentials_metadata
                        .push(WifiCredentialsMetadata {
                            payload_id: Some(id),
                            ..Default::default()
                        }),
                    3 => frame.app_metadata.push(AppMetadata {
                        payload_id: vec![7, id],
                        ..Default::default()
                    }),
                    4 => frame.stream_metadata.push(StreamMetadata {
                        payload_id: Some(id),
                        ..Default::default()
                    }),
                    _ => unreachable!(),
                }
                assert!(PreviewIds::from_introduction(&frame).is_err());
                frame.preview_payload_ids = vec![123];
                assert!(PreviewIds::from_introduction(&frame).is_ok());
            }
        }
        for family in [0, 1, 2, 4] {
            let mut frame = IntroductionFrame {
                preview_payload_ids: vec![0],
                ..Default::default()
            };
            match family {
                0 => frame.file_metadata.push(FileMetadata::default()),
                1 => frame.text_metadata.push(TextMetadata::default()),
                2 => frame
                    .wifi_credentials_metadata
                    .push(WifiCredentialsMetadata::default()),
                _ => frame.stream_metadata.push(StreamMetadata::default()),
            }
            assert!(PreviewIds::from_introduction(&frame).is_err());
        }
    }
}
