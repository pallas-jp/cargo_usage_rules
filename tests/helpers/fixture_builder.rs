use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Builder for creating test fixtures programmatically
pub struct FixtureBuilder {
  temp_dir: TempDir,
  packages: Vec<PackageFixture>,
}

pub struct PackageFixture {
  name: String,
  main_content: Option<String>,
  sub_files: Vec<(String, String)>, // (relative_path, content)
}

impl FixtureBuilder {
  /// Create a new fixture builder with a temporary directory
  pub fn new() -> Result<Self> {
    Ok(Self {
      temp_dir: TempDir::new()?,
      packages: Vec::new(),
    })
  }

  /// Add a package to the fixture
  pub fn with_package(mut self, name: &str) -> PackageBuilder {
    PackageBuilder {
      fixture: self,
      package_name: name.to_string(),
      main_content: None,
      sub_files: Vec::new(),
    }
  }

  /// Build the fixture and return the temporary directory path
  pub fn build(self) -> Result<TempDir> {
    for package in self.packages {
      let pkg_dir = self.temp_dir.path().join(&package.name);
      fs::create_dir_all(&pkg_dir)?;

      // Create main usage-rules.md if provided
      if let Some(content) = package.main_content {
        fs::write(pkg_dir.join("usage-rules.md"), content)?;
      }

      // Create sub-files if provided
      if !package.sub_files.is_empty() {
        let sub_dir = pkg_dir.join("usage_rules");
        fs::create_dir_all(&sub_dir)?;

        for (relative_path, content) in package.sub_files {
          let file_path = sub_dir.join(&relative_path).with_extension("md");

          // Create parent directories if needed
          if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
          }

          fs::write(file_path, content)?;
        }
      }
    }

    Ok(self.temp_dir)
  }

  /// Get the path to the temporary directory
  pub fn path(&self) -> &Path {
    self.temp_dir.path()
  }
}

pub struct PackageBuilder {
  fixture: FixtureBuilder,
  package_name: String,
  main_content: Option<String>,
  sub_files: Vec<(String, String)>,
}

impl PackageBuilder {
  /// Set the main usage-rules.md content
  pub fn with_main_file(mut self, content: &str) -> Self {
    self.main_content = Some(content.to_string());
    self
  }

  /// Add a sub-file in the usage_rules directory
  /// Path should be relative (e.g., "async" or "patterns/builder")
  pub fn with_sub_file(mut self, relative_path: &str, content: &str) -> Self {
    self.sub_files.push((relative_path.to_string(), content.to_string()));
    self
  }

  /// Finish building this package and return to the fixture builder
  pub fn done(mut self) -> FixtureBuilder {
    self.fixture.packages.push(PackageFixture {
      name: self.package_name,
      main_content: self.main_content,
      sub_files: self.sub_files,
    });
    self.fixture
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_fixture_builder_creates_structure() {
    let fixture = FixtureBuilder::new()
      .unwrap()
      .with_package("test-pkg")
      .with_main_file("Main content")
      .with_sub_file("async", "Async content")
      .done()
      .build()
      .unwrap();

    let pkg_path = fixture.path().join("test-pkg");
    assert!(pkg_path.join("usage-rules.md").exists());
    assert!(pkg_path.join("usage_rules/async.md").exists());

    let main_content = fs::read_to_string(pkg_path.join("usage-rules.md")).unwrap();
    assert_eq!(main_content, "Main content");
  }

  #[test]
  fn test_fixture_builder_nested_sub_files() {
    let fixture = FixtureBuilder::new()
      .unwrap()
      .with_package("test-pkg")
      .with_main_file("Main")
      .with_sub_file("patterns/builder", "Builder pattern")
      .done()
      .build()
      .unwrap();

    let sub_file = fixture
      .path()
      .join("test-pkg/usage_rules/patterns/builder.md");
    assert!(sub_file.exists());

    let content = fs::read_to_string(sub_file).unwrap();
    assert_eq!(content, "Builder pattern");
  }
}
