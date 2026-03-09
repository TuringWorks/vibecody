# Cloud Sandbox IDE

Launch browser-based development environments powered by cloud containers.

## Triggers
- "cloud sandbox", "remote IDE", "cloud dev environment"
- "sandbox instance", "dev container", "browser IDE"

## Usage
```
/sandbox create "my-project" --template rust-dev
/sandbox start sandbox-1
/sandbox stop sandbox-1
/sandbox list
/sandbox sync sandbox-1           # Sync files
/sandbox templates                # List available templates
```

## Features
- Container-based sandbox instances with full lifecycle (Creating, Running, Stopped, Failed, Expired)
- Configurable: image, CPU cores, memory, disk, ports, env vars, workspace path
- 3 built-in templates: Rust Development, Node.js Development, Python Development
- Custom template support with preinstalled package lists
- Auto-generated sandbox URLs (https://{id}.sandbox.vibecody.dev)
- File sync tracking
- Owner-based instance filtering
- Port mapping with protocol specification
