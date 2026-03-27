use crate::app::cli::Cli;
use crate::app::models::RuntimeConfig;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize, Debug, Default)]
struct PresetsFile {
    #[serde(default)]
    global: PresetConfig,
    #[serde(default)]
    presets: HashMap<String, PresetConfig>,
}

#[derive(Deserialize, Debug, Clone, Default)]
struct PresetConfig {
    include: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
    include_in_tree: Option<Vec<String>>,
}

fn load_presets_file() -> Result<PresetsFile> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join(".config")
        .join("config_context")
        .join("presets.toml");

    if !config_path.exists() {
        return Ok(PresetsFile::default());
    }

    let content = fs::read_to_string(&config_path)
        .context(format!("Failed to read config at {:?}", config_path))?;

    let parsed: PresetsFile = toml::from_str(&content).context("Failed to parse presets.toml")?;
    Ok(parsed)
}

fn combine_lists(lists: Vec<Option<Vec<String>>>) -> Vec<String> {
    let mut combined = Vec::new();
    for list in lists.into_iter().flatten() {
        combined.extend(list);
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
    let presets_file = load_presets_file()?;

    let global = presets_file.global;
    let preset = preset_name
        .and_then(|k| presets_file.presets.get(k))
        .cloned()
        .unwrap_or_default();

    // 1. Process Includes (Fallback to "**" if nothing is specified anywhere)
    let mut final_include = combine_lists(vec![global.include, preset.include, include]);
    if final_include.is_empty() {
        final_include = vec!["**".into()];
    }

    // 2. Process Excludes (Always include sensible binary defaults)
    let hardcoded_excludes = vec![
        "**/.git/**".into(),
        "**/*.db".into(),
        "**/*.sqlite".into(),
        "**/*.png".into(),
        "**/*.jpg".into(),
        "**/*.so".into(),
        "**/*.zip".into(),
        "**/*.tar.gz".into(),
    ];
    let final_exclude = combine_lists(vec![
        Some(hardcoded_excludes),
        global.exclude,
        preset.exclude,
        exclude,
    ]);

    // 3. Process Include-in-tree
    let final_include_in_tree = combine_lists(vec![
        global.include_in_tree,
        preset.include_in_tree,
        include_in_tree,
    ]);

    Ok(RuntimeConfig {
        include: final_include,
        exclude: final_exclude,
        include_in_tree: final_include_in_tree,
        tree_only_output: tree_only,
    })
}

pub fn resolve_config(cli: Cli, fallback_preset: Option<&str>) -> Result<RuntimeConfig> {
    let selected_preset = cli.preset.as_deref().or(fallback_preset);

    build_config(
        selected_preset,
        cli.include,
        cli.exclude,
        cli.include_in_tree,
        cli.tree,
    )
}
