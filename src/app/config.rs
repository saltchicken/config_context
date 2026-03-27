use crate::app::cli::Cli;
use crate::app::models::RuntimeConfig;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize, Debug)]
struct PresetsFile {
    #[serde(flatten)]
    presets: HashMap<String, PresetConfig>,
}

#[derive(Deserialize, Debug, Clone, Default)]
struct PresetConfig {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    include_in_tree: Option<Vec<String>>,
}

fn load_presets_file() -> Result<HashMap<String, PresetConfig>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join(".config")
        .join("config_context")
        .join("presets.toml");

    if !config_path.exists() {
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&config_path)
        .context(format!("Failed to read config at {:?}", config_path))?;

    let parsed: PresetsFile = toml::from_str(&content).context("Failed to parse presets.toml")?;
    Ok(parsed.presets)
}

fn merge_vecs(preset_vec: Option<Vec<String>>, cli_vec: Option<Vec<String>>) -> Vec<String> {
    let mut combined = preset_vec.unwrap_or_default();
    if let Some(mut cli_items) = cli_vec {
        combined.append(&mut cli_items);
    }
    // Deduplicate while keeping order
    let mut seen = std::collections::HashSet::new();
    combined.retain(|item| seen.insert(item.clone()));
    combined
}

pub fn build_config(
    preset_name: Option<&str>,
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    include_in_tree: Option<Vec<String>>,
    tree_only: bool,
) -> Result<RuntimeConfig> {
    let presets = load_presets_file()?;

    let preset = preset_name
        .and_then(|k| presets.get(k))
        .cloned()
        .unwrap_or_else(|| PresetConfig {
            // Default: Grab all files in all folders
            include: Some(vec!["**".into()]),
            // Sensible defaults to avoid polluting LLM context with large binary blobs
            exclude: Some(vec![
                "**/.git/**".into(),
                "**/*.db".into(),
                "**/*.sqlite".into(),
                "**/*.png".into(),
                "**/*.jpg".into(),
                "**/*.so".into(),
                "**/*.zip".into(),
                "**/*.tar.gz".into(),
            ]),
            include_in_tree: None,
        });

    let config = RuntimeConfig {
        include: merge_vecs(preset.include, include),
        exclude: merge_vecs(preset.exclude, exclude),
        include_in_tree: merge_vecs(preset.include_in_tree, include_in_tree),
        tree_only_output: tree_only,
    };

    Ok(config)
}

pub fn resolve_config(cli: Cli, fallback_preset: Option<&str>) -> Result<RuntimeConfig> {
    // Priority: CLI Flag > Fallback (Folder Name) > None
    let selected_preset = cli.preset.as_deref().or(fallback_preset);

    build_config(
        selected_preset,
        cli.include,
        cli.exclude,
        cli.include_in_tree,
        cli.tree,
    )
}
