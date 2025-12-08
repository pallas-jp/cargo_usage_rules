use crate::aggregator::{format_package_section, PackageContentInfo};
use anyhow::{Context, Result};
use std::{fs, path::Path};

/// Generates the standard header for the output file usage-rules section.
pub fn generate_header(use_folder_mode: bool) -> String {
  let mut header = "IMPORTANT: Consult these usage rules early and often when working with the \
                    packages listed below. Before attempting to use any of these packages or to \
                    discover if you should use them, review their usage rules to understand the \
                    correct patterns, conventions, and best practices.\n\nThere are general rules \
                    for rust, cargo, etc also contained directly in this file."
    .to_string();

  if use_folder_mode {
    header.push_str(
      "\n\nEach package's usage rules are contained in separate files within the linked folder. \
       Please refer to the individual files for detailed usage instructions.",
    );
  }

  header.push_str(&format!(
    "## General Rust Usage\n\n{}",
    include_str!("../base.md")
  ));

  header
}

/// Writes package content inline to a single output file.
///
/// All package content is written directly into the main output file, with each
/// package section wrapped in HTML comment markers for identification.
///
/// # Arguments
///
/// * `output_path` - Path where the output file should be written
/// * `packages` - Vector of package content to write
/// * `preamble` - Optional custom preamble to use instead of the default header
///
/// # Returns
///
/// `Ok(())` if the file is written successfully.
///
/// # Errors
///
/// Returns an error if the file cannot be written to the specified path.
pub fn write_inline(
  output_path: &Path,
  packages: Vec<PackageContentInfo>,
  preamble: Option<String>,
) -> Result<()> {
  let content = create_main_agents_file(packages, preamble, None)?;
  fs::write(output_path, content)
    .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

  Ok(())
}

fn create_main_agents_file(
  packages: Vec<PackageContentInfo>,
  preamble: Option<String>,
  link_folder_name: Option<&str>,
) -> Result<String> {
  let header = generate_header(link_folder_name.is_some());

  let mut package_sections = Vec::new();
  for pkg in &packages {
    package_sections.push(format_package_section(pkg, link_folder_name)?);
  }

  // Wrap the generated content with cargo-usage-rules markers
  let generated_section = format!(
    "<!-- cargo-usage-rules-start -->\n\n{}\n{}\n<!-- cargo-usage-rules-end -->\n\n",
    header,
    package_sections.join("\n\n")
  );

  Ok(if let Some(pre) = preamble {
    if pre.is_empty() {
      generated_section
    } else {
      format!("{}\n\n{}", pre, generated_section)
    }
  } else {
    generated_section
  })
}

/// Writes package content in folder mode with separate files and links.
pub fn write_linked(
  output_path: &Path,
  folder_path: &Path,
  packages: Vec<PackageContentInfo>,
  preamble: Option<String>,
) -> Result<()> {
  for pkg in packages.iter() {
    // Create package subdirectory in usage_rules folder
    let pkg_dir = folder_path.join(&pkg.name);
    fs::create_dir_all(&pkg_dir)
      .with_context(|| format!("Failed to create package dir: {}", pkg_dir.display()))?;

    // Copy usage-rules.md main file to the output folder with the package
    // name, and copy it's own usage_rules directory to the output folder with
    // a subdirectory equal to the package name.
    if let Some(main_file_path) = &pkg.content.main_file {
      let dest_main_file = pkg_dir.join(format!("{}.md", pkg.name));
      fs::copy(main_file_path, &dest_main_file).with_context(|| {
        format!(
          "Failed to copy main usage-rules.md for package {}: {}",
          pkg.name,
          dest_main_file.display()
        )
      })?;
    }

    // Copy sub-files preserving directory structure
    for sub_file in &pkg.content.sub_files {
      let dest_sub_file_path = folder_path
        .join(&pkg.name)
        .join(&sub_file.relative_path_name)
        .with_extension("md");

      if let Some(parent) = dest_sub_file_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
          format!(
            "Failed to create parent directory for sub-file {}: {}",
            sub_file.relative_path_name,
            parent.display()
          )
        })?;
      }

      fs::copy(&sub_file.full_path, &dest_sub_file_path).with_context(|| {
        format!(
          "Failed to copy sub-file {} for package {}: {}",
          sub_file.relative_path_name,
          pkg.name,
          dest_sub_file_path.display()
        )
      })?;
    }
  }

  // Extract folder name from the path for generating relative links
  let folder_name = folder_path
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("usage_rules");

  let content = create_main_agents_file(packages, preamble, Some(folder_name))?;

  fs::write(output_path, content)
    .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::aggregator::PackageContent;
  use tempfile::TempDir;

  fn create_test_package(name: &str, main_content: &str) -> (PackageContentInfo, TempDir) {
    let temp = TempDir::new().unwrap();
    let main_file = temp.path().join("usage-rules.md");
    fs::write(&main_file, main_content).unwrap();

    let package = PackageContentInfo {
      name: name.to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    };

    (package, temp)
  }

  #[test]
  fn test_generate_header_inline_mode() {
    let header = generate_header(false);
    assert!(header.contains("IMPORTANT"));
    assert!(header.contains("General Rust Usage"));
    assert!(!header.contains("separate files"));
  }

  #[test]
  fn test_generate_header_folder_mode() {
    let header = generate_header(true);
    assert!(header.contains("IMPORTANT"));
    assert!(header.contains("separate files"));
  }

  #[test]
  fn test_write_inline_creates_file() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");

    let (pkg, _pkg_temp) = create_test_package("test-pkg", "Test content");
    let packages = vec![pkg];

    write_inline(&output, packages, None).unwrap();

    assert!(output.exists());
    let content = fs::read_to_string(&output).unwrap();

    assert!(content.contains("<!-- cargo-usage-rules-start -->"));
    assert!(content.contains("<!-- cargo-usage-rules-end -->"));
    assert!(content.contains("## test-pkg usage"));
    assert!(content.contains("Test content"));
  }

  #[test]
  fn test_write_inline_preserves_preamble() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");

    let (pkg, _pkg_temp) = create_test_package("test-pkg", "Content");
    let packages = vec![pkg];
    let preamble = "# My Custom Header\n\nCustom preamble text".to_string();

    write_inline(&output, packages, Some(preamble.clone())).unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.starts_with("# My Custom Header"));
    assert!(content.contains("Custom preamble text"));
  }

  #[test]
  fn test_write_inline_empty_preamble_uses_default() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");

    let (pkg, _pkg_temp) = create_test_package("test-pkg", "Content");
    let packages = vec![pkg];

    write_inline(&output, packages, Some(String::new())).unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.contains("IMPORTANT"));
  }

  #[test]
  fn test_write_linked_creates_folder_structure() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");
    let folder = temp.path().join("usage_rules_folder");

    let pkg_temp = TempDir::new().unwrap();
    let main_file = pkg_temp.path().join("usage-rules.md");
    fs::write(&main_file, "Main content").unwrap();

    let packages = vec![PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    }];

    write_linked(&output, &folder, packages, None).unwrap();

    // Check output file exists
    assert!(output.exists());

    // Check folder structure
    assert!(folder.exists());
    assert!(folder.join("test-pkg").exists());
    assert!(folder.join("test-pkg/test-pkg.md").exists());

    // Check main file has markers
    let main_content = fs::read_to_string(&output).unwrap();
    assert!(main_content.contains("<!-- cargo-usage-rules-start -->"));
    assert!(main_content.contains("<!-- cargo-usage-rules-end -->"));
  }

  #[test]
  fn test_write_linked_copies_sub_files() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");
    let folder = temp.path().join("usage_rules");

    let pkg_temp = TempDir::new().unwrap();
    let main_file = pkg_temp.path().join("usage-rules.md");
    fs::write(&main_file, "Main").unwrap();

    let sub_file = pkg_temp.path().join("async.md");
    fs::write(&sub_file, "Async content").unwrap();

    let packages = vec![PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![crate::scanner::UsageRuleSubFile {
          relative_path_name: "async".to_string(),
          full_path: sub_file,
        }],
      },
    }];

    write_linked(&output, &folder, packages, None).unwrap();

    // Check sub-file was copied
    assert!(folder.join("test-pkg/async.md").exists());
    let sub_content = fs::read_to_string(folder.join("test-pkg/async.md")).unwrap();
    assert_eq!(sub_content, "Async content");
  }

  #[test]
  fn test_write_linked_handles_multiple_sub_files() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");
    let folder = temp.path().join("usage_rules");

    let pkg_temp = TempDir::new().unwrap();
    let main_file = pkg_temp.path().join("usage-rules.md");
    fs::write(&main_file, "Main").unwrap();

    let sub_file1 = pkg_temp.path().join("async.md");
    fs::write(&sub_file1, "Async patterns").unwrap();

    let sub_file2 = pkg_temp.path().join("builder.md");
    fs::write(&sub_file2, "Builder pattern").unwrap();

    let packages = vec![PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![
          crate::scanner::UsageRuleSubFile {
            relative_path_name: "async".to_string(),
            full_path: sub_file1,
          },
          crate::scanner::UsageRuleSubFile {
            relative_path_name: "builder".to_string(),
            full_path: sub_file2,
          },
        ],
      },
    }];

    write_linked(&output, &folder, packages, None).unwrap();

    // Check both sub-files were copied
    assert!(folder.join("test-pkg/async.md").exists());
    assert!(folder.join("test-pkg/builder.md").exists());
  }

  #[test]
  fn test_write_linked_preserves_preamble() {
    let temp = TempDir::new().unwrap();
    let output = temp.path().join("output.md");
    let folder = temp.path().join("usage_rules");

    let pkg_temp = TempDir::new().unwrap();
    let main_file = pkg_temp.path().join("usage-rules.md");
    fs::write(&main_file, "Content").unwrap();

    let packages = vec![PackageContentInfo {
      name: "test-pkg".to_string(),
      content: PackageContent {
        main_file: Some(main_file),
        sub_files: vec![],
      },
    }];

    let preamble = "# Custom Header".to_string();

    write_linked(&output, &folder, packages, Some(preamble)).unwrap();

    let content = fs::read_to_string(&output).unwrap();
    assert!(content.starts_with("# Custom Header"));
  }
}
