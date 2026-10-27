//! Explicit bounded private files; creation never replaces or removes an old intent.
use crate::{Error, Result};
use std::{
    io::{Read, Write},
    os::unix::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawFd, FromRawFd},
    },
    path::Path,
};
pub fn read(path: impl AsRef<Path>, limit: usize) -> Result<Vec<u8>> {
    let work = || -> std::io::Result<Vec<u8>> {
        if limit == 0 || limit > 131072 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        let file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)?;
        let m = file.metadata()?;
        if !m.is_file()
            || m.mode() & 0o077 != 0
            || m.uid() != unsafe { libc::getuid() }
            || m.len() == 0
            || m.len() > limit as u64
        {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(bytes)
    };
    work().map_err(|_| Error::input("invalid_private_file"))
}
pub fn create(path: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
    let path = path.as_ref();
    let work = || -> std::io::Result<()> {
        if bytes.is_empty() || bytes.len() > 131072 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(parent)?;
        let m = directory.metadata()?;
        if !m.is_dir() || m.mode() & 0o077 != 0 || m.uid() != unsafe { libc::getuid() } {
            return Err(std::io::ErrorKind::PermissionDenied.into());
        }
        let name = std::ffi::CString::new(
            path.file_name()
                .ok_or(std::io::ErrorKind::InvalidInput)?
                .as_bytes(),
        )
        .map_err(|_| std::io::ErrorKind::InvalidInput)?;
        // SAFETY: directory/name remain live; openat returns an owned new descriptor.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW,
                0o600,
            )
        };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: fd>=0 is owned exclusively by this function and closed by File.
        let mut file = unsafe { std::fs::File::from_raw_fd(fd) };
        file.write_all(bytes)?;
        file.sync_all()?;
        directory.sync_all()?;
        Ok(())
    };
    work().map_err(|_| Error::input("private_persistence_failed"))
}
