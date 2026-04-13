# Windows Sandbox Policy
Enforce Windows-style ACL path and network isolation using pure policy logic — no actual OS API calls.

## When to Use
- Restricting which filesystem paths an agent or plugin may access
- Blocking outbound network access except to explicitly allow-listed hosts
- Simulating restricted-token (low-privilege) execution environments on any platform

## Commands
- `WindowsSandboxConfig::default_restricted()` — deny-all baseline (no paths, no internet, restricted token)
- `config.allow_path(prefix)` — builder: permit paths starting with `prefix`
- `config.deny_path(prefix)` — builder: explicitly deny paths (takes precedence over allow)
- `WindowsSandbox::new(config)` — create sandbox from config
- `sandbox.check_path(path)` — returns `SandboxVerdict { allowed, reason }`
- `sandbox.check_network(host)` — returns `SandboxVerdict { allowed, reason }`
- `sandbox.is_restricted_token()` — query token restriction flag
- `sandbox.audit_path_rules()` — total rule count for reporting

## Examples
```rust
let cfg = WindowsSandboxConfig::default_restricted()
    .allow_path("/workspace")
    .deny_path("/workspace/secret");

let sb = WindowsSandbox::new(cfg);

let v = sb.check_path("/workspace/src/main.rs");
assert!(v.allowed);

let v2 = sb.check_path("/workspace/secret/key.pem");
assert!(!v2.allowed); // deny wins over allow
```
