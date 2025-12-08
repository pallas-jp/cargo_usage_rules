mod aggregator;
mod cli;
mod metadata;
mod scanner;
mod writer;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, SubCommands};

fn main() {
  if let Err(e) = run() {
    eprintln!("Error: {:?}", e);
    std::process::exit(1);
  }
}

fn run() -> Result<()> {
  let cli = Cli::parse();

  match cli.command {
    Commands::UsageRules(args) => {
      println!("Fetching dependencies...");
      metadata::fetch_dependencies().context("Failed to fetch dependencies with 'cargo fetch'")?;

      println!("Reading dependency metadata...");
      let dependencies =
        metadata::get_dependencies().context("Failed to get dependency metadata")?;

      println!("Scanning for usage-rules.md files...");
      let usage_rules =
        scanner::scan_for_usage_rules(&dependencies).context("Failed to scan for usage rules")?;

      if usage_rules.is_empty() {
        println!("No usage-rules.md files found in dependencies.");
      }

      match args.subcommand {
        SubCommands::Sync(sync_args) => {
          println!("Found {} packages with usage rules:", usage_rules.len());
          for rule in &usage_rules {
            println!("  - {} v{}", rule.package_name, rule.package_version);
          }

          println!("\nAggregating content...");
          let package_content =
            aggregator::aggregate_content(usage_rules.clone(), &sync_args.remove)
              .context("Failed to aggregate content")?;

          if package_content.is_empty() && !sync_args.all {
            println!("No packages selected for output. Use --all to include all packages.");
            return Ok(());
          }

          let preamble = aggregator::extract_agents_md_preamble(&sync_args.output)
            .context("Failed to merge with existing content")?;

          println!("Writing output...");
          if sync_args.linked {
            writer::write_linked(
              &sync_args.output,
              &sync_args.link_folder,
              package_content,
              Some(preamble),
            )
            .context("Failed to write linked output")?;

            println!(
              "✓ Successfully wrote usage rules to {} (linked mode: {})",
              sync_args.output.display(),
              sync_args.link_folder.display()
            );
          } else {
            writer::write_inline(&sync_args.output, package_content, Some(preamble))
              .context("Failed to write inline output")?;

            println!(
              "✓ Successfully wrote usage rules to {}",
              sync_args.output.display()
            );
          }
        }

        SubCommands::List => {
          if usage_rules.is_empty() {
            println!("No usage-rules.md files found in dependencies.");
          } else {
            println!("Packages with usage rules:\n");
            for rule in usage_rules {
              let main_file_marker = if rule.main_file.is_some() { "✓" } else { " " };
              let sub_files_count = if !rule.sub_files.is_empty() {
                format!(" ({} sub-files)", rule.sub_files.len())
              } else {
                String::new()
              };

              println!(
                "  [{}] {} v{}{}",
                main_file_marker, rule.package_name, rule.package_version, sub_files_count
              );
            }
          }
        }
      }
    }
  }

  Ok(())
}
