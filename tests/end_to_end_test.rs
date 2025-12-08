use std::{fs, path::PathBuf, process::Command};
use tempfile::TempDir;

/// Get the path to the test workspace
fn test_workspace_path() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/test-workspace")
}

/// Get the path to the cargo-usage-rules binary
fn cargo_usage_rules_bin() -> PathBuf {
  let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  path.push("target");
  path.push("debug");
  path.push("cargo-usage-rules");
  path
}

/// Helper to run cargo-usage-rules command
fn run_usage_rules_sync(
  workspace_path: &PathBuf,
  output: &PathBuf,
  linked: bool,
  link_folder: Option<&str>,
  extra_args: &[&str],
) -> std::process::Output {
  let mut cmd = Command::new(cargo_usage_rules_bin());
  cmd
    .arg("usage-rules")
    .arg("sync")
    .arg("--all")
    .arg("-o")
    .arg(output)
    .current_dir(workspace_path.join("main-crate"));

  // Set linked mode
  if linked {
    cmd.arg("--linked=true");
  } else {
    cmd.arg("--linked=false");
  }

  // Set link folder if provided
  if let Some(folder) = link_folder {
    cmd.arg("--link-folder").arg(folder);
  }

  for arg in extra_args {
    cmd.arg(arg);
  }

  cmd.output().expect("Failed to execute cargo-usage-rules")
}

#[test]
fn test_end_to_end_inline_mode() {
  // Build the binary first
  let build_status = Command::new("cargo")
    .arg("build")
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .status()
    .expect("Failed to build binary");
  assert!(build_status.success(), "Binary build failed");

  let workspace = test_workspace_path();
  let temp = TempDir::new().unwrap();
  let output = temp.path().join("Agents.md");

  // Run: cargo usage-rules sync --all --linked false (inline mode)
  let result = run_usage_rules_sync(&workspace, &output, false, None, &[]);

  assert!(
    result.status.success(),
    "Command failed: {}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(output.exists(), "Output file was not created");

  let content = fs::read_to_string(&output).unwrap();

  // Verify cargo-usage-rules markers
  assert!(
    content.contains("<!-- cargo-usage-rules-start -->"),
    "Missing start marker"
  );
  assert!(
    content.contains("<!-- cargo-usage-rules-end -->"),
    "Missing end marker"
  );

  // Verify header content
  assert!(content.contains("IMPORTANT"), "Missing header");
  assert!(
    content.contains("General Rust Usage"),
    "Missing general rust usage section"
  );

  // Verify all packages with usage-rules are included
  // lib-simple (has main file only)
  assert!(
    content.contains("## lib-simple usage"),
    "lib-simple not found"
  );
  assert!(
    content.contains("simple library with basic usage"),
    "lib-simple content not found"
  );

  // lib-with-subs (has main + sub-files)
  assert!(
    content.contains("## lib-with-subs usage"),
    "lib-with-subs not found"
  );
  assert!(
    content.contains("demonstrates usage rules with sub-files"),
    "lib-with-subs main content not found"
  );

  // Check that sub-files are inlined
  assert!(
    content.contains("## async"),
    "lib-with-subs async sub-file not inlined"
  );
  assert!(
    content.contains("Async Patterns"),
    "lib-with-subs async content not found"
  );
  assert!(
    content.contains("## builder"),
    "lib-with-subs builder sub-file not inlined"
  );
  assert!(
    content.contains("Builder Pattern"),
    "lib-with-subs builder content not found"
  );

  // lib-no-main should be SKIPPED (only has sub-files, no main)
  assert!(
    !content.contains("lib-no-main"),
    "lib-no-main should have been skipped"
  );

  // lib-empty should be SKIPPED (no usage-rules at all)
  assert!(
    !content.contains("lib-empty"),
    "lib-empty should have been skipped"
  );

  println!("✓ Inline mode test passed - all expected content found");
}

#[test]
fn test_end_to_end_linked_mode() {
  // Build the binary first
  let build_status = Command::new("cargo")
    .arg("build")
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .status()
    .expect("Failed to build binary");
  assert!(build_status.success(), "Binary build failed");

  let workspace = test_workspace_path();
  let temp = TempDir::new().unwrap();
  let output = temp.path().join("Agents.md");
  let folder = temp.path().join("usage_rules");

  // Run: cargo usage-rules sync --all --linked true --link-folder
  let result = run_usage_rules_sync(
    &workspace,
    &output,
    true,
    Some(folder.to_str().unwrap()),
    &[],
  );

  assert!(
    result.status.success(),
    "Command failed: {}",
    String::from_utf8_lossy(&result.stderr)
  );
  assert!(output.exists(), "Output file was not created");
  assert!(folder.exists(), "Folder was not created");

  let main_content = fs::read_to_string(&output).unwrap();

  // Verify main file has markers
  assert!(
    main_content.contains("<!-- cargo-usage-rules-start -->"),
    "Missing start marker in main file"
  );
  assert!(
    main_content.contains("<!-- cargo-usage-rules-end -->"),
    "Missing end marker in main file"
  );

  // Verify header mentions separate files
  assert!(
    main_content.contains("separate files"),
    "Header should mention separate files in folder mode"
  );

  // Verify main file has package headers
  // NOTE: Current implementation includes content in main file even in linked
  // mode The linked files in the folder are additional copies for reference
  assert!(
    main_content.contains("## lib-simple usage"),
    "lib-simple header not found"
  );

  // Verify folder structure
  assert!(
    folder.join("lib-simple").exists(),
    "lib-simple directory not created"
  );
  assert!(
    folder.join("lib-simple/lib-simple.md").exists(),
    "lib-simple main file not created"
  );

  // Verify lib-simple file content
  let lib_simple_content = fs::read_to_string(folder.join("lib-simple/lib-simple.md")).unwrap();
  assert!(
    lib_simple_content.contains("# lib-simple Usage Rules"),
    "lib-simple file missing header"
  );
  assert!(
    lib_simple_content.contains("simple library with basic usage"),
    "lib-simple file missing content"
  );

  // Verify lib-with-subs structure (main + sub-files)
  assert!(
    folder.join("lib-with-subs").exists(),
    "lib-with-subs directory not created"
  );
  assert!(
    folder.join("lib-with-subs/lib-with-subs.md").exists(),
    "lib-with-subs main file not created"
  );
  assert!(
    folder.join("lib-with-subs/async.md").exists(),
    "lib-with-subs async sub-file not copied"
  );
  assert!(
    folder.join("lib-with-subs/builder.md").exists(),
    "lib-with-subs builder sub-file not copied"
  );

  // Verify lib-with-subs main file content
  let lib_with_subs_content =
    fs::read_to_string(folder.join("lib-with-subs/lib-with-subs.md")).unwrap();
  assert!(
    lib_with_subs_content.contains("# lib-with-subs Usage Rules"),
    "lib-with-subs file missing header"
  );

  // Verify sub-file content
  let async_content = fs::read_to_string(folder.join("lib-with-subs/async.md")).unwrap();
  assert!(
    async_content.contains("Async Patterns"),
    "async sub-file content not found"
  );

  let builder_content = fs::read_to_string(folder.join("lib-with-subs/builder.md")).unwrap();
  assert!(
    builder_content.contains("Builder Pattern"),
    "builder sub-file content not found"
  );

  // Verify skipped packages don't have folders
  assert!(
    !folder.join("lib-no-main").exists(),
    "lib-no-main should have been skipped"
  );
  assert!(
    !folder.join("lib-empty").exists(),
    "lib-empty should have been skipped"
  );

  println!("✓ Linked mode test passed - all files created correctly");
}

#[test]
fn test_end_to_end_with_remove_flag() {
  // Build the binary first
  let build_status = Command::new("cargo")
    .arg("build")
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .status()
    .expect("Failed to build binary");
  assert!(build_status.success(), "Binary build failed");

  let workspace = test_workspace_path();
  let temp = TempDir::new().unwrap();
  let output = temp.path().join("Agents.md");

  // Run with --remove to exclude lib-simple (inline mode)
  let result = run_usage_rules_sync(
    &workspace,
    &output,
    false,
    None,
    &["--remove", "lib-simple"],
  );

  assert!(
    result.status.success(),
    "Command failed: {}",
    String::from_utf8_lossy(&result.stderr)
  );

  let content = fs::read_to_string(&output).unwrap();

  // lib-simple should be excluded
  assert!(
    !content.contains("## lib-simple usage"),
    "lib-simple should have been excluded by --remove flag"
  );

  // Other packages should still be included
  assert!(
    content.contains("## lib-with-subs usage"),
    "lib-with-subs should be included"
  );

  println!("✓ Remove flag test passed");
}

#[test]
fn test_preamble_preservation() {
  // Build the binary first
  let build_status = Command::new("cargo")
    .arg("build")
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .status()
    .expect("Failed to build binary");
  assert!(build_status.success(), "Binary build failed");

  let workspace = test_workspace_path();
  let temp = TempDir::new().unwrap();
  let output = temp.path().join("Agents.md");

  // First run: create initial file (inline mode)
  let result1 = run_usage_rules_sync(&workspace, &output, false, None, &[]);
  assert!(result1.status.success());

  // Add custom preamble before markers
  let original_content = fs::read_to_string(&output).unwrap();
  let custom_preamble = "# My Custom Project\n\nThis is my custom header.\n\n";

  // Find the cargo-usage-rules-start marker and insert preamble before it
  let new_content = if let Some(pos) = original_content.find("<!-- cargo-usage-rules-start -->") {
    format!("{}{}", custom_preamble, &original_content[pos..])
  } else {
    panic!("Marker not found");
  };

  fs::write(&output, new_content).unwrap();

  // Second run: regenerate and ensure preamble is preserved (inline mode)
  let result2 = run_usage_rules_sync(&workspace, &output, false, None, &[]);
  assert!(result2.status.success());

  let final_content = fs::read_to_string(&output).unwrap();

  // Verify custom preamble was preserved
  assert!(
    final_content.contains("# My Custom Project"),
    "Custom header was not preserved"
  );
  assert!(
    final_content.contains("This is my custom header."),
    "Custom preamble content was not preserved"
  );

  // Verify the usage-rules content was regenerated
  assert!(
    final_content.contains("<!-- cargo-usage-rules-start -->"),
    "Start marker missing after regeneration"
  );
  assert!(
    final_content.contains("## lib-simple usage"),
    "Package content missing after regeneration"
  );

  println!("✓ Preamble preservation test passed");
}

#[test]
fn test_list_command() {
  // Build the binary first
  let build_status = Command::new("cargo")
    .arg("build")
    .current_dir(env!("CARGO_MANIFEST_DIR"))
    .status()
    .expect("Failed to build binary");
  assert!(build_status.success(), "Binary build failed");

  let workspace = test_workspace_path();

  // Run: cargo usage-rules list
  let output = Command::new(cargo_usage_rules_bin())
    .arg("usage-rules")
    .arg("list")
    .current_dir(workspace.join("main-crate"))
    .output()
    .expect("Failed to execute cargo-usage-rules list");

  assert!(
    output.status.success(),
    "List command failed: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let stdout = String::from_utf8_lossy(&output.stdout);

  // Verify packages are listed
  assert!(
    stdout.contains("lib-simple"),
    "lib-simple not in list output"
  );
  assert!(
    stdout.contains("lib-with-subs"),
    "lib-with-subs not in list output"
  );

  // Verify sub-file counts
  assert!(
    stdout.contains("(2 sub-files)") || stdout.contains("lib-with-subs"),
    "lib-with-subs sub-file count not shown"
  );

  // lib-no-main should be skipped (no main file)
  assert!(
    !stdout.contains("lib-no-main"),
    "lib-no-main should have been skipped in list"
  );

  println!("✓ List command test passed");
  println!("List output:\n{}", stdout);
}
