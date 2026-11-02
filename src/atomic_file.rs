//! 状态文件的原子落盘与跨进程互斥。
//!
//! 写入走"同目录临时文件 → 整体替换"，进程半路崩掉也不会留下截断的 JSON。
//! 两把锁分工不同：短锁只保护一次写入，长锁占住整个进程生命周期，靠内核句柄
//! 而非时间戳判定归属。

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

/// 崩在半路的进程会把锁文件留在盘上；超过这个时长就当作失效。
const STALE_AFTER: Duration = Duration::from_secs(30);

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// 用 `contents` 原子替换 `path`，临时文件先落盘再换名。
pub(crate) fn write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path
        .parent()
        .ok_or_else(|| io::Error::other("state path has no parent"))?;
    std::fs::create_dir_all(directory)?;

    let staged = directory.join(temporary_name(path));
    let outcome = stage(&staged, contents).and_then(|()| replace(&staged, path));
    if outcome.is_err() {
        let _ = std::fs::remove_file(&staged);
    }
    outcome
}

/// 临时文件名带上进程号与自增序号，同进程并发写也不会互相覆盖。
fn temporary_name(path: &Path) -> String {
    let stem = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("state");
    let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!(".{stem}.{}-{sequence}.tmp", std::process::id())
}

fn stage(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(contents)?;
    file.sync_all()
}

/// 尝试拿一次写锁，拿不到立刻返回 `None`，不在 UI 线程上等。
pub(crate) fn try_lock(path: &Path) -> io::Result<Option<FileLock>> {
    let target = lock_path(path);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match claim(&target) {
        Ok(lock) => Ok(Some(lock)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if !is_stale(&target) {
                return Ok(None);
            }
            // 上一任写者已经不在（或早已结束），回收后重试一次。
            let _ = std::fs::remove_file(&target);
            match claim(&target) {
                Ok(lock) => Ok(Some(lock)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(None),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    }
}

fn claim(path: &Path) -> io::Result<FileLock> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    if let Err(error) = write!(file, "{}", std::process::id()) {
        // 文件一旦建成就代表"已占用"，写 PID 失败必须立刻回收，
        // 否则一次磁盘错误会伪装成 30 秒的跨进程竞争。
        drop(file);
        let _ = std::fs::remove_file(path);
        return Err(error);
    }
    Ok(FileLock {
        path: path.to_owned(),
    })
}

fn is_stale(path: &Path) -> bool {
    std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= STALE_AFTER)
}

/// 锁文件与数据文件同目录同名，只换后缀，避免跨目录的锁语义。
fn lock_path(path: &Path) -> PathBuf {
    path.with_extension("lock")
}

pub(crate) struct FileLock {
    path: PathBuf,
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
pub(crate) fn replace(source: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let from: Vec<u16> = source
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let to: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    let moved = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
pub(crate) fn replace(source: &Path, destination: &Path) -> io::Result<()> {
    std::fs::rename(source, destination)?;
    // rename 只保证目录项切换原子；父目录同步之后，掉电恢复时才不会出现
    // "内容已落盘、文件名还没更新"的空档。
    if let Some(parent) = destination.parent() {
        std::fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{try_lock, write};

    #[test]
    fn writes_replace_and_locks_stay_exclusive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        write(&path, b"first").unwrap();
        write(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");

        let held = try_lock(&path).unwrap().expect("first lock");
        assert!(try_lock(&path).unwrap().is_none());
        drop(held);
        assert!(try_lock(&path).unwrap().is_some());
    }

    #[test]
    fn write_creates_missing_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deeper").join("state.json");
        write(&path, b"{}").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{}");
        // 临时文件不留下残骸。
        let leftovers = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);
    }
}
