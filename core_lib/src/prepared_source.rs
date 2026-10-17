//! Native immutable source capability. Names never confer file-open/delete authority.
use std::{fmt, sync::Arc};

use crate::{
    file_source::{OpenedFile, OpenedSelection},
    sessions::{details::PayloadKind, text::MAX_TEXT_BYTES},
};

pub const TEXT_RESERVATION_BYTES: u64 = 16 * 1024 * 1024;
#[derive(Clone, Eq, PartialEq)]
pub struct NativeText(String);
impl NativeText {
    pub fn new(text: String) -> Option<Self> {
        (!text.is_empty() && text.len() <= MAX_TEXT_BYTES && !text.contains('\0'))
            .then_some(Self(text))
    }
    pub fn text(&self) -> &str {
        &self.0
    }
    pub fn kind(&self) -> PayloadKind {
        let text = self.0.trim();
        if text
            .get(..7)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
            || text
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        {
            PayloadKind::Url
        } else {
            PayloadKind::Text
        }
    }
}
impl fmt::Debug for NativeText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NativeText")
            .field("bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}
#[derive(Debug)]
pub enum PreparedSource {
    Files(Arc<OpenedSelection>),
    Text(NativeText),
}
impl PreparedSource {
    pub fn files(&self) -> &[OpenedFile] {
        match self {
            Self::Files(files) => files.files(),
            Self::Text(_) => &[],
        }
    }
    pub fn text(&self) -> Option<&NativeText> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }
    pub fn total_bytes(&self) -> u64 {
        match self {
            Self::Files(files) => files.total_bytes(),
            Self::Text(text) => text.text().len() as u64,
        }
    }
    pub fn kind(&self) -> PayloadKind {
        match self {
            Self::Files(_) => PayloadKind::Files,
            Self::Text(text) => text.kind(),
        }
    }
}
impl From<OpenedSelection> for PreparedSource {
    fn from(files: OpenedSelection) -> Self {
        Self::Files(Arc::new(files))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn given_native_text_boundaries_when_constructed_then_utf8_is_preserved_bounded_and_debug_is_private(
    ) {
        // Arrange / Act / Assert
        for (text, valid) in [
            (String::new(), false),
            ("x".into(), true),
            ("é".repeat(65536), true),
            ("x".repeat(MAX_TEXT_BYTES + 1), false),
            ("x\0y".into(), false),
        ] {
            let result = NativeText::new(text.clone());
            assert_eq!(result.is_some(), valid);
            if let Some(body) = result {
                assert_eq!(body.text(), text);
                let source = PreparedSource::Text(body);
                assert_eq!(source.total_bytes(), text.len() as u64);
                assert!(source.files().is_empty());
                if text.len() > 20 {
                    assert!(!format!("{source:?}").contains(&text));
                }
            }
        }
        for (value, kind) in [
            ("https://example.invalid", PayloadKind::Url),
            ("  HTTP://example.invalid  ", PayloadKind::Url),
            ("é", PayloadKind::Text),
            ("+123456789", PayloadKind::Text),
        ] {
            let text = NativeText::new(value.into()).unwrap();
            assert_eq!(text.kind(), kind);
            assert_eq!(text.text(), value);
            assert!(!format!("{text:?}").contains(value));
        }
    }
}
