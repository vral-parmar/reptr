use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::model::{
    Client, EngagementMeta, LibraryConfig, OutputConfig, SeverityThresholds, TemplateConfig,
};

#[derive(Debug, Deserialize)]
pub struct EngagementConfig {
    pub engagement: EngagementMeta,
    pub client: Option<ClientRef>,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub template: TemplateConfig,
    #[serde(default)]
    pub severity_thresholds: SeverityThresholds,
    #[serde(default)]
    pub library: LibraryConfig,
}

#[derive(Debug, Deserialize)]
pub struct ClientRef {
    #[serde(default)]
    pub file: Option<String>,
    /// Inline client fields are also supported.
    #[serde(flatten)]
    pub inline: toml::Value,
}

pub fn load_engagement_config(root: &Path) -> Result<(EngagementConfig, Client)> {
    let reptr_toml = root.join("reptr.toml");
    let raw = fs::read_to_string(&reptr_toml)
        .with_context(|| format!("reading {}", reptr_toml.display()))?;
    let cfg: EngagementConfig =
        toml::from_str(&raw).with_context(|| format!("parsing {}", reptr_toml.display()))?;

    let client = load_client(root, cfg.client.as_ref()).with_context(|| "loading client config")?;

    Ok((cfg, client))
}

fn load_client(root: &Path, client_ref: Option<&ClientRef>) -> Result<Client> {
    let Some(client_ref) = client_ref else {
        return Ok(Client::default());
    };

    if let Some(file) = &client_ref.file {
        let path: PathBuf = root.join(file);
        let raw =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let parsed: Client =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        return Ok(parsed);
    }

    // No `file = ...`: try to deserialize the inline fields directly.
    let inline_str = toml::to_string(&client_ref.inline).unwrap_or_default();
    let parsed: Client = toml::from_str(&inline_str).unwrap_or_default();
    Ok(parsed)
}
