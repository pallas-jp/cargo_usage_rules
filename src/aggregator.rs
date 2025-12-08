use crate::scanner::{read_file_content, UsageRuleSubFile, UsageRules};
use anyhow::Result;
use std::{
  fs,
  path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct PackageContent {
  pub main_file: Option<PathBuf>,
  pub sub_files: Vec<UsageRuleSubFile>, // (relative_path, source_path)
}

#[derive(Clone)]
pub struct PackageContentInfo {
  pub name: String,
  pub content: PackageContent,
}

impl PackageContentInfo {
  pub fn get_aggregated_content(&self) -> Result<String> {
    let mut parts = Vec::new();

    if let Some(path) = &self.content.main_file {
      let content = read_file_content(path)?;
      parts.push(content);
    }

    for UsageRuleSubFile {
      relative_path_name,
      full_path,
    } in &self.content.sub_files
    {
      let content = read_file_content(full_path)?;
      parts.push(format!("\n## {}\n\n{}", relative_path_name, content));
    }

    Ok(parts.join("\n\n"))
  }
}

/// Aggregates usage rules content from multiple packages, excluding any
/// packages specified in the `remove_packages` list.
pub fn aggregate_content(
  usage_rules: Vec<UsageRules>,
  remove_packages: &[String],
) -> Result<Vec<PackageContentInfo>> {
  let mut results = Vec::new();

  for rule in usage_rules {
    if remove_packages.contains(&rule.package_name) {
      continue;
    }

    let package_content = PackageContent {
      main_file: rule.main_file.clone(),
      sub_files: rule.sub_files.clone(),
    };

    results.push(PackageContentInfo {
      name: rule.package_name.clone(),
      content: package_content,
    });
  }

  Ok(results)
}

/// Extracts the preamble from an existing output file if it exists.
///
/// This function reads an existing output file and removes the entire
/// cargo-usage-rules section (between `<!-- cargo-usage-rules-start -->` and
/// `<!-- cargo-usage-rules-end -->` markers), preserving everything else as
/// the preamble. This allows users to add custom content that will be preserved
/// across regenerations.
///
/// # Arguments
///
/// * `output_path` - Path to the existing output file
///
/// # Returns
///
/// The preamble text from the existing file with the cargo-usage-rules section
/// removed, or an empty string if the file doesn't exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be read.
pub fn extract_agents_md_preamble(output_path: &Path) -> Result<String> {
  let existing_content = if output_path.exists() {
    fs::read_to_string(output_path).ok()
  } else {
    None
  };

  let mut preamble = String::new();

  if let Some(existing) = existing_content {
    // Find the cargo-usage-rules section markers
    match (
      existing.find("<!-- cargo-usage-rules-start -->"),
      existing.find("<!-- cargo-usage-rules-end -->"),
    ) {
      (Some(start_pos), Some(end_pos)) => {
        // Both markers found - remove everything between them (inclusive)
        let before = &existing[..start_pos];
        let after_end_marker = end_pos + "<!-- cargo-usage-rules-end -->".len();
        let after = &existing[after_end_marker..];
        preamble = format!("{}{}", before.trim(), after.trim());
        preamble = preamble.trim().to_string();
      }
      _ => {
        // No or malformed markers found - keep entire content as preamble
        preamble = existing.trim().to_string();
      }
    }
  }

  Ok(preamble)
}

/// Formats a package's content into a marked section with MD headers, either
/// inline or to linked folders.
///
/// # Arguments
///
/// * `package` - The package content to format
/// * `link_folder_name` - Optional folder name for linked mode (e.g.,
///   "usage_rules"). If None, content is inlined.
pub fn format_package_section(
  package: &PackageContentInfo,
  link_folder_name: Option<&str>,
) -> Result<String> {
  let content = if let Some(folder) = link_folder_name {
    // Generate relative path to the linked file
    let relative_path = format!("./{}/{}/{}.md", folder, package.name, package.name);
    format!("[{} usage rules]({})", package.name, relative_path)
  } else {
    package.get_aggregated_content()?
  };
  Ok(format!("## {} usage\n{}", package.name, content))
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  fn create_test_usage_rules(name: &str, version: &str, main_content: Option<&str>) -> UsageRules {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();

    let main_file = if let Some(content) = main_content {
      let file = pkg_path.join("usage-rules.md");
      fs::write(&file, content).unwrap();
      Some(file)
    } else {
      None
    };

    UsageRules {
      package_name: name.to_string(),
      package_version: version.to_string(),
      main_file,
      sub_files: vec![],
    }
  }

  #[test]
  fn test_aggregate_content_excludes_removed_packages() {
    let rules = vec![
      create_test_usage_rules("pkg1", "1.0.0", Some("Content 1")),
      create_test_usage_rules("pkg2", "2.0.0", Some("Content 2")),
      create_test_usage_rules("pkg3", "3.0.0", Some("Content 3")),
    ];

    let remove = vec!["pkg2".to_string()];
    let result = aggregate_content(rules, &remove).unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].name, "pkg1");
    assert_eq!(result[1].name, "pkg3");
  }

  #[test]
  fn test_aggregate_content_with_empty_remove_list() {
    let rules = vec![
      create_test_usage_rules("pkg1", "1.0.0", Some("Content 1")),
      create_test_usage_rules("pkg2", "2.0.0", Some("Content 2")),
    ];

    let result = aggregate_content(rules, &[]).unwrap();

    assert_eq!(result.len(), 2);
  }

  #[test]
  fn test_aggregate_content_handles_empty_input() {
    let result = aggregate_content(vec![], &[]).unwrap();
    assert_eq!(result.len(), 0);
  }

  #[test]
  fn test_get_aggregated_content_main_only() {
    let temp = TempDir::new().unwrap();
    let main_file = temp.path().join("usage-rules.md");
    fs::write(&main_file, "Main content").unwrap();

    let package = PackageContentInfo {
      name: "test".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    };

    let content = package.get_aggregated_content().unwrap();
    assert_eq!(content, "Main content");
  }

  #[test]
  fn test_get_aggregated_content_with_sub_files() {
    let temp = TempDir::new().unwrap();
    let main_file = temp.path().join("usage-rules.md");
    fs::write(&main_file, "Main content").unwrap();

    let sub_file = temp.path().join("async.md");
    fs::write(&sub_file, "Async content").unwrap();

    let package = PackageContentInfo {
      name: "test".to_string(),
      content: PackageContent {
        main_file: Some(main_file.clone()),
        sub_files: vec![UsageRuleSubFile {
          relative_path_name: "async".to_string(),
          full_path: sub_file,
        }],
      },
    };

    let content = package.get_aggregated_content().unwrap();
    assert!(content.contains("Main content"));
    assert!(content.contains("## async"));
    assert!(content.contains("Async content"));
  }

  #[test]
  fn test_extract_preamble_with_markers() {
    let temp = TempDir::new().unwrap();
    let output_file = temp.path().join("Agents.md");

    let existing_content = "# Custom Header\n\nMy preamble\n\n<!-- cargo-usage-rules-start \
                            -->\nOld generated content\n<!-- cargo-usage-rules-end -->\n\nFooter \
                            content";

    fs::write(&output_file, existing_content).unwrap();

    let preamble = extract_agents_md_preamble(&output_file).unwrap();

    assert!(preamble.contains("Custom Header"));
    assert!(preamble.contains("My preamble"));
    assert!(preamble.contains("Footer content"));
    assert!(!preamble.contains("cargo-usage-rules"));
    assert!(!preamble.contains("Old generated content"));
  }

  #[test]
  fn test_extract_preamble_without_markers() {
    let temp = TempDir::new().unwrap();
    let output_file = temp.path().join("Agents.md");

    let existing_content = "# No markers here\n\nJust regular content";
    fs::write(&output_file, existing_content).unwrap();

    let preamble = extract_agents_md_preamble(&output_file).unwrap();

    // Should keep entire content as preamble when no markers found
    assert_eq!(preamble, "# No markers here\n\nJust regular content");
  }

  #[test]
  fn test_extract_preamble_with_only_start_marker() {
    let temp = TempDir::new().unwrap();
    let output_file = temp.path().join("Agents.md");

    let existing_content = "Preamble\n\n<!-- cargo-usage-rules-start -->\nContent";
    fs::write(&output_file, existing_content).unwrap();

    let preamble = extract_agents_md_preamble(&output_file).unwrap();

    // Malformed markers - should keep entire content
    assert!(preamble.contains("Preamble"));
    assert!(preamble.contains("Content"));
  }

  #[test]
  fn test_extract_preamble_non_existent_file() {
    let non_existent = PathBuf::from("/tmp/nonexistent-file.md");
    let preamble = extract_agents_md_preamble(&non_existent).unwrap();
    assert_eq!(preamble, "");
  }

  #[test]
  fn test_format_package_section_inline() {
    let temp = TempDir::new().unwrap();
    let main_file = temp.path().join("usage-rules.md");
    fs::write(&main_file, "Test content").unwrap();

    let package = PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    };

    let formatted = format_package_section(&package, None).unwrap();

    assert!(formatted.contains("## test-pkg usage"));
    assert!(formatted.contains("Test content"));
  }

  #[test]
  fn test_format_package_section_linked() {
    let temp = TempDir::new().unwrap();
    let main_file = temp.path().join("usage-rules.md");
    fs::write(&main_file, "Test content").unwrap();

    let package = PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    };

    let formatted = format_package_section(&package, Some("usage_rules")).unwrap();

    assert!(formatted.contains("## test-pkg usage"));
    assert!(formatted.contains("[test-pkg usage rules]"));
    assert!(formatted.contains("./usage_rules/test-pkg/test-pkg.md"));
    assert!(!formatted.contains("Test content")); // Content not included in
                                                  // linked mode
  }
}
