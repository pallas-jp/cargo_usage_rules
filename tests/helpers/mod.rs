pub mod fixture_builder;

use std::path::PathBuf;

/// Get the path to the test fixtures directory
pub fn fixtures_dir() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Get the path to the test workspace
pub fn test_workspace_dir() -> PathBuf {
  fixtures_dir().join("test-workspace")
}
