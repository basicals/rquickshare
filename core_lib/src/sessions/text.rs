//! Private immutable received content; never part of ordinary display metadata.
use std::fmt;

use super::details::PayloadKind;
pub const MAX_TEXT_BYTES: usize = 128 * 1024;

#[derive(Clone, Eq, PartialEq)]
pub struct ReceivedText {
    kind: PayloadKind,
    text: String,
}
impl fmt::Debug for ReceivedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceivedText")
            .field("kind", &self.kind)
            .field("bytes", &self.text.len())
            .finish_non_exhaustive()
    }
}
impl ReceivedText {
    pub fn new(kind: PayloadKind, text: String) -> Option<Self> {
        if kind == PayloadKind::Files || text.len() > MAX_TEXT_BYTES || text.contains('\0') {
            return None;
        }
        Some(Self { kind, text })
    }
    pub fn kind(&self) -> PayloadKind {
        self.kind
    }
    pub fn text(&self) -> &str {
        &self.text
    }
}
