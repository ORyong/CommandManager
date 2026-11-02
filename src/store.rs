use crate::{atomic_file, model::Library};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    atomic_file::write(path, &serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

pub struct Store {
    pub path: PathBuf,
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn read(&self) -> Result<Library> {
        if !self.path.exists() {
            return Ok(Library::default());
        }
        ensure!(
            std::fs::metadata(&self.path)?.len() <= 64 * 1024 * 1024,
            "命令库超过 64 MB"
        );
        Library::decode(&std::fs::read(&self.path)?)
    }
    pub fn mutate(&self, operation: impl FnOnce(&mut Library) -> Result<()>) -> Result<Library> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let _lock = atomic_file::try_lock(&self.path)?
            .ok_or_else(|| anyhow::anyhow!("命令库正被其他进程写入，请重试"))?;
        let mut library = self.read()?;
        operation(&mut library)?;
        library.validate()?;
        atomic_file::write(&self.path, &serde_json::to_vec_pretty(&library)?)?;
        Ok(library)
    }
    pub fn export(&self, path: &Path) -> Result<()> {
        ensure!(path != self.path, "请选择其他导出位置");
        atomic_file::write(path, &serde_json::to_vec_pretty(&self.read()?)?)?;
        Ok(())
    }
}
