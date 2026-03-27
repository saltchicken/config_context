pub mod cli;
pub mod config;
pub mod formatter;
pub mod models;
pub mod scanner;

use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

use self::cli::Cli;
use self::config::resolve_config;
use self::formatter::OutputGenerator;
use self::models::RuntimeConfig;
use self::scanner::Scanner;

pub fn generate(config: RuntimeConfig, root: PathBuf) -> Result<String> {
    // Scan Directory
    let scanner = Scanner::new(root, &config)?;
    let entries = scanner.scan();

    if entries.is_empty() {
        return Ok(String::new());
    }

    // Generate Output
    let tree_str = OutputGenerator::generate_tree(&entries);

    let final_output = if config.tree_only_output {
        format!(
            "<directory_structure>\n{}\n</directory_structure>",
            tree_str
        )
    } else {
        let content_str = OutputGenerator::generate_content(&entries);
        OutputGenerator::format_full_output(&tree_str, &content_str)
    };

    Ok(final_output)
}

pub fn run() -> Result<()> {
    // 1. Parse Args
    let args = Cli::parse();

    // 2. Identify target root (.config or .config/FOLDER)
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_dir = home.join(".config");
    
    let target_dir = if let Some(folder) = args.folder.as_deref() {
        config_dir.join(folder)
    } else {
        config_dir
    };

    if !target_dir.exists() {
        anyhow::bail!("Target directory does not exist: {:?}", target_dir);
    }

    let project_name = target_dir.file_name().and_then(|n| n.to_str());

    // 3. Resolve Configuration
    let config = resolve_config(args, project_name)?;

    if config.include.is_empty() && config.include_in_tree.is_empty() {
        anyhow::bail!("No include patterns provided.");
    }

    // 4. Generate context
    let output = generate(config, target_dir)?;

    if output.is_empty() {
        log::warn!("⚠️ No content found for the specified criteria.");
        return Ok(());
    }

    // 5. Print to Stdout
    println!("{}", output);

    Ok(())
}
