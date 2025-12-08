use crate::metadata::Dependency;
use anyhow::{Context, Result};
use std::{fs, path::PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct UsageRules {
  pub package_name: String,
  pub package_version: String,
  pub main_file: Option<PathBuf>,
  pub sub_files: Vec<UsageRuleSubFile>,
}

#[derive(Debug, Clone)]
pub struct UsageRuleSubFile {
  pub relative_path_name: String,
  pub full_path: PathBuf,
}

/// Scans dependencies for usage-rules.md files and associated sub-files.
///
/// For each dependency, this function looks for:
/// - A `usage-rules.md` file in the package root
/// - A `usage-rules/` directory containing additional markdown files
///
/// # Arguments
///
/// * `dependencies` - Slice of dependencies to scan
///
/// # Returns
///
/// A vector of `UsageRules` for packages that have usage rules files.
/// Only packages with at least one usage rules file (main or sub-file) are
/// included.
///
/// # Errors
///
/// Returns an error if filesystem operations fail during scanning.
pub fn scan_for_usage_rules(dependencies: &[Dependency]) -> Result<Vec<UsageRules>> {
  let mut results = Vec::new();

  for dep in dependencies {
    let main_file_path = dep.path.join("usage-rules.md");
    let sub_dir_path = dep.path.join("usage_rules");

    let main_file = if main_file_path.exists() && main_file_path.is_file() {
      Some(main_file_path)
    } else {
      None
    };

    if main_file.is_none() {
      continue;
    }

    let mut sub_files = Vec::new();

    if sub_dir_path.exists() && sub_dir_path.is_dir() {
      for entry in WalkDir::new(&sub_dir_path)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
      {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
          if let Ok(relative) = path.strip_prefix(&sub_dir_path) {
            let relative_path_name = relative
              .to_string_lossy()
              .trim_end_matches(".md")
              .to_string();
            sub_files.push(UsageRuleSubFile {
              relative_path_name,
              full_path: path.to_path_buf(),
            });
          }
        }
      }
    }

    results.push(UsageRules {
      package_name: dep.name.clone(),
      package_version: dep.version.clone(),
      main_file,
      sub_files,
    });
  }

  Ok(results)
}

pub fn read_file_content(path: &PathBuf) -> Result<String> {
  fs::read_to_string(path)
    .with_context(|| anyhow::anyhow!("Failed to read file {}", path.display()))
}

#[cfg(test)]
mod tests {
  use super::*;
  use tempfile::TempDir;

  #[test]
  fn test_finds_main_file_only() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();
    fs::write(pkg_path.join("usage-rules.md"), "Main content").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].main_file.is_some());
    assert_eq!(results[0].sub_files.len(), 0);
    assert_eq!(results[0].package_name, "test");
    assert_eq!(results[0].package_version, "1.0.0");
  }

  #[test]
  fn test_finds_main_and_sub_files() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();
    fs::write(pkg_path.join("usage-rules.md"), "Main content").unwrap();

    // Create sub-files in usage_rules directory (underscore!)
    let sub_dir = pkg_path.join("usage_rules");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("async.md"), "Async content").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    assert_eq!(results.len(), 1);
    assert!(results[0].main_file.is_some());
    assert_eq!(results[0].sub_files.len(), 1);
    assert_eq!(results[0].sub_files[0].relative_path_name, "async");
  }

  #[test]
  fn test_skips_package_with_only_sub_files() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();

    // Only create sub-files, no main file
    let sub_dir = pkg_path.join("usage_rules");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("orphan.md"), "Orphan content").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    // Should be skipped because no main file
    assert_eq!(results.len(), 0);
  }

  #[test]
  fn test_finds_nested_sub_files() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();
    fs::write(pkg_path.join("usage-rules.md"), "Main").unwrap();

    let sub_dir = pkg_path.join("usage_rules");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("builder.md"), "Builder pattern").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    assert_eq!(results[0].sub_files.len(), 1);
    assert_eq!(results[0].sub_files[0].relative_path_name, "builder");
  }

  #[test]
  fn test_handles_multiple_sub_files() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();
    fs::write(pkg_path.join("usage-rules.md"), "Main").unwrap();

    let sub_dir = pkg_path.join("usage_rules");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("async.md"), "Async content").unwrap();
    fs::write(sub_dir.join("sync.md"), "Sync content").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    assert_eq!(results[0].sub_files.len(), 2);
  }

  #[test]
  fn test_ignores_non_md_files() {
    let temp = TempDir::new().unwrap();
    let pkg_path = temp.path();
    fs::write(pkg_path.join("usage-rules.md"), "Main").unwrap();

    let sub_dir = pkg_path.join("usage_rules");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("file.md"), "MD file").unwrap();
    fs::write(sub_dir.join("file.txt"), "TXT file").unwrap();
    fs::write(sub_dir.join("README"), "README").unwrap();

    let dep = Dependency {
      name: "test".into(),
      version: "1.0.0".into(),
      path: pkg_path.to_path_buf(),
    };

    let results = scan_for_usage_rules(&[dep]).unwrap();

    // Should only find the .md file
    assert_eq!(results[0].sub_files.len(), 1);
    assert_eq!(results[0].sub_files[0].relative_path_name, "file");
  }

  #[test]
  fn test_handles_multiple_dependencies() {
    let temp = TempDir::new().unwrap();

    // Create two packages
    let pkg1_path = temp.path().join("pkg1");
    let pkg2_path = temp.path().join("pkg2");
    fs::create_dir(&pkg1_path).unwrap();
    fs::create_dir(&pkg2_path).unwrap();

    fs::write(pkg1_path.join("usage-rules.md"), "Pkg1").unwrap();
    fs::write(pkg2_path.join("usage-rules.md"), "Pkg2").unwrap();

    let deps = vec![
      Dependency {
        name: "pkg1".into(),
        version: "1.0.0".into(),
        path: pkg1_path,
      },
      Dependency {
        name: "pkg2".into(),
        version: "2.0.0".into(),
        path: pkg2_path,
      },
    ];

    let results = scan_for_usage_rules(&deps).unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].package_name, "pkg1");
    assert_eq!(results[1].package_name, "pkg2");
  }

  #[test]
  fn test_handles_empty_dependency_list() {
    let results = scan_for_usage_rules(&[]).unwrap();
    assert_eq!(results.len(), 0);
  }

  #[test]
  fn test_read_file_content_success() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("test.md");
    fs::write(&file_path, "Test content").unwrap();

    let content = read_file_content(&file_path).unwrap();
    assert_eq!(content, "Test content");
  }

  #[test]
  fn test_read_file_content_error() {
    let non_existent = PathBuf::from("/nonexistent/path/file.md");
    let result = read_file_content(&non_existent);

    assert!(result.is_err());
    assert!(result
      .unwrap_err()
      .to_string()
      .contains("Failed to read file"));
  }
}
