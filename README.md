# cargo-usage-rules

A cargo subcommand that aggregates `usage-rules.md` files from your Rust project dependencies into a single file for AI agent consumption.

Inspired by the Elixir [ash-project/usage_rules](https://github.com/ash-project/usage_rules).  They have some pretty compelling evidence of it's efficacy.

## Installation

```sh
cargo install cargo-usage-rules
```

Or with binstall:
```sh
cargo binstall cargo-usage-rules
```

Or from source:
```sh
cargo install --git https://github.com/pallas-jp/cargo_usage_rules
```

## Usage

### Sync all dependencies
```sh
cargo usage-rules sync --all
```

### List available packages with usage rules
```sh
cargo usage-rules list
```

### Sync with custom output file
```sh
cargo usage-rules sync --all --output AI.md
```

### Inline specific packages
```sh
cargo usage-rules sync --inline serde,tokio,clap
```

### Exclude specific packages
```sh
cargo usage-rules sync --all --remove old-crate,deprecated-dep
```

### Create separate files with links (folder mode)
```sh
# Markdown links (default)
cargo usage-rules sync --all --link-to-folder usage-rules

# Claude @ links
cargo usage-rules sync --all --link-to-folder usage-rules --link-style at
```

## Usage Reccomendations

In my experience using the inspiring project, linked mode works great and
doesn't pollute the context window, so syncing all with the default linked mode.

If you have few dependencies then inline might be more performant for you, but YMMV.

## How It Works

`cargo-usage-rules` scans your Rust project dependencies (from crates.io, git, local paths, etc) for:
- `usage-rules.md` files (main usage guidance)

It then aggregates these files into a single output file (default: `Agents.md`)
with clear package boundaries marked by HTML comments.

This project also has some general rust guidance (see [the base file](./base.md)) inspired by various sources (see acknowledgements).

### Creating Usage Rules for Your Crate

To make your crate discoverable by `cargo-usage-rules`, add a `usage-rules.md` file to your crate's root directory:

```markdown
# My Awesome Crate

## Quick Start

[Your usage guidance for AI agents here]

## Common Patterns

[Examples and best practices]

## Common Mistakes

[What to avoid]
```