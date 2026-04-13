# Plugin Bundle
`.vibepkg` plugin bundle format — manifest validation, install, uninstall, and list for VibeCLI plugin bundles.

## When to Use
- Packaging and distributing VibeCLI skill collections as `.vibepkg` bundles
- Validating manifests before installation (name, author, version)
- Managing an installed-bundle registry (add, remove, lookup by name)

## Commands
- `BundleVersion::parse("1.2.3")` — parse a semver-style version string
- `BundleVersion::is_compatible_with(min)` — check self >= min version
- `BundleManifest::validate()` — verify required fields are non-empty
- `BundleRegistry::install(manifest, path, now)` — validate and add bundle
- `BundleRegistry::uninstall(name)` — remove by name
- `BundleRegistry::find(name)` — lookup an installed bundle
- `BundleRegistry::list()` — iterate all installed bundles

## Examples
```rust
use vibecli_cli::plugin_bundle::{BundleManifest, BundleRegistry, BundleVersion};

let manifest = BundleManifest {
    name: "vibe-git".into(),
    version: BundleVersion::parse("1.0.0").unwrap(),
    author: "VibeTeam".into(),
    description: "Git integration skills".into(),
    skills: vec!["git-commit.md".into()],
    mcp_configs: vec![],
    min_vibecli_version: Some("0.9.0".into()),
};

let mut registry = BundleRegistry::new();
registry.install(manifest, "/plugins/vibe-git", 1700000000).unwrap();
let found = registry.find("vibe-git").unwrap();
println!("Installed at: {}", found.install_path);
```
