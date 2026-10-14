//! Owned Linux input resources for native clients. Names never authorize path opens.

use std::collections::HashSet;
use std::fmt;
use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};

use rustix::fs::{fcntl_getfl, openat2, Dir, Mode, OFlags, ResolveFlags, CWD};
use rustix::io::{fcntl_getfd, fcntl_setfd, FdFlags};

use crate::file_reader::read_file_chunk;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message)
}

fn validate_name(name: &str) -> io::Result<()> {
    if name.is_empty()
        || name.len() > 16 * 1024
        || name
            .split('/')
            .any(|part| part.is_empty() || part.len() > 255 || matches!(part, "." | ".."))
        || name
            .chars()
            .any(|value| matches!(value, '\\' | ':') || value.is_control())
    {
        return Err(invalid("Unsafe relative destination name"));
    }
    Ok(())
}

fn source_size(size: u64) -> io::Result<u64> {
    if size > i64::MAX as u64 {
        return Err(invalid("Source size exceeds protocol representation"));
    }
    Ok(size)
}

fn checked_total(total: u64, size: u64, limit: u64) -> io::Result<u64> {
    total
        .checked_add(size)
        .filter(|sum| *sum <= limit)
        .ok_or_else(|| invalid("Selection byte budget exceeded"))
}

/// Per-selection acquisition budgets. The service must also bound aggregate usage.
#[derive(Clone, Copy, Debug)]
pub struct SelectionLimits {
    pub max_files: usize,
    pub max_entries: usize,
    pub max_depth: usize,
    pub max_bytes: u64,
}

impl Default for SelectionLimits {
    fn default() -> Self {
        Self {
            max_files: 256,
            max_entries: 4096,
            max_depth: 64,
            max_bytes: 1024 * 1024 * 1024 * 1024,
        }
    }
}

impl SelectionLimits {
    fn validate(self) -> io::Result<()> {
        if self.max_files == 0
            || self.max_entries == 0
            || self.max_depth == 0
            || self.max_bytes == 0
        {
            return Err(invalid("Selection budgets must be nonzero"));
        }
        Ok(())
    }
}

/// A validated, read-only regular file. Drop closes the handle; it never unlinks a source.
pub struct OpenedFile {
    file: File,
    relative_name: String,
    size: u64,
    memory_bytes: u64,
}

impl fmt::Debug for OpenedFile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedFile")
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl OpenedFile {
    /// Adopts a descriptor received from a native client, without reopening its name.
    pub fn adopt(file: File, relative_name: String) -> io::Result<Self> {
        validate_name(&relative_name)?;
        fcntl_setfd(&file, fcntl_getfd(&file)? | FdFlags::CLOEXEC)?;
        let flags = fcntl_getfl(&file)?;
        if flags.contains(OFlags::PATH) || flags & OFlags::ACCMODE != OFlags::RDONLY {
            return Err(invalid("Expected a read-only file descriptor"));
        }
        let memory_backed = match rustix::fs::fcntl_get_seals(&file) {
            Ok(seals) if seals.contains(crate::memory_source::IMMUTABLE_SEALS) => true,
            Ok(_) => return Err(invalid("Expected an immutable memory descriptor")),
            Err(rustix::io::Errno::INVAL) => false,
            Err(error) => return Err(error.into()),
        };
        // Read size after checking irrevocable seals; a mutable memory size cannot race adoption.
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(invalid("Expected a regular file descriptor"));
        }
        let size = source_size(metadata.len())?;
        if memory_backed && (size == 0 || size > crate::memory_source::MAX_MEMORY_FILE_BYTES as u64)
        {
            return Err(invalid("Memory source size exceeds bounds"));
        }
        Ok(Self {
            file,
            relative_name,
            size,
            memory_bytes: if memory_backed { size } else { 0 },
        })
    }

    pub fn relative_name(&self) -> &str {
        &self.relative_name
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    /// Kernel-derived immutable memory charge, independent of the display name.
    pub fn memory_bytes(&self) -> u64 {
        self.memory_bytes
    }

    pub fn read_chunk_at(&self, offset: u64, buffer: &mut [u8]) -> io::Result<usize> {
        read_file_chunk(&self.file, self.size, offset, buffer)
    }

    pub(crate) fn clone_handle(&self) -> io::Result<File> {
        self.file.try_clone()
    }
}

/// An acquired selection with validated, unique protocol-relative names.
pub struct OpenedSelection {
    files: Vec<OpenedFile>,
    total_bytes: u64,
}

impl fmt::Debug for OpenedSelection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenedSelection")
            .field("file_count", &self.files.len())
            .field("total_bytes", &self.total_bytes)
            .finish()
    }
}

impl OpenedSelection {
    pub fn from_files(files: Vec<OpenedFile>, limits: SelectionLimits) -> io::Result<Self> {
        let total_bytes = validate_files(files.iter(), limits)?;
        if files.is_empty() {
            return Err(invalid("Selection contains no regular files"));
        }
        Ok(Self { files, total_bytes })
    }

    /// Internal transaction commit: the same validator already checked this manifest.
    pub(crate) fn from_validated_files(files: Vec<OpenedFile>, total_bytes: u64) -> Self {
        Self { files, total_bytes }
    }

    /// Desktop ingress only. A daemon IPC endpoint must accept handles, not invoke this on caller paths.
    pub fn acquire(roots: &[PathBuf], limits: SelectionLimits) -> io::Result<Self> {
        let mut builder = SelectionBuilder::new(limits)?;
        let mut root_names = HashSet::new();
        for path in roots {
            builder.visit()?;
            let root = AcquiredRoot::open(path)?;
            if !root_names.insert(root.name.clone()) {
                return Err(invalid("Duplicate selected root name"));
            }
            root.collect(&mut builder)?;
        }
        builder.finish()
    }

    pub fn files(&self) -> &[OpenedFile] {
        &self.files
    }

    /// Duplicate already-owned read-only capabilities; names never authorize reopening.
    pub fn try_clone(&self) -> io::Result<Self> {
        let files = self
            .files
            .iter()
            .map(|file| OpenedFile::adopt(file.clone_handle()?, file.relative_name().to_owned()))
            .collect::<io::Result<Vec<_>>>()?;
        Self::from_files(files, SelectionLimits::default())
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }
}

/// Validate a candidate manifest without cloning handles or mutating a preparation.
pub(crate) fn validate_files<'a>(
    files: impl Iterator<Item = &'a OpenedFile>,
    limits: SelectionLimits,
) -> io::Result<u64> {
    limits.validate()?;
    let mut names: Vec<&str> = Vec::new();
    let mut total = 0;
    for file in files {
        total = validate_next_file(file, names.iter().copied(), names.len(), total, limits)?;
        names.push(&file.relative_name);
    }
    Ok(total)
}

fn validate_next_file<'a>(
    file: &OpenedFile,
    mut names: impl Iterator<Item = &'a str>,
    count: usize,
    total: u64,
    limits: SelectionLimits,
) -> io::Result<u64> {
    if file.relative_name.split('/').count() - 1 > limits.max_depth {
        return Err(invalid("Selection manifest depth budget exceeded"));
    }
    if count >= limits.max_files {
        return Err(invalid("Selection file budget exceeded"));
    }
    let total = checked_total(total, file.size, limits.max_bytes)?;
    if names.any(|name| {
        name == file.relative_name
            || name
                .strip_prefix(&file.relative_name)
                .is_some_and(|suffix| suffix.starts_with('/'))
            || file
                .relative_name
                .strip_prefix(name)
                .is_some_and(|suffix| suffix.starts_with('/'))
    }) {
        return Err(invalid("Duplicate destination name"));
    }
    Ok(total)
}

struct SelectionBuilder {
    limits: SelectionLimits,
    files: Vec<OpenedFile>,
    entries: usize,
    total_bytes: u64,
}

impl SelectionBuilder {
    fn new(limits: SelectionLimits) -> io::Result<Self> {
        limits.validate()?;
        Ok(Self {
            limits,
            files: Vec::new(),
            entries: 0,
            total_bytes: 0,
        })
    }

    fn visit(&mut self) -> io::Result<()> {
        if self.entries >= self.limits.max_entries {
            return Err(invalid("Selection entry budget exceeded"));
        }
        self.entries += 1;
        Ok(())
    }

    fn push(&mut self, file: OpenedFile) -> io::Result<()> {
        self.total_bytes = validate_next_file(
            &file,
            self.files.iter().map(OpenedFile::relative_name),
            self.files.len(),
            self.total_bytes,
            self.limits,
        )?;
        self.files.push(file);
        Ok(())
    }

    fn finish(self) -> io::Result<OpenedSelection> {
        if self.files.is_empty() {
            return Err(invalid("Selection contains no regular files"));
        }
        Ok(OpenedSelection {
            files: self.files,
            total_bytes: self.total_bytes,
        })
    }
}

struct AcquiredRoot {
    file: File,
    name: String,
}

fn open_child(directory: &File, name: &Path) -> io::Result<File> {
    Ok(openat2(
        directory,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
    )?
    .into())
}

impl AcquiredRoot {
    fn open(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid("Selected root has no valid UTF-8 name"))?
            .to_owned();
        validate_name(&name)?;
        let file = openat2(
            CWD,
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY,
            Mode::empty(),
            ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_MAGICLINKS,
        )?
        .into();
        Ok(Self { file, name })
    }

    fn collect(self, builder: &mut SelectionBuilder) -> io::Result<()> {
        let metadata = self.file.metadata()?;
        if metadata.is_dir() {
            walk(&self.file, &self.name, 1, builder)
        } else {
            builder.push(OpenedFile::adopt(self.file, self.name)?)
        }
    }
}

fn walk(
    directory: &File,
    parent: &str,
    depth: usize,
    builder: &mut SelectionBuilder,
) -> io::Result<()> {
    if depth > builder.limits.max_depth {
        return Err(invalid("Selection depth budget exceeded"));
    }
    let mut names = Vec::new();
    for entry in Dir::read_from(directory)? {
        let entry = entry?;
        let name = entry.file_name().to_bytes();
        if matches!(name, b"." | b"..") {
            continue;
        }
        builder.visit()?;
        let text =
            std::str::from_utf8(name).map_err(|_| invalid("Source name is not valid UTF-8"))?;
        validate_name(text)?;
        names.push(text.to_owned());
    }
    names.sort();
    for name in names {
        let file = open_child(directory, Path::new(&name))?;
        let relative = format!("{parent}/{name}");
        if file.metadata()?.is_dir() {
            walk(&file, &relative, depth + 1, builder)?;
        } else {
            builder.push(OpenedFile::adopt(file, relative)?)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
