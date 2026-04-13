# App Server
Unified JSON-RPC 2.0 server dispatcher powering CLI, VS Code extension, and VibeUI over the same wire protocol.

## When to Use
- Exposing VibeCLI capabilities over a language-agnostic JSON-RPC 2.0 interface
- Building VS Code extensions or VibeUI panels that call backend methods by name
- Adding new server-side handlers without touching the transport layer

## Commands
- `AppServer::new()` — create a new dispatcher
- `AppServer::register(method, handler)` — register a named handler
- `AppServer::dispatch(request)` — route a request to the matching handler
- `AppServer::parse_request(json)` — deserialize a JSON string into `RpcRequest`
- `AppServer::handle_raw(json)` — full pipeline: parse → dispatch → serialize

## Examples
```rust
use vibecli_cli::app_server::{AppServer, RpcId};
use serde_json::json;

let mut server = AppServer::new();
server.register("ping", Box::new(|_params| json!("pong")));

// handle a raw JSON-RPC request
let response = server.handle_raw(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#);
// response: {"jsonrpc":"2.0","id":1,"result":"pong"}
```
