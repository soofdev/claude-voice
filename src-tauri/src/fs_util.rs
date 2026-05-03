use anyhow::{Context, Result};
use std::path::Path;

/// Write `bytes` to `path` atomically: create a sibling temp file, fsync it,
/// then rename it into place. On unix, also set the file's mode (0o600 for
/// secrets, etc.) before the rename so the final file is never readable
/// with looser perms.
pub fn write_atomic(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    write_atomic_inner(path, bytes, mode, None)
}

/// Like [`write_atomic`], but also tightens the parent directory's unix
/// permissions to `dir_mode` (e.g. 0o700). Use only for directories we own
/// — never for shared paths like `~/.claude`.
pub fn write_atomic_with_dir_mode(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    dir_mode: u32,
) -> Result<()> {
    write_atomic_inner(path, bytes, mode, Some(dir_mode))
}

fn write_atomic_inner(
    path: &Path,
    bytes: &[u8],
    mode: u32,
    dir_mode: Option<u32>,
) -> Result<()> {
    let parent = path
        .parent()
        .with_context(|| format!("path has no parent: {}", path.display()))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("create_dir_all {}", parent.display()))?;
    #[cfg(unix)]
    {
        if let Some(m) = dir_mode {
            ensure_dir_mode(parent, m)?;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = dir_mode;
    }

    let file_name = path
        .file_name()
        .with_context(|| format!("path has no file name: {}", path.display()))?;
    let mut tmp_name = std::ffi::OsString::from(".");
    tmp_name.push(file_name);
    tmp_name.push(format!(".tmp.{}", std::process::id()));
    let tmp = parent.join(tmp_name);

    {
        use std::io::Write;
        let mut f = open_for_write(&tmp, mode)?;
        f.write_all(bytes)
            .with_context(|| format!("write {}", tmp.display()))?;
        f.sync_all().ok();
    }

    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(anyhow::Error::from(e)
            .context(format!("rename {} -> {}", tmp.display(), path.display())));
    }
    Ok(())
}

#[cfg(unix)]
fn open_for_write(path: &Path, mode: u32) -> Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(mode)
        .open(path)
        .with_context(|| format!("open {}", path.display()))
}

#[cfg(not(unix))]
fn open_for_write(path: &Path, _mode: u32) -> Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))
}

#[cfg(unix)]
fn ensure_dir_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)
        .with_context(|| format!("stat {}", path.display()))?;
    let current = meta.permissions().mode() & 0o777;
    if current != mode {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
            .with_context(|| format!("chmod {} -> {:o}", path.display(), mode))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_file_with_content() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.json");
        write_atomic(&path, b"hello", 0o600).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn overwrites_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.json");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, b"new", 0o600).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn applies_unix_mode() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.json");
        write_atomic(&path, b"x", 0o600).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn no_temp_file_left_behind() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a.json");
        write_atomic(&path, b"x", 0o600).unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("a.json")]);
    }
}
