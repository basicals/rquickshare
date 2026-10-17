//! Private completed-file receipts. Names describe an already-owned receive root.
use std::{
    fs::{File, Metadata},
    io,
    os::unix::fs::MetadataExt,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{
    receive::Root,
    sessions::details::{TransferDetails, FILE_LIMIT, MANIFEST_BYTES},
};

pub(crate) const RECEIPT_LIMIT: usize = 128;
#[derive(Clone, Default)]
pub(crate) struct ReceiptBudget(Arc<AtomicUsize>);
struct Reservation(ReceiptBudget);
impl Drop for Reservation {
    fn drop(&mut self) {
        self.0 .0.fetch_sub(1, Ordering::AcqRel);
    }
}
impl ReceiptBudget {
    fn reserve(&self) -> Option<Reservation> {
        let mut count = self.0.load(Ordering::Acquire);
        loop {
            if count >= RECEIPT_LIMIT {
                return None;
            }
            match self.0.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => count = current,
            }
        }
        Some(Reservation(self.clone()))
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Fingerprint {
    dev: u64,
    ino: u64,
    size: u64,
    mtime: i64,
    mtime_ns: i64,
    ctime: i64,
    ctime_ns: i64,
}
impl Fingerprint {
    fn new(metadata: Metadata) -> io::Result<Self> {
        if !metadata.is_file() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.len(),
            mtime: metadata.mtime(),
            mtime_ns: metadata.mtime_nsec(),
            ctime: metadata.ctime(),
            ctime_ns: metadata.ctime_nsec(),
        })
    }
}
#[derive(Clone, Eq, PartialEq)]
struct Record {
    relative: String,
    identity: Fingerprint,
}
pub(crate) struct PendingFiles {
    root: Root,
    ids: Vec<i64>,
    records: Vec<Option<Record>>,
    reservation: Reservation,
}
impl std::fmt::Debug for PendingFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingFiles")
            .field("count", &self.ids.len())
            .finish_non_exhaustive()
    }
}
impl PendingFiles {
    pub(crate) fn new(root: Root, ids: Vec<i64>, budget: ReceiptBudget) -> Option<Self> {
        if ids.is_empty() || ids.len() > FILE_LIMIT {
            return None;
        }
        let records = vec![None; ids.len()];
        Some(Self {
            root,
            ids,
            records,
            reservation: budget.reserve()?,
        })
    }
    pub(crate) fn record(
        &mut self,
        id: i64,
        destination: &Path,
        file: &File,
        size: u64,
    ) -> io::Result<()> {
        let index = self
            .ids
            .iter()
            .position(|value| *value == id)
            .ok_or(io::ErrorKind::InvalidInput)?;
        if self.records[index].is_some() {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let relative = destination
            .strip_prefix(self.root.display())
            .ok()
            .and_then(Path::to_str)
            .ok_or(io::ErrorKind::InvalidInput)?;
        if relative.len() > crate::sessions::details::NAME_LIMIT
            || self
                .records
                .iter()
                .flatten()
                .map(|record| record.relative.len())
                .sum::<usize>()
                + relative.len()
                > MANIFEST_BYTES
        {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let relative = relative.to_owned();
        crate::file_source::validate_name(&relative)?;
        let identity = Fingerprint::new(file.metadata()?)?;
        if identity.size != size {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        self.records[index] = Some(Record { relative, identity });
        Ok(())
    }
    pub(crate) fn finish(self) -> Option<Arc<ReceivedFiles>> {
        if self.records.iter().any(Option::is_none) {
            return None;
        }
        self.finish_partial()
    }
    pub(crate) fn finish_partial(self) -> Option<Arc<ReceivedFiles>> {
        let mut records: Vec<Record> = self.records.into_iter().flatten().collect();
        if records.is_empty() {
            return None;
        }
        records.sort_unstable_by(|a, b| {
            (&a.relative, a.identity.size).cmp(&(&b.relative, b.identity.size))
        });
        if records
            .iter()
            .map(|record| record.relative.len())
            .sum::<usize>()
            > MANIFEST_BYTES
        {
            return None;
        }
        Some(Arc::new(ReceivedFiles {
            root: self.root,
            records,
            _reservation: self.reservation,
        }))
    }
}
pub(crate) struct ReceivedFiles {
    root: Root,
    records: Vec<Record>,
    _reservation: Reservation,
}
impl PartialEq for ReceivedFiles {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl std::fmt::Debug for ReceivedFiles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReceivedFiles")
            .field("count", &self.records.len())
            .finish_non_exhaustive()
    }
}
impl ReceivedFiles {
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }
    pub(crate) fn valid_for(&self, details: &TransferDetails) -> bool {
        self.records.len() == details.files.len()
            && self
                .records
                .iter()
                .zip(details.files.iter())
                .all(|(record, detail)| {
                    record.relative == detail.name && record.identity.size == detail.size
                })
    }
    pub(crate) fn subset_of(&self, details: &TransferDetails) -> bool {
        !self.records.is_empty()
            && self.records.iter().all(|record| {
                details
                    .files
                    .iter()
                    .any(|file| file.name == record.relative && file.size == record.identity.size)
            })
    }
    pub(crate) fn details(&self) -> Vec<crate::sessions::details::FileDetail> {
        self.records
            .iter()
            .map(|record| crate::sessions::details::FileDetail {
                name: record.relative.clone(),
                size: record.identity.size,
            })
            .collect()
    }
    pub(crate) fn display_bytes(&self) -> usize {
        self.records
            .iter()
            .map(|record| {
                std::mem::size_of::<crate::sessions::details::FileDetail>() + record.relative.len()
            })
            .sum::<usize>()
            + 64
    }
    pub(crate) fn open(&self, index: u32) -> io::Result<File> {
        let record = self
            .records
            .get(index as usize)
            .ok_or(io::ErrorKind::InvalidInput)?;
        let file = self.root.open_received(&record.relative)?;
        if Fingerprint::new(file.metadata()?)? != record.identity {
            return Err(io::ErrorKind::NotFound.into());
        }
        Ok(file)
    }
    /// The returned mode describes only verified filesystem authority, never a handler.
    pub(crate) fn open_result(&self) -> io::Result<(u8, File)> {
        use rustix::fs::{openat2, Mode, OFlags, ResolveFlags};
        let first = self.records.first().ok_or(io::ErrorKind::InvalidInput)?;
        if self.records.len() == 1 && !first.relative.contains('/') {
            return self.open(0).map(|file| (0, file));
        }
        let common = first
            .relative
            .split_once('/')
            .map(|(top, _)| top)
            .filter(|top| {
                self.records.iter().all(|r| {
                    r.relative
                        .strip_prefix(top)
                        .is_some_and(|rest| rest.starts_with('/'))
                })
            });
        let directory = self.root.open_received(common.unwrap_or("."))?;
        if !directory.metadata()?.is_dir() {
            return Err(io::ErrorKind::NotADirectory.into());
        }
        for record in &self.records {
            let relative = common.map_or(record.relative.as_str(), |top| {
                &record.relative[top.len() + 1..]
            });
            let file = File::from(openat2(
                &directory,
                relative,
                OFlags::RDONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
                Mode::empty(),
                ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
            )?);
            if Fingerprint::new(file.metadata()?)? != record.identity {
                return Err(io::ErrorKind::NotFound.into());
            }
        }
        Ok((if common.is_some() { 1 } else { 2 }, directory))
    }
}
#[cfg(test)]
pub(crate) mod tests;
