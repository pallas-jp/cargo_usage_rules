use anyhow::{Context, Result};
use serde::Deserialize;
use std::{path::PathBuf, process::Command};

#[derive(Debug, Clone)]
pub struct Dependency {
  pub name: String,
  pub version: String,
  pub path: PathBuf,
}

#[derive(Deserialize)]
struct CargoMetadata {
  packages: Vec<Package>,
  #[serde(rename = "workspace_members")]
  _workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
  name: String,
  version: String,
  manifest_path: String,
  dependencies: Vec<PackageDependency>,
}

#[derive(Deserialize)]
struct PackageDependency {
  name: String,
}

/// Fetches all dependencies for the current Rust project using `cargo fetch`.
///
/// This ensures that all dependencies are downloaded and available in the local
/// cargo cache before attempting to scan them for usage rules.
///
/// # Returns
///
/// `Ok(())` if the fetch succeeds.
///
/// # Errors
///
/// Returns an error if:
/// - The `cargo fetch` command fails to execute
/// - The command exits with a non-zero status code
pub fn fetch_dependencies() -> Result<()> {
  let status = Command::new("cargo")
    .arg("fetch")
    .status()
    .context("Failed to execute 'cargo fetch'")?;

  if !status.success() {
    anyhow::bail!("'cargo fetch' failed with status: {}", status);
  }

  Ok(())
}

/// Retrieves metadata for all dependencies in the current Rust project.
///
/// Uses `cargo metadata` to get information about all packages in the
/// dependency graph, including their names, versions, and filesystem paths.
///
/// # Returns
///
/// A vector of `Dependency` structs containing the name, version, and path for
/// each package.
///
/// # Errors
///
/// Returns an error if:
/// - The `cargo metadata` command fails to execute
/// - The command exits with a non-zero status code
/// - The JSON output cannot be parsed
pub fn get_dependencies() -> Result<Vec<Dependency>> {
  let output = Command::new("cargo")
    .args(["metadata", "--format-version", "1"])
    .output()
    .context("Failed to execute 'cargo metadata'")?;

  if !output.status.success() {
    let stderr = String::from_utf8_lossy(&output.stderr);
    anyhow::bail!("'cargo metadata' failed: {}", stderr);
  }

  // Get all the top level dependencies of the current project.
  let cargo_package_name_full = String::from_utf8(
    Command::new("cargo")
      .args(["tree", "--depth", "0", "--format", "{p}"])
      .output()
      .context("Failed to execute 'cargo pkgid'")?
      .stdout,
  )
  .context("Failed to parse cargo tree output as utf-8")?;

  let cargo_package_name = cargo_package_name_full
    .trim()
    .split_ascii_whitespace()
    .next()
    .context("Cargo tree package output malformed")?;

  let metadata: CargoMetadata =
    serde_json::from_slice(&output.stdout).context("Failed to parse cargo metadata JSON")?;

  let package_dep_names: Vec<_> = metadata
    .packages
    .iter()
    .find(|pkg| pkg.name == cargo_package_name)
    .context(format!(
      "Cargo package name {cargo_package_name} not found in metadata"
    ))?
    .dependencies
    .iter()
    .map(|d| d.name.clone())
    .collect();

  Ok(
    metadata
      .packages
      .iter()
      .filter_map(|p| {
        if package_dep_names.contains(&p.name) {
          let manifest_path = PathBuf::from(&p.manifest_path);
          let path = manifest_path
            .parent()
            .expect("Failed to get package path")
            .to_path_buf();
          Some(Dependency {
            name: p.name.clone(),
            version: p.version.clone(),
            path,
          })
        } else {
          None
        }
      })
      .collect(),
  )
}
