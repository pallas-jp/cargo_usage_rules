use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cargo-usage-rules")]
#[command(bin_name = "cargo")]
#[command(
  version,
  about = "Aggregate usage-rules.md files from Rust dependencies"
)]
pub struct Cli {
  #[command(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  /// Aggregate usage rules from dependencies
  #[command(name = "usage-rules")]
  UsageRules(UsageRulesArgs),
}

#[derive(Parser)]
pub struct UsageRulesArgs {
  #[command(subcommand)]
  pub subcommand: SubCommands,
}

#[derive(Subcommand)]
pub enum SubCommands {
  /// Sync usage rules from dependencies into output file
  Sync(SyncArgs),

  /// List all dependencies that have usage-rules.md files
  List,
}

#[derive(Parser)]
pub struct SyncArgs {
  /// Include all dependencies (default if no specific packages given)
  #[arg(long)]
  pub all: bool,

  /// Output file path
  #[arg(long, short = 'o', default_value = "Agents.md")]
  pub output: PathBuf,

  /// Use linked mode (create separate files in folder)
  #[arg(long, action = clap::ArgAction::Set, default_value_t = true, value_parser = clap::value_parser!(bool))]
  pub linked: bool,

  /// Folder path for linked mode files
  #[arg(long, default_value = "usage_rules")]
  pub link_folder: PathBuf,

  /// Comma-separated list of package names to inline (even in folder mode)
  #[arg(long, value_delimiter = ',')]
  pub inline: Vec<String>,

  /// Comma-separated list of package names to exclude
  #[arg(long, value_delimiter = ',')]
  pub remove: Vec<String>,
}
