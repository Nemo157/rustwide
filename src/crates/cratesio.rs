use super::CrateTrait;
use crate::Workspace;
use failure::{Error, ResultExt};
use log::info;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

static CRATES_ROOT: &str = "https://static.crates.io/crates";

impl CratesIOCrate {
    pub(super) fn new(name: &str, version: &str) -> Self {
        CratesIOCrate {
            name: name.into(),
            version: version.into(),
        }
    }

    fn cache_path(&self, workspace: &Workspace) -> PathBuf {
        workspace
            .cache_dir()
            .join("cratesio-sources")
            .join(&self.name)
            .join(format!("{}-{}.crate", self.name, self.version))
    }
}

pub(super) struct CratesIOCrate {
    name: String,
    version: String,
}

impl CrateTrait for CratesIOCrate {
    fn fetch(&self, workspace: &Workspace) -> Result<(), Error> {
        let local = self.cache_path(workspace);
        if local.exists() {
            info!("crate {} {} is already in cache", self.name, self.version);
            return Ok(());
        }

        info!("fetching crate {} {}...", self.name, self.version);
        if let Some(parent) = local.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let remote = format!(
            "{0}/{1}/{1}-{2}.crate",
            CRATES_ROOT, self.name, self.version
        );
        let mut resp = workspace
            .http_client()
            .get(&remote)
            .send()?
            .error_for_status()?;
        resp.copy_to(&mut BufWriter::new(File::create(&local)?))?;

        Ok(())
    }

    fn purge_from_cache(&self, workspace: &Workspace) -> Result<(), Error> {
        let path = self.cache_path(workspace);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn copy_source_to(&self, workspace: &Workspace, dest: &Path) -> Result<(), Error> {
        info!(
            "extracting crate {} {} into {}",
            self.name,
            self.version,
            dest.display()
        );
        super::archive::unpack(&self.cache_path(workspace), dest)
            .context(format!(
                "failed to unpack {} {}",
                self.name, self.version
            ))?;
        Ok(())
    }
}

impl std::fmt::Display for CratesIOCrate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "crates.io crate {} {}", self.name, self.version)
    }
}
