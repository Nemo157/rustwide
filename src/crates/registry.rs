use super::CrateTrait;
use crate::cmd::{Command, ProcessLinesActions};
use crate::Workspace;
use failure::{Error, ResultExt};
use log::info;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use url::Url;

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
struct IndexConfig {
    dl: String,
    #[serde(default)]
    api: Option<Url>,
    #[serde(default)]
    allowed_registries: Vec<String>,
}

impl RegistryCrate {
    pub(super) fn new(name: &str, version: &str, index: &str) -> Self {
        RegistryCrate {
            name: name.into(),
            version: version.into(),
            index: index.into(),
        }
    }

    fn crate_cache_path(&self, workspace: &Workspace) -> PathBuf {
        workspace
            .cache_dir()
            .join("registry-sources")
            .join(slugify(&self.index))
            .join(&self.name)
            .join(format!("{}-{}.crate", self.name, self.version))
    }

    fn index_cache_path(&self, workspace: &Workspace) -> PathBuf {
        workspace
            .cache_dir()
            .join("registry-index")
            .join(slugify(&self.index))
    }

    fn update_index(&self, workspace: &Workspace) -> Result<PathBuf, Error> {
        let path = self.index_cache_path(workspace);

        if path.join("HEAD").is_file() {
            info!("updating cached index repository {}", self.index);
            Command::new(workspace, "git")
                // .args(&self.suppress_password_prompt_args(workspace))
                .args(&["-c", "remote.origin.fetch=refs/heads/*:refs/heads/*"])
                .args(&["fetch", "origin", "--force", "--prune"])
                .cd(&path)
                // .process_lines(&mut detect_private_repositories)
                .run()
                .with_context(|_| format!("failed to update {}", self.index))?;
        } else {
            info!("cloning index repository {}", self.index);
            Command::new(workspace, "git")
                // .args(&self.suppress_password_prompt_args(workspace))
                .args(&["clone", "--bare", "--no-tags", "--single-branch", &self.index])
                .args(&[&path])
                // .process_lines(&mut detect_private_repositories)
                .run()
                .with_context(|_| format!("failed to clone {}", self.index))?;
        }

        Ok(path)
    }

    /// Inspects the given repository to find the config as specified in [RFC 2141][], from the
    /// current HEAD ref.
    ///
    /// [RFC 2141]: https://rust-lang.github.io/rfcs/2141-alternative-registries.html
    fn index_config(&self, workspace: &Workspace) -> Result<IndexConfig, Error> {
        let path = self.update_index(workspace)?;
        let content = Command::new(workspace, "git")
            .args(&["show", "HEAD:config.json"])
            .cd(&path)
            .run_capture()
            .with_context(|_| format!("failed to get config file for {}", self.index))?
            .stdout_lines()
            .join("\n");
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }

    fn prefix(&self) -> String {
        match self.name.chars().count() {
            1 => "1".into(),
            2 => "2".into(),
            3 => format!("3/{}", self.name.chars().next().unwrap()),
            _ => {
                let chars: Vec<_> = self.name.chars().take(4).collect();
                format!("{}{}/{}{}", chars[0], chars[1], chars[2], chars[3])
            }
        }
    }

    fn dl_url(&self, workspace: &Workspace) -> Result<Url, Error> {
        let template = self.index_config(workspace)?.dl;
        let replacements = [
            ("{crate}", &self.name),
            ("{version}", &self.version),
            ("{prefix}", &self.prefix()),
            ("{lowerprefix}", &self.prefix().to_lowercase()),
        ];
        let url = if replacements.iter().any(|(key, _)| template.contains(key) ) {
            let mut url = template;
            for (key, value) in &replacements {
                url = url.replace(key, value);
            }
            url
        } else {
            format!("{}/{}/{}/download", template, self.name, self.version)
        };

        Ok(Url::parse(&url)?)
    }
}

pub(super) struct RegistryCrate {
    name: String,
    version: String,
    index: String,
}

impl CrateTrait for RegistryCrate {
    fn fetch(&self, workspace: &Workspace) -> Result<(), Error> {
        let local = self.crate_cache_path(workspace);
        if local.exists() {
            info!("crate {} {} ({}) is already in cache", self.name, self.version, self.index);
            return Ok(());
        }

        info!("fetching crate {} {} ({})...", self.name, self.version, self.index);
        if let Some(parent) = local.parent() {
            std::fs::create_dir_all(parent)?;
        }

        
        let mut resp = workspace
            .http_client()
            .get(self.dl_url(&workspace)?.as_str())
            .send()?
            .error_for_status()?;
        resp.copy_to(&mut BufWriter::new(File::create(&local)?))?;

        Ok(())
    }

    fn purge_from_cache(&self, workspace: &Workspace) -> Result<(), Error> {
        let path = self.crate_cache_path(workspace);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn copy_source_to(&self, workspace: &Workspace, dest: &Path) -> Result<(), Error> {
        info!(
            "extracting crate {} {} ({}) into {}",
            self.name,
            self.version,
            self.index,
            dest.display()
        );
        super::archive::unpack(&self.crate_cache_path(workspace), dest)
            .context(format!(
                "failed to unpack {} {} ({})",
                self.name, self.version, self.index
            ))?;
        Ok(())
    }
}

impl std::fmt::Display for RegistryCrate {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "registry crate {} {} ({})", self.name, self.version, self.index)
    }
}

fn slugify(s: &str) -> String {
    s.replace(|c: char| !c.is_alphanumeric(), "-")
}
