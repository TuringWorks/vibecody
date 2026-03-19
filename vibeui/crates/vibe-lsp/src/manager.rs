//! LSP manager for handling multiple language servers

use crate::client::LspClient;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;

/// LSP manager
pub struct LspManager {
    clients: HashMap<String, LspClient>, // language_id -> client
    server_configs: HashMap<String, (String, Vec<String>)>, // language_id -> (cmd, args)
}

/// LSP server metadata: command, args, install instructions
pub struct LspServerInfo {
    pub command: String,
    pub args: Vec<String>,
    pub install_hint: String,
}

impl LspManager {
    pub fn new() -> Self {
        let mut server_configs = HashMap::new();
        let s = |cmd: &str, args: &[&str]| (cmd.to_string(), args.iter().map(|a| a.to_string()).collect::<Vec<_>>());

        // ── Systems languages ──
        server_configs.insert("rust".into(),         s("rust-analyzer", &[]));
        server_configs.insert("c".into(),            s("clangd", &[]));
        server_configs.insert("cpp".into(),          s("clangd", &[]));
        server_configs.insert("zig".into(),          s("zls", &[]));
        server_configs.insert("nim".into(),          s("nimlangserver", &[]));
        server_configs.insert("d".into(),             s("serve-d", &[]));
        server_configs.insert("dlang".into(),         s("serve-d", &[]));
        server_configs.insert("v".into(),             s("v-analyzer", &[]));
        server_configs.insert("vala".into(),          s("vala-language-server", &[]));

        // ── Web languages ──
        server_configs.insert("typescript".into(),   s("typescript-language-server", &["--stdio"]));
        server_configs.insert("javascript".into(),   s("typescript-language-server", &["--stdio"]));
        server_configs.insert("html".into(),          s("vscode-html-language-server", &["--stdio"]));
        server_configs.insert("css".into(),           s("vscode-css-language-server", &["--stdio"]));
        server_configs.insert("json".into(),          s("vscode-json-language-server", &["--stdio"]));

        // ── JVM languages ──
        server_configs.insert("java".into(),         s("jdtls", &[]));
        server_configs.insert("kotlin".into(),       s("kotlin-language-server", &[]));
        server_configs.insert("scala".into(),        s("metals", &[]));
        server_configs.insert("groovy".into(),       s("groovy-language-server", &[]));
        server_configs.insert("clojure".into(),      s("clojure-lsp", &[]));

        // ── .NET languages ──
        server_configs.insert("csharp".into(),       s("OmniSharp", &["-lsp"]));
        server_configs.insert("fsharp".into(),       s("fsautocomplete", &["--adaptive-lsp-server-enabled"]));
        server_configs.insert("vb".into(),            s("OmniSharp", &["-lsp"]));

        // ── Scripting languages ──
        server_configs.insert("python".into(),       s("pyright-langserver", &["--stdio"]));
        server_configs.insert("ruby".into(),         s("solargraph", &["stdio"]));
        server_configs.insert("php".into(),           s("intelephense", &["--stdio"]));
        server_configs.insert("perl".into(),          s("perl-language-server", &[]));
        server_configs.insert("lua".into(),           s("lua-language-server", &[]));
        server_configs.insert("r".into(),             s("R", &["--slave", "-e", "languageserver::run()"]));

        // ── Go ──
        server_configs.insert("go".into(),           s("gopls", &[]));

        // ── Functional languages ──
        server_configs.insert("haskell".into(),      s("haskell-language-server-wrapper", &["--lsp"]));
        server_configs.insert("elixir".into(),       s("elixir-ls", &[]));
        server_configs.insert("erlang".into(),       s("erlang_ls", &[]));
        server_configs.insert("ocaml".into(),        s("ocamllsp", &[]));
        server_configs.insert("racket".into(),       s("racket-langserver", &[]));
        server_configs.insert("lisp".into(),          s("cl-lsp", &[]));

        // ── Mobile / Apple ──
        server_configs.insert("swift".into(),        s("sourcekit-lsp", &[]));
        server_configs.insert("dart".into(),         s("dart", &["language-server", "--protocol=lsp"]));

        // ── Other compiled ──
        server_configs.insert("crystal".into(),      s("crystalline", &[]));
        server_configs.insert("fortran".into(),      s("fortls", &[]));
        server_configs.insert("pascal".into(),       s("pasls", &[]));
        server_configs.insert("julia".into(),        s("julia", &["--project=@.", "-e", "using LanguageServer; runserver()"]));
        server_configs.insert("prolog".into(),       s("swipl", &["-g", "use_module(library(lsp_server))", "-t", "lsp_server:main"]));

        // ── Markup / Config ──
        server_configs.insert("yaml".into(),         s("yaml-language-server", &["--stdio"]));
        server_configs.insert("toml".into(),         s("taplo", &["lsp", "stdio"]));
        server_configs.insert("dockerfile".into(),   s("docker-langserver", &["--stdio"]));
        server_configs.insert("markdown".into(),     s("marksman", &["server"]));
        server_configs.insert("sql".into(),           s("sqls", &[]));
        server_configs.insert("graphql".into(),      s("graphql-lsp", &["server", "-m", "stream"]));

        // ── CFML ──
        server_configs.insert("cfml".into(),          s("cfml-language-server", &[]));

        Self {
            clients: HashMap::new(),
            server_configs,
        }
    }

    /// Get the full list of supported languages and their LSP server info.
    pub fn supported_languages(&self) -> Vec<(String, String, String)> {
        let install_hints = Self::install_hints();
        self.server_configs.iter().map(|(lang, (cmd, _))| {
            let hint = install_hints.get(lang.as_str()).unwrap_or(&"Check your package manager");
            (lang.clone(), cmd.clone(), hint.to_string())
        }).collect()
    }

    /// Check which LSP servers are available on PATH.
    pub fn check_available(&self) -> Vec<(String, String, bool)> {
        self.server_configs.iter().map(|(lang, (cmd, _))| {
            let available = std::process::Command::new("which").arg(cmd).output()
                .map(|o| o.status.success()).unwrap_or(false);
            (lang.clone(), cmd.clone(), available)
        }).collect()
    }

    fn install_hints() -> HashMap<&'static str, &'static str> {
        let mut h = HashMap::new();
        h.insert("rust", "rustup component add rust-analyzer");
        h.insert("c", "brew install llvm (macOS) | apt install clangd (Linux)");
        h.insert("cpp", "brew install llvm (macOS) | apt install clangd (Linux)");
        h.insert("typescript", "npm i -g typescript-language-server typescript");
        h.insert("javascript", "npm i -g typescript-language-server typescript");
        h.insert("python", "pip install pyright | pip install python-lsp-server");
        h.insert("go", "go install golang.org/x/tools/gopls@latest");
        h.insert("java", "https://github.com/eclipse-jdtls/eclipse.jdt.ls");
        h.insert("kotlin", "https://github.com/fwcd/kotlin-language-server");
        h.insert("scala", "https://scalameta.org/metals/docs/editors/new-editor");
        h.insert("ruby", "gem install solargraph");
        h.insert("php", "npm i -g @intelephense/server");
        h.insert("lua", "brew install lua-language-server");
        h.insert("swift", "Included with Xcode | swift.org/download");
        h.insert("dart", "dart pub global activate dart_language_server");
        h.insert("haskell", "ghcup install hls");
        h.insert("elixir", "https://github.com/elixir-lsp/elixir-ls");
        h.insert("erlang", "https://github.com/erlang-ls/erlang_ls");
        h.insert("ocaml", "opam install ocaml-lsp-server");
        h.insert("crystal", "https://github.com/elbywan/crystalline");
        h.insert("zig", "brew install zls | https://github.com/zigtools/zls");
        h.insert("nim", "nimble install nimlangserver");
        h.insert("d", "https://github.com/Pure-D/serve-d");
        h.insert("csharp", "https://github.com/OmniSharp/omnisharp-roslyn");
        h.insert("fsharp", "dotnet tool install fsautocomplete");
        h.insert("perl", "cpanm Perl::LanguageServer");
        h.insert("r", "R -e 'install.packages(\"languageserver\")'");
        h.insert("fortran", "pip install fortls");
        h.insert("julia", "julia -e 'using Pkg; Pkg.add(\"LanguageServer\")'");
        h.insert("clojure", "brew install clojure-lsp/brew/clojure-lsp-native");
        h.insert("groovy", "https://github.com/GroovyLanguageServer/groovy-language-server");
        h.insert("racket", "raco pkg install racket-langserver");
        h.insert("yaml", "npm i -g yaml-language-server");
        h.insert("toml", "cargo install taplo-cli");
        h.insert("dockerfile", "npm i -g dockerfile-language-server-nodejs");
        h.insert("markdown", "brew install marksman");
        h.insert("sql", "go install github.com/sqls-server/sqls@latest");
        h.insert("graphql", "npm i -g graphql-language-service-cli");
        h.insert("html", "npm i -g vscode-langservers-extracted");
        h.insert("css", "npm i -g vscode-langservers-extracted");
        h.insert("json", "npm i -g vscode-langservers-extracted");
        h.insert("pascal", "https://github.com/castle-engine/pascal-language-server");
        h.insert("v", "https://github.com/nickolasgasworker/v-analyzer");
        h.insert("vala", "https://github.com/vala-lang/vala-language-server");
        h.insert("prolog", "swipl (SWI-Prolog with lsp_server pack)");
        h
    }

    /// Get or create a client for the given language
    pub async fn get_client_for_language(&mut self, language: &str, root_path: &Path) -> Result<&mut LspClient> {
        if !self.clients.contains_key(language) {
            if let Some((cmd, args)) = self.server_configs.get(language) {
                let mut client = LspClient::new(cmd.clone(), args.clone());
                client.initialize(root_path.to_path_buf()).await?;
                self.clients.insert(language.to_string(), client);
            } else {
                return Err(anyhow::anyhow!("No LSP server configured for language: {}", language));
            }
        }
        
        self.clients.get_mut(language)
            .ok_or_else(|| anyhow::anyhow!("LSP client for '{}' missing after initialization", language))
    }

    pub fn add_client(&mut self, language: String, client: LspClient) {
        self.clients.insert(language, client);
    }

    pub fn get_client(&self, language: &str) -> Option<&LspClient> {
        self.clients.get(language)
    }

    pub fn get_client_mut(&mut self, language: &str) -> Option<&mut LspClient> {
        self.clients.get_mut(language)
    }
}

impl Default for LspManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_has_default_configs() {
        let mgr = LspManager::new();
        assert!(mgr.server_configs.contains_key("rust"));
        assert!(mgr.server_configs.contains_key("typescript"));
        assert!(mgr.server_configs.contains_key("javascript"));
        assert!(mgr.server_configs.contains_key("python"));
    }

    #[test]
    fn new_has_four_default_configs() {
        let mgr = LspManager::new();
        assert!(mgr.server_configs.len() >= 40);
    }

    #[test]
    fn new_has_no_clients_initially() {
        let mgr = LspManager::new();
        assert!(mgr.clients.is_empty());
    }

    #[test]
    fn default_rust_config_is_rust_analyzer() {
        let mgr = LspManager::new();
        let (cmd, args) = mgr.server_configs.get("rust").unwrap();
        assert_eq!(cmd, "rust-analyzer");
        assert!(args.is_empty());
    }

    #[test]
    fn default_typescript_config() {
        let mgr = LspManager::new();
        let (cmd, args) = mgr.server_configs.get("typescript").unwrap();
        assert_eq!(cmd, "typescript-language-server");
        assert_eq!(args, &["--stdio"]);
    }

    #[test]
    fn default_python_config_is_pyright() {
        let mgr = LspManager::new();
        let (cmd, _) = mgr.server_configs.get("python").unwrap();
        assert_eq!(cmd, "pyright-langserver");
    }

    #[test]
    fn get_client_for_unknown_language_returns_none() {
        let mgr = LspManager::new();
        assert!(mgr.get_client("haskell").is_none());
    }

    #[test]
    fn get_client_mut_for_unknown_returns_none() {
        let mut mgr = LspManager::new();
        assert!(mgr.get_client_mut("cobol").is_none());
    }

    #[test]
    fn default_is_same_as_new() {
        let mgr = LspManager::default();
        assert!(mgr.server_configs.len() >= 40);
    }

    #[test]
    fn add_client_and_retrieve() {
        let mut mgr = LspManager::new();
        let client = LspClient::new("test-server".to_string(), vec![]);
        mgr.add_client("test-lang".to_string(), client);
        assert!(mgr.get_client("test-lang").is_some());
    }

    #[test]
    fn add_client_is_retrievable_via_get_client_mut() {
        let mut mgr = LspManager::new();
        let client = LspClient::new("server".to_string(), vec![]);
        mgr.add_client("go".to_string(), client);
        assert!(mgr.get_client_mut("go").is_some());
    }

    #[test]
    fn add_client_overwrites_existing() {
        let mut mgr = LspManager::new();
        let client1 = LspClient::new("server-v1".to_string(), vec![]);
        let client2 = LspClient::new("server-v2".to_string(), vec![]);
        mgr.add_client("lang".to_string(), client1);
        mgr.add_client("lang".to_string(), client2);
        // After overwrite, the key should still resolve
        assert!(mgr.get_client("lang").is_some());
    }

    #[test]
    fn javascript_shares_config_with_typescript() {
        let mgr = LspManager::new();
        let (ts_cmd, ts_args) = mgr.server_configs.get("typescript").unwrap();
        let (js_cmd, js_args) = mgr.server_configs.get("javascript").unwrap();
        assert_eq!(ts_cmd, js_cmd);
        assert_eq!(ts_args, js_args);
    }

    #[tokio::test]
    async fn get_client_for_unsupported_language_errors() {
        let mut mgr = LspManager::new();
        let result = mgr.get_client_for_language(
            "brainfuck",
            std::path::Path::new("/tmp"),
        ).await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("brainfuck"));
    }

    #[test]
    fn clients_map_starts_empty() {
        let mgr = LspManager::new();
        assert!(mgr.clients.is_empty());
        assert_eq!(mgr.clients.len(), 0);
    }

    #[test]
    fn add_multiple_clients_tracks_count() {
        let mut mgr = LspManager::new();
        mgr.add_client("go".to_string(), LspClient::new("gopls".to_string(), vec![]));
        mgr.add_client("c".to_string(), LspClient::new("clangd".to_string(), vec![]));
        mgr.add_client("lua".to_string(), LspClient::new("lua-language-server".to_string(), vec![]));
        assert_eq!(mgr.clients.len(), 3);
    }

    #[test]
    fn get_client_returns_none_after_adding_different_language() {
        let mut mgr = LspManager::new();
        mgr.add_client("go".to_string(), LspClient::new("gopls".to_string(), vec![]));
        assert!(mgr.get_client("go").is_some());
        assert!(mgr.get_client("ruby").is_none());
    }

    #[test]
    fn server_configs_include_go_c_java() {
        let mgr = LspManager::new();
        assert!(mgr.server_configs.contains_key("go"));
        assert!(mgr.server_configs.contains_key("c"));
        assert!(mgr.server_configs.contains_key("java"));
    }

    #[tokio::test]
    async fn get_client_for_language_error_message_contains_language_name() {
        let mut mgr = LspManager::new();
        let result = mgr.get_client_for_language("brainfuck", std::path::Path::new("/tmp")).await;
        match result {
            Err(e) => assert!(e.to_string().contains("brainfuck"), "Error should mention the language name"),
            Ok(_) => panic!("Expected error for unsupported language"),
        }
    }

    #[test]
    fn rust_analyzer_has_no_args() {
        let mgr = LspManager::new();
        let (_, args) = mgr.server_configs.get("rust").unwrap();
        assert!(args.is_empty(), "rust-analyzer should have no default args");
    }

    #[test]
    fn python_pyright_has_stdio_arg() {
        let mgr = LspManager::new();
        let (_, args) = mgr.server_configs.get("python").unwrap();
        assert_eq!(args, &["--stdio"]);
    }

    #[test]
    fn add_client_then_get_returns_some() {
        let mut mgr = LspManager::new();
        mgr.add_client("swift".to_string(), LspClient::new("sourcekit-lsp".to_string(), vec![]));
        assert!(mgr.get_client("swift").is_some());
        assert!(mgr.get_client_mut("swift").is_some());
    }

    #[test]
    fn add_client_does_not_affect_server_configs() {
        let mut mgr = LspManager::new();
        let config_count_before = mgr.server_configs.len();
        mgr.add_client("swift".to_string(), LspClient::new("sourcekit-lsp".to_string(), vec![]));
        assert_eq!(mgr.server_configs.len(), config_count_before, "adding a client should not modify server_configs");
    }

    #[test]
    fn default_and_new_produce_same_config_count() {
        let from_new = LspManager::new();
        let from_default = LspManager::default();
        assert_eq!(from_new.server_configs.len(), from_default.server_configs.len());
    }

    #[tokio::test]
    async fn get_client_for_language_unknown_returns_descriptive_error() {
        let mut mgr = LspManager::new();
        let result = mgr.get_client_for_language("cobol", std::path::Path::new("/tmp")).await;
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(msg.contains("No LSP server configured"), "error should mention missing config");
                assert!(msg.contains("cobol"), "error should name the language");
            }
            Ok(_) => panic!("Expected error for unsupported language"),
        }
    }

    #[test]
    fn get_client_after_adding_multiple_languages() {
        let mut mgr = LspManager::new();
        mgr.add_client("go".to_string(), LspClient::new("gopls".to_string(), vec![]));
        mgr.add_client("ruby".to_string(), LspClient::new("solargraph".to_string(), vec![]));
        mgr.add_client("elixir".to_string(), LspClient::new("elixir-ls".to_string(), vec![]));

        assert!(mgr.get_client("go").is_some());
        assert!(mgr.get_client("ruby").is_some());
        assert!(mgr.get_client("elixir").is_some());
        assert!(mgr.get_client("scala").is_none());
    }

    #[test]
    fn overwrite_client_replaces_previous() {
        let mut mgr = LspManager::new();
        mgr.add_client("go".to_string(), LspClient::new("gopls-v1".to_string(), vec![]));
        mgr.add_client("go".to_string(), LspClient::new("gopls-v2".to_string(), vec![]));
        // Should still have exactly one entry for "go"
        assert_eq!(mgr.clients.len(), 1);
        assert!(mgr.get_client("go").is_some());
    }
}
