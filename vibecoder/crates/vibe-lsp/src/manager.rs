//! LSP manager for handling multiple language servers

use crate::client::LspClient;
use crate::discovery::{server_available, ServerSearchPaths};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

/// LSP manager
pub struct LspManager {
    /// language_id -> live client. `Arc` so a caller can await a slow request
    /// after releasing the manager lock; otherwise one cold rust-analyzer
    /// blocks hover and completion in every other open language.
    clients: HashMap<String, Arc<LspClient>>,
    server_configs: HashMap<String, (String, Vec<String>)>, // language_id -> (cmd, args)
    /// Languages we already failed to start, and why.
    ///
    /// Without this, Monaco's per-keystroke completion retries the spawn on
    /// every character: for an uninstalled server that is a process-spawn storm
    /// and a guaranteed multi-second stall per keypress.
    unavailable: HashMap<String, String>,
    /// Where to look for server binaries. Held so tests can inject a fixture
    /// directory instead of depending on what the developer has installed.
    search_paths: ServerSearchPaths,
}

/// LSP server metadata: command, args, install instructions
pub struct LspServerInfo {
    pub command: String,
    pub args: Vec<String>,
    pub install_hint: String,
}

/// Whether a language can currently get IntelliSense, and why not if it can't.
///
/// The frontend uses this to stop asking: an "unsupported" answer is cached, so
/// a `.cbl` file never pays for a failed lookup twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguageStatus {
    /// A server is running and initialized.
    Running,
    /// A server is configured and its binary is installed, but not started yet.
    Available { command: String },
    /// A server is configured, but its binary isn't installed.
    NotInstalled {
        command: String,
        install_hint: String,
    },
    /// We have no server configured for this language at all.
    Unconfigured,
    /// We tried and it failed.
    Failed { reason: String },
}

impl LspManager {
    pub fn new() -> Self {
        let mut server_configs = HashMap::new();
        let s = |cmd: &str, args: &[&str]| {
            (
                cmd.to_string(),
                args.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            )
        };

        // ── Systems languages ──
        server_configs.insert("rust".into(), s("rust-analyzer", &[]));
        server_configs.insert("c".into(), s("clangd", &[]));
        server_configs.insert("cpp".into(), s("clangd", &[]));
        server_configs.insert("zig".into(), s("zls", &[]));
        server_configs.insert("nim".into(), s("nimlangserver", &[]));
        server_configs.insert("d".into(), s("serve-d", &[]));
        server_configs.insert("v".into(), s("v-analyzer", &[]));
        server_configs.insert("vala".into(), s("vala-language-server", &[]));
        server_configs.insert("odin".into(), s("ols", &[]));
        server_configs.insert("gleam".into(), s("gleam", &["lsp"]));
        // CUDA is a clangd language; `.cu` / `.cuh` need no separate server.
        server_configs.insert("cuda".into(), s("clangd", &[]));

        // ── Web languages ──
        server_configs.insert(
            "typescript".into(),
            s("typescript-language-server", &["--stdio"]),
        );
        server_configs.insert(
            "javascript".into(),
            s("typescript-language-server", &["--stdio"]),
        );
        server_configs.insert(
            "html".into(),
            s("vscode-html-language-server", &["--stdio"]),
        );
        server_configs.insert("css".into(), s("vscode-css-language-server", &["--stdio"]));
        server_configs.insert(
            "json".into(),
            s("vscode-json-language-server", &["--stdio"]),
        );
        // Component frameworks. These are *not* covered by Monaco's built-in
        // HTML service even though `.vue` / `.svelte` highlight as HTML: the
        // real value (script blocks, props, type-checked templates) only comes
        // from the framework's own server. See `hasBuiltinLanguageService`.
        server_configs.insert("svelte".into(), s("svelteserver", &["--stdio"]));
        server_configs.insert("vue".into(), s("vue-language-server", &["--stdio"]));
        server_configs.insert("astro".into(), s("astro-ls", &["--stdio"]));
        server_configs.insert(
            "purescript".into(),
            s("purescript-language-server", &["--stdio"]),
        );
        server_configs.insert(
            "rescript".into(),
            s("rescript-language-server", &["--stdio"]),
        );
        server_configs.insert("elm".into(), s("elm-language-server", &["--stdio"]));

        // ── Infrastructure / build ──
        server_configs.insert("terraform".into(), s("terraform-ls", &["serve"]));
        server_configs.insert("nix".into(), s("nil", &[]));
        server_configs.insert("cmake".into(), s("cmake-language-server", &[]));
        server_configs.insert("protobuf".into(), s("protols", &[]));

        // ── Shaders / hardware description ──
        server_configs.insert("glsl".into(), s("glsl_analyzer", &[]));
        server_configs.insert("wgsl".into(), s("wgsl-analyzer", &[]));
        server_configs.insert("systemverilog".into(), s("svls", &[]));
        server_configs.insert("vhdl".into(), s("vhdl_ls", &[]));

        // ── Typesetting / shell ──
        server_configs.insert("latex".into(), s("texlab", &[]));
        server_configs.insert("nushell".into(), s("nu", &["--lsp"]));

        // ── JVM languages ──
        server_configs.insert("java".into(), s("jdtls", &[]));
        server_configs.insert("kotlin".into(), s("kotlin-language-server", &[]));
        server_configs.insert("scala".into(), s("metals", &[]));
        server_configs.insert("groovy".into(), s("groovy-language-server", &[]));
        server_configs.insert("clojure".into(), s("clojure-lsp", &[]));

        // ── .NET languages ──
        server_configs.insert("csharp".into(), s("OmniSharp", &["-lsp"]));
        server_configs.insert(
            "fsharp".into(),
            s("fsautocomplete", &["--adaptive-lsp-server-enabled"]),
        );
        server_configs.insert("vb".into(), s("OmniSharp", &["-lsp"]));

        // ── Scripting languages ──
        server_configs.insert("python".into(), s("pyright-langserver", &["--stdio"]));
        server_configs.insert("ruby".into(), s("solargraph", &["stdio"]));
        server_configs.insert("php".into(), s("intelephense", &["--stdio"]));
        server_configs.insert("perl".into(), s("perl-language-server", &[]));
        server_configs.insert("lua".into(), s("lua-language-server", &[]));
        server_configs.insert(
            "r".into(),
            s("R", &["--slave", "-e", "languageserver::run()"]),
        );

        // ── Go ──
        server_configs.insert("go".into(), s("gopls", &[]));

        // ── Functional languages ──
        server_configs.insert(
            "haskell".into(),
            s("haskell-language-server-wrapper", &["--lsp"]),
        );
        server_configs.insert("elixir".into(), s("elixir-ls", &[]));
        server_configs.insert("erlang".into(), s("erlang_ls", &[]));
        server_configs.insert("ocaml".into(), s("ocamllsp", &[]));
        server_configs.insert("racket".into(), s("racket-langserver", &[]));
        server_configs.insert("lisp".into(), s("cl-lsp", &[]));

        // ── Mobile / Apple ──
        server_configs.insert("swift".into(), s("sourcekit-lsp", &[]));
        server_configs.insert(
            "dart".into(),
            s("dart", &["language-server", "--protocol=lsp"]),
        );
        // Objective-C is a first-class clangd language, not an approximation.
        server_configs.insert("objective-c".into(), s("clangd", &[]));

        // ── Scientific / engineering ──
        server_configs.insert("matlab".into(), s("matlab-language-server", &["--stdio"]));

        // ── Systems / low-level ──
        server_configs.insert("asm".into(), s("asm-lsp", &[]));
        server_configs.insert("ada".into(), s("ada_language_server", &[]));

        // ── Enterprise / legacy ──
        // SuperBOL is the maintained COBOL LSP (GnuCOBOL ecosystem).
        server_configs.insert("cobol".into(), s("superbol-free", &["lsp"]));
        server_configs.insert("sas".into(), s("sas-lsp", &["--stdio"]));
        server_configs.insert("abap".into(), s("abaplsp", &[]));
        // PL/SQL and T-SQL get generic SQL analysis from `sqls` — dialect-aware
        // servers for either do not exist. Named separately (rather than folded
        // into `sql`) so the status bar can say which dialect it resolved, and
        // so a real PL/SQL server can be swapped in here later.
        server_configs.insert("plsql".into(), s("sqls", &[]));
        server_configs.insert("tsql".into(), s("sqls", &[]));

        // ── Scripting / shell / web3 ──
        server_configs.insert(
            "powershell".into(),
            s("powershell-editor-services", &["--stdio"]),
        );
        server_configs.insert("bash".into(), s("bash-language-server", &["start"]));
        server_configs.insert(
            "solidity".into(),
            s("nomicfoundation-solidity-language-server", &["--stdio"]),
        );

        // ── Other compiled ──
        server_configs.insert("crystal".into(), s("crystalline", &[]));
        server_configs.insert("fortran".into(), s("fortls", &[]));
        server_configs.insert("pascal".into(), s("pasls", &[]));
        server_configs.insert(
            "julia".into(),
            s(
                "julia",
                &["--project=@.", "-e", "using LanguageServer; runserver()"],
            ),
        );
        server_configs.insert(
            "prolog".into(),
            s(
                "swipl",
                &[
                    "-g",
                    "use_module(library(lsp_server))",
                    "-t",
                    "lsp_server:main",
                ],
            ),
        );

        // ── Markup / Config ──
        server_configs.insert("yaml".into(), s("yaml-language-server", &["--stdio"]));
        server_configs.insert("toml".into(), s("taplo", &["lsp", "stdio"]));
        server_configs.insert("dockerfile".into(), s("docker-langserver", &["--stdio"]));
        server_configs.insert("markdown".into(), s("marksman", &["server"]));
        server_configs.insert("sql".into(), s("sqls", &[]));
        server_configs.insert(
            "graphql".into(),
            s("graphql-lsp", &["server", "-m", "stream"]),
        );

        // ── CFML ──
        server_configs.insert("cfml".into(), s("cfml-language-server", &[]));

        Self {
            clients: HashMap::new(),
            server_configs,
            unavailable: HashMap::new(),
            search_paths: ServerSearchPaths::from_env(),
        }
    }

    /// Same table of servers, but searching an explicit set of directories.
    /// Lets tests exercise availability and start-up without depending on
    /// whatever the developer happens to have installed.
    pub fn with_search_paths(search_paths: ServerSearchPaths) -> Self {
        Self {
            search_paths,
            ..Self::new()
        }
    }

    /// Get the full list of supported languages and their LSP server info.
    pub fn supported_languages(&self) -> Vec<(String, String, String)> {
        let install_hints = Self::install_hints();
        self.server_configs
            .iter()
            .map(|(lang, (cmd, _))| {
                let hint = install_hints
                    .get(lang.as_str())
                    .unwrap_or(&"Check your package manager");
                (lang.clone(), cmd.clone(), hint.to_string())
            })
            .collect()
    }

    /// Check which LSP servers are installed.
    ///
    /// A directory scan, not ~60 `which` subprocesses — and it searches the
    /// same standard install dirs we spawn from, so a GUI-launched app doesn't
    /// report every server as missing just because launchd's `PATH` is bare.
    pub fn check_available(&self) -> Vec<(String, String, bool)> {
        self.server_configs
            .iter()
            .map(|(lang, (cmd, _))| {
                let available = server_available(cmd, &self.search_paths);
                (lang.clone(), cmd.clone(), available)
            })
            .collect()
    }

    /// What IntelliSense this language can currently get.
    pub fn language_status(&self, language: &str) -> LanguageStatus {
        if let Some(client) = self.clients.get(language) {
            if client.is_alive() {
                return LanguageStatus::Running;
            }
        }
        if let Some(reason) = self.unavailable.get(language) {
            return LanguageStatus::Failed {
                reason: reason.clone(),
            };
        }
        let Some((cmd, _)) = self.server_configs.get(language) else {
            return LanguageStatus::Unconfigured;
        };
        if server_available(cmd, &self.search_paths) {
            LanguageStatus::Available {
                command: cmd.clone(),
            }
        } else {
            LanguageStatus::NotInstalled {
                command: cmd.clone(),
                install_hint: Self::install_hints()
                    .get(language)
                    .unwrap_or(&"Check your package manager")
                    .to_string(),
            }
        }
    }

    /// Languages with a live server right now.
    pub fn running_languages(&self) -> Vec<String> {
        self.clients
            .iter()
            .filter(|(_, client)| client.is_alive())
            .map(|(lang, _)| lang.clone())
            .collect()
    }

    fn install_hints() -> HashMap<&'static str, &'static str> {
        let mut h = HashMap::new();
        h.insert("rust", "rustup component add rust-analyzer");
        h.insert(
            "c",
            "brew install llvm (macOS) | apt install clangd (Linux)",
        );
        h.insert(
            "cpp",
            "brew install llvm (macOS) | apt install clangd (Linux)",
        );
        h.insert(
            "typescript",
            "npm i -g typescript-language-server typescript",
        );
        h.insert(
            "javascript",
            "npm i -g typescript-language-server typescript",
        );
        h.insert(
            "python",
            "pip install pyright | pip install python-lsp-server",
        );
        h.insert("go", "go install golang.org/x/tools/gopls@latest");
        h.insert("java", "https://github.com/eclipse-jdtls/eclipse.jdt.ls");
        h.insert("kotlin", "https://github.com/fwcd/kotlin-language-server");
        h.insert(
            "scala",
            "https://scalameta.org/metals/docs/editors/new-editor",
        );
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
        h.insert(
            "clojure",
            "brew install clojure-lsp/brew/clojure-lsp-native",
        );
        h.insert(
            "groovy",
            "https://github.com/GroovyLanguageServer/groovy-language-server",
        );
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
        h.insert(
            "pascal",
            "https://github.com/castle-engine/pascal-language-server",
        );
        h.insert("v", "https://github.com/nickolasgasworker/v-analyzer");
        h.insert("vala", "https://github.com/vala-lang/vala-language-server");
        h.insert("prolog", "swipl (SWI-Prolog with lsp_server pack)");
        h.insert(
            "objective-c",
            "brew install llvm (macOS) | apt install clangd",
        );
        // Pre-existing servers that had no hint: "not installed" with no way to
        // fix it is only half a message.
        h.insert("vb", "https://github.com/OmniSharp/omnisharp-roslyn");
        h.insert("lisp", "https://github.com/cxxxr/cl-lsp");
        h.insert("cfml", "https://github.com/KamasamaK/vscode-cfml");
        // Modern / emerging languages
        h.insert("odin", "https://github.com/DanielGavin/ols");
        h.insert(
            "gleam",
            "Included with the Gleam toolchain — https://gleam.run/getting-started",
        );
        h.insert("cuda", "brew install llvm (macOS) | apt install clangd");
        // Component frameworks
        h.insert("svelte", "npm i -g svelte-language-server");
        h.insert("vue", "npm i -g @vue/language-server");
        h.insert("astro", "npm i -g @astrojs/language-server");
        h.insert("purescript", "npm i -g purescript-language-server");
        h.insert("rescript", "npm i -g @rescript/language-server");
        h.insert("elm", "npm i -g @elm-tooling/elm-language-server");
        // Infrastructure / build
        h.insert("terraform", "brew install hashicorp/tap/terraform-ls");
        h.insert("nix", "nix profile install nixpkgs#nil");
        h.insert("cmake", "pip install cmake-language-server");
        h.insert("protobuf", "cargo install protols");
        // Shaders / hardware description
        h.insert("glsl", "https://github.com/nolanderc/glsl_analyzer");
        h.insert(
            "wgsl",
            "cargo install --git https://github.com/wgsl-analyzer/wgsl-analyzer wgsl-analyzer",
        );
        h.insert("systemverilog", "cargo install svls");
        h.insert(
            "vhdl",
            "cargo install --git https://github.com/VHDL-LS/rust_hdl vhdl_ls",
        );
        // Typesetting / shell
        h.insert("latex", "brew install texlab | cargo install texlab");
        h.insert("nushell", "Included with Nushell — https://www.nushell.sh");
        h.insert(
            "matlab",
            "https://github.com/mathworks/MATLAB-language-server",
        );
        h.insert("asm", "cargo install asm-lsp");
        h.insert(
            "ada",
            "alr install ada_language_server | https://github.com/AdaCore/ada_language_server",
        );
        h.insert(
            "cobol",
            "opam install superbol-free | https://github.com/OCamlPro/superbol-studio-oss",
        );
        h.insert("sas", "https://github.com/sassoftware/vscode-sas-extension");
        h.insert("abap", "https://github.com/abaplint/abaplint-vscode");
        h.insert(
            "plsql",
            "go install github.com/sqls-server/sqls@latest (generic SQL — no PL/SQL-specific server exists)",
        );
        h.insert(
            "tsql",
            "go install github.com/sqls-server/sqls@latest (generic SQL — no T-SQL-specific server exists)",
        );
        h.insert(
            "powershell",
            "https://github.com/PowerShell/PowerShellEditorServices",
        );
        h.insert("bash", "npm i -g bash-language-server");
        h.insert(
            "solidity",
            "npm i -g @nomicfoundation/solidity-language-server",
        );
        h
    }

    /// Get or start a client for the given language.
    ///
    /// Returns a handle, not a borrow: the caller drops the manager lock before
    /// awaiting the actual request, so a slow server in one language cannot
    /// stall every other language's completion behind the same mutex.
    ///
    /// A start failure is remembered — see [`Self::unavailable`] — and every
    /// later call for that language fails immediately with the same message
    /// until [`Self::retry_language`] clears it.
    pub async fn get_client_for_language(
        &mut self,
        language: &str,
        root_path: &Path,
    ) -> Result<Arc<LspClient>> {
        // A dead client (server crashed, or was killed by the OS) must be
        // replaced, not reused — every request through it would time out.
        if let Some(existing) = self.clients.get(language) {
            if existing.is_alive() {
                return Ok(Arc::clone(existing));
            }
            tracing::info!("{language}: language server died; restarting");
            self.clients.remove(language);
        }

        if let Some(reason) = self.unavailable.get(language) {
            return Err(anyhow!("{reason}"));
        }

        let (cmd, args) = self
            .server_configs
            .get(language)
            .cloned()
            .ok_or_else(|| anyhow!("No LSP server configured for language: {language}"))?;

        // Check before spawning so "not installed" is an install hint rather
        // than a `No such file or directory` from deep inside tokio.
        if !server_available(&cmd, &self.search_paths) {
            let hint = Self::install_hints()
                .get(language)
                .copied()
                .unwrap_or("Check your package manager");
            let reason = format!(
                "'{cmd}' is not installed, so {language} has no IntelliSense. Install it: {hint}"
            );
            self.unavailable
                .insert(language.to_string(), reason.clone());
            return Err(anyhow!("{reason}"));
        }

        let mut client = LspClient::new(cmd.clone(), args);
        if let Err(e) = client.initialize(root_path.to_path_buf()).await {
            let reason = format!("'{cmd}' failed to start for {language}: {e}");
            self.unavailable
                .insert(language.to_string(), reason.clone());
            return Err(anyhow!("{reason}"));
        }

        let client = Arc::new(client);
        self.clients
            .insert(language.to_string(), Arc::clone(&client));
        Ok(client)
    }

    /// Forget that a language's server failed, so the next request retries it.
    /// Called after the user installs a server, or from a "restart server" action.
    pub fn retry_language(&mut self, language: &str) {
        self.unavailable.remove(language);
        self.search_paths = ServerSearchPaths::from_env();
    }

    /// Stop a language's server and allow a fresh start on the next request.
    pub async fn restart_language(&mut self, language: &str) {
        if let Some(client) = self.clients.remove(language) {
            let _ = client.shutdown().await;
        }
        self.retry_language(language);
    }

    /// Stop every server. Call on app shutdown so we don't orphan processes.
    pub async fn shutdown_all(&mut self) {
        for (_, client) in std::mem::take(&mut self.clients) {
            let _ = client.shutdown().await;
        }
    }

    pub fn add_client(&mut self, language: String, client: LspClient) {
        self.clients.insert(language, Arc::new(client));
    }

    pub fn get_client(&self, language: &str) -> Option<Arc<LspClient>> {
        self.clients.get(language).map(Arc::clone)
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
    fn get_client_for_unstarted_language_returns_none() {
        let mgr = LspManager::new();
        assert!(mgr.get_client("cobol").is_none());
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
    fn added_client_handle_can_be_cloned_out() {
        let mut mgr = LspManager::new();
        mgr.add_client(
            "go".to_string(),
            LspClient::new("server".to_string(), vec![]),
        );
        let handle = mgr.get_client("go").expect("client present");
        assert_eq!(handle.command(), "server");
        // A second handle to the same client, not a second client.
        assert!(mgr.get_client("go").is_some());
        assert_eq!(mgr.clients.len(), 1);
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
        let result = mgr
            .get_client_for_language("brainfuck", std::path::Path::new("/tmp"))
            .await;
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
        mgr.add_client(
            "go".to_string(),
            LspClient::new("gopls".to_string(), vec![]),
        );
        mgr.add_client(
            "c".to_string(),
            LspClient::new("clangd".to_string(), vec![]),
        );
        mgr.add_client(
            "lua".to_string(),
            LspClient::new("lua-language-server".to_string(), vec![]),
        );
        assert_eq!(mgr.clients.len(), 3);
    }

    #[test]
    fn get_client_returns_none_after_adding_different_language() {
        let mut mgr = LspManager::new();
        mgr.add_client(
            "go".to_string(),
            LspClient::new("gopls".to_string(), vec![]),
        );
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
        let result = mgr
            .get_client_for_language("brainfuck", std::path::Path::new("/tmp"))
            .await;
        match result {
            Err(e) => assert!(
                e.to_string().contains("brainfuck"),
                "Error should mention the language name"
            ),
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
        mgr.add_client(
            "swift".to_string(),
            LspClient::new("sourcekit-lsp".to_string(), vec![]),
        );
        assert!(mgr.get_client("swift").is_some());
    }

    #[test]
    fn add_client_does_not_affect_server_configs() {
        let mut mgr = LspManager::new();
        let config_count_before = mgr.server_configs.len();
        mgr.add_client(
            "swift".to_string(),
            LspClient::new("sourcekit-lsp".to_string(), vec![]),
        );
        assert_eq!(
            mgr.server_configs.len(),
            config_count_before,
            "adding a client should not modify server_configs"
        );
    }

    #[test]
    fn default_and_new_produce_same_config_count() {
        let from_new = LspManager::new();
        let from_default = LspManager::default();
        assert_eq!(
            from_new.server_configs.len(),
            from_default.server_configs.len()
        );
    }

    #[tokio::test]
    async fn get_client_for_language_unknown_returns_descriptive_error() {
        // Deliberately a language nobody would configure — `cobol` used to
        // stand in here and became a false negative once COBOL gained a server.
        let mut mgr = LspManager::new();
        let result = mgr
            .get_client_for_language("malbolge", std::path::Path::new("/tmp"))
            .await;
        match result {
            Err(e) => {
                let msg = e.to_string();
                assert!(
                    msg.contains("No LSP server configured"),
                    "error should mention missing config"
                );
                assert!(msg.contains("malbolge"), "error should name the language");
            }
            Ok(_) => panic!("Expected error for unsupported language"),
        }
    }

    #[test]
    fn get_client_after_adding_multiple_languages() {
        let mut mgr = LspManager::new();
        mgr.add_client(
            "go".to_string(),
            LspClient::new("gopls".to_string(), vec![]),
        );
        mgr.add_client(
            "ruby".to_string(),
            LspClient::new("solargraph".to_string(), vec![]),
        );
        mgr.add_client(
            "elixir".to_string(),
            LspClient::new("elixir-ls".to_string(), vec![]),
        );

        assert!(mgr.get_client("go").is_some());
        assert!(mgr.get_client("ruby").is_some());
        assert!(mgr.get_client("elixir").is_some());
        assert!(mgr.get_client("scala").is_none());
    }

    #[test]
    fn overwrite_client_replaces_previous() {
        let mut mgr = LspManager::new();
        mgr.add_client(
            "go".to_string(),
            LspClient::new("gopls-v1".to_string(), vec![]),
        );
        mgr.add_client(
            "go".to_string(),
            LspClient::new("gopls-v2".to_string(), vec![]),
        );
        // Should still have exactly one entry for "go"
        assert_eq!(mgr.clients.len(), 1);
        assert!(mgr.get_client("go").is_some());
    }

    // ── Availability, status, and the negative cache ─────────────────────────

    /// A manager whose server search finds nothing installed.
    fn manager_with_no_servers() -> LspManager {
        LspManager::with_search_paths(ServerSearchPaths {
            path_entries: vec![std::path::PathBuf::from("/nonexistent-bin")],
            extra_prefixes: vec![],
        })
    }

    #[test]
    fn unconfigured_language_status() {
        let mgr = manager_with_no_servers();
        assert_eq!(
            mgr.language_status("brainfuck"),
            LanguageStatus::Unconfigured
        );
    }

    #[test]
    fn configured_but_missing_binary_reports_install_hint() {
        let mgr = manager_with_no_servers();
        match mgr.language_status("rust") {
            LanguageStatus::NotInstalled {
                command,
                install_hint,
            } => {
                assert_eq!(command, "rust-analyzer");
                assert!(install_hint.contains("rustup"), "{install_hint}");
            }
            other => panic!("expected NotInstalled, got {other:?}"),
        }
    }

    #[test]
    fn every_configured_language_has_a_status_that_is_not_unconfigured() {
        let mgr = manager_with_no_servers();
        let languages: Vec<String> = mgr.server_configs.keys().cloned().collect();
        for language in languages {
            assert_ne!(
                mgr.language_status(&language),
                LanguageStatus::Unconfigured,
                "{language} is configured, so its status must say so"
            );
        }
    }

    #[test]
    fn check_available_covers_every_configured_language() {
        let mgr = manager_with_no_servers();
        let reported = mgr.check_available();
        assert_eq!(reported.len(), mgr.server_configs.len());
        assert!(
            reported.iter().all(|(_, _, available)| !available),
            "nothing is installed under a bogus search path"
        );
    }

    #[tokio::test]
    async fn missing_server_is_reported_once_and_then_cached() {
        // The point of the cache: Monaco asks for completion on every
        // keystroke, and each miss used to attempt a fresh process spawn.
        let mut mgr = manager_with_no_servers();
        let root = std::path::Path::new("/tmp");

        let first = mgr
            .get_client_for_language("rust", root)
            .await
            .expect_err("nothing installed");
        assert!(first.to_string().contains("not installed"), "{first}");
        assert!(
            first.to_string().contains("rustup"),
            "must say how to fix it"
        );

        assert!(
            mgr.unavailable.contains_key("rust"),
            "failure is remembered"
        );

        let second = mgr
            .get_client_for_language("rust", root)
            .await
            .expect_err("still nothing installed");
        assert_eq!(first.to_string(), second.to_string());
        assert_eq!(
            mgr.language_status("rust"),
            LanguageStatus::Failed {
                reason: first.to_string()
            }
        );
    }

    #[tokio::test]
    async fn retry_language_clears_the_negative_cache() {
        let mut mgr = manager_with_no_servers();
        let _ = mgr
            .get_client_for_language("go", std::path::Path::new("/tmp"))
            .await;
        assert!(mgr.unavailable.contains_key("go"));

        mgr.retry_language("go");
        assert!(!mgr.unavailable.contains_key("go"));
        // A retry re-reads PATH, so a server installed since startup is found.
        assert!(!matches!(
            mgr.language_status("go"),
            LanguageStatus::Failed { .. }
        ));
    }

    #[tokio::test]
    async fn unconfigured_language_is_not_negatively_cached() {
        // No spawn was attempted, so there is nothing to remember — and the
        // answer can never change at runtime.
        let mut mgr = manager_with_no_servers();
        let _ = mgr
            .get_client_for_language("brainfuck", std::path::Path::new("/tmp"))
            .await;
        assert!(!mgr.unavailable.contains_key("brainfuck"));
    }

    #[test]
    fn a_dead_client_is_not_reported_as_running() {
        // `add_client` inserts an un-started client: alive == false.
        let mut mgr = LspManager::new();
        mgr.add_client("go".into(), LspClient::new("gopls".into(), vec![]));
        assert_ne!(mgr.language_status("go"), LanguageStatus::Running);
        assert!(mgr.running_languages().is_empty());
    }

    #[tokio::test]
    async fn get_client_for_language_replaces_a_dead_client() {
        // A crashed server must not be handed out again — every request through
        // it would fail. The manager drops it and tries a fresh start.
        let mut mgr = manager_with_no_servers();
        mgr.add_client(
            "rust".into(),
            LspClient::new("rust-analyzer".into(), vec![]),
        );
        let err = mgr
            .get_client_for_language("rust", std::path::Path::new("/tmp"))
            .await
            .expect_err("restart is attempted, and fails: nothing installed");
        assert!(err.to_string().contains("not installed"), "{err}");
        assert!(
            !mgr.clients.contains_key("rust"),
            "the dead client must be dropped"
        );
    }

    #[tokio::test]
    async fn shutdown_all_drops_every_client() {
        let mut mgr = LspManager::new();
        mgr.add_client("go".into(), LspClient::new("gopls".into(), vec![]));
        mgr.add_client(
            "rust".into(),
            LspClient::new("rust-analyzer".into(), vec![]),
        );
        mgr.shutdown_all().await;
        assert!(mgr.clients.is_empty());
    }

    // ── TIOBE coverage ───────────────────────────────────────────────────────

    /// The TIOBE top 30 (April 2026 ranking, mirroring
    /// `vibecoder/src/hooks/useLanguageRegistry.ts`) paired with the language id
    /// that must resolve to a server — the id the frontend's
    /// `lspLanguageForPath` produces for that language's files.
    ///
    /// `None` means "no language server exists for this, and pretending
    /// otherwise would be a lie": Scratch is a block language with no text
    /// files, and VB6 has no LSP implementation anywhere.
    const TIOBE_TOP_30: [(u32, &str, Option<&str>); 30] = [
        (1, "Python", Some("python")),
        (2, "C", Some("c")),
        (3, "C++", Some("cpp")),
        (4, "Java", Some("java")),
        (5, "C#", Some("csharp")),
        (6, "JavaScript", Some("javascript")),
        (7, "Visual Basic", Some("vb")),
        (8, "SQL", Some("sql")),
        (9, "R", Some("r")),
        (10, "Delphi/Object Pascal", Some("pascal")),
        (11, "Scratch", None),
        (12, "Perl", Some("perl")),
        (13, "Fortran", Some("fortran")),
        (14, "PHP", Some("php")),
        (15, "Go", Some("go")),
        (16, "Rust", Some("rust")),
        (17, "MATLAB", Some("matlab")),
        (18, "Assembly", Some("asm")),
        (19, "Swift", Some("swift")),
        (20, "Ada", Some("ada")),
        (21, "PL/SQL", Some("plsql")),
        (22, "Prolog", Some("prolog")),
        (23, "COBOL", Some("cobol")),
        (24, "Kotlin", Some("kotlin")),
        (25, "SAS", Some("sas")),
        (26, "Classic Visual Basic", None),
        (27, "Objective-C", Some("objective-c")),
        (28, "Dart", Some("dart")),
        (29, "Ruby", Some("ruby")),
        (30, "Lua", Some("lua")),
    ];

    #[test]
    fn every_tiobe_top_30_language_has_a_configured_server() {
        // The product commitment. A language whose server config is renamed or
        // dropped fails here rather than becoming a silently dead file type.
        let mgr = LspManager::new();
        let missing: Vec<String> = TIOBE_TOP_30
            .iter()
            .filter_map(|(rank, name, language)| {
                let language = (*language)?;
                (!mgr.server_configs.contains_key(language))
                    .then(|| format!("#{rank} {name} (expected id `{language}`)"))
            })
            .collect();
        assert!(
            missing.is_empty(),
            "TIOBE top-30 languages with no configured LSP server: {missing:#?}"
        );
    }

    #[test]
    fn every_configured_server_has_an_install_hint() {
        // "Not installed" is only actionable with the command that installs it.
        let mgr = LspManager::new();
        let hints = LspManager::install_hints();
        let missing: Vec<&String> = mgr
            .server_configs
            .keys()
            .filter(|language| !hints.contains_key(language.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "configured servers with no install hint: {missing:#?}"
        );
    }

    #[test]
    fn languages_without_a_server_are_declared_deliberately() {
        // Guards the *shape* of the commitment: if someone marks a language
        // `None` to make the coverage test pass, this says why that is allowed.
        let declared: Vec<&str> = TIOBE_TOP_30
            .iter()
            .filter(|(_, _, language)| language.is_none())
            .map(|(_, name, _)| *name)
            .collect();
        assert_eq!(
            declared,
            vec!["Scratch", "Classic Visual Basic"],
            "only a block language and a dialect with no LSP in existence may \
             be exempt; adding an entry here needs a reason in the table comment"
        );
    }

    #[test]
    fn sql_dialects_are_named_separately_from_generic_sql() {
        // PL/SQL and T-SQL share `sqls`, but keep their own ids so the status
        // bar can name the dialect and a real server can be swapped in later.
        let mgr = LspManager::new();
        for dialect in ["plsql", "tsql"] {
            let (cmd, _) = mgr
                .server_configs
                .get(dialect)
                .unwrap_or_else(|| panic!("{dialect} must be configured"));
            assert_eq!(cmd, "sqls", "{dialect} currently rides on generic SQL");
        }
        let hints = LspManager::install_hints();
        for dialect in ["plsql", "tsql"] {
            assert!(
                hints[dialect].contains("generic SQL"),
                "{dialect}'s hint must admit it is not dialect-aware"
            );
        }
    }

    /// Languages outside the TIOBE list that people actually work in daily.
    /// Not a ranking — a list of "if this is missing, someone notices".
    const POPULAR_BEYOND_TIOBE: [(&str, &str); 22] = [
        ("zig", "zls"),
        ("nim", "nimlangserver"),
        ("crystal", "crystalline"),
        ("v", "v-analyzer"),
        ("d", "serve-d"),
        ("vala", "vala-language-server"),
        ("odin", "ols"),
        ("gleam", "gleam"),
        ("elixir", "elixir-ls"),
        ("elm", "elm-language-server"),
        ("purescript", "purescript-language-server"),
        ("rescript", "rescript-language-server"),
        ("svelte", "svelteserver"),
        ("vue", "vue-language-server"),
        ("astro", "astro-ls"),
        ("terraform", "terraform-ls"),
        ("nix", "nil"),
        ("cmake", "cmake-language-server"),
        ("protobuf", "protols"),
        ("latex", "texlab"),
        ("wgsl", "wgsl-analyzer"),
        ("cuda", "clangd"),
    ];

    #[test]
    fn popular_non_tiobe_languages_have_the_expected_server() {
        let mgr = LspManager::new();
        let wrong: Vec<String> = POPULAR_BEYOND_TIOBE
            .iter()
            .filter_map(
                |(language, expected)| match mgr.server_configs.get(*language) {
                    Some((cmd, _)) if cmd == expected => None,
                    Some((cmd, _)) => Some(format!(
                        "{language}: configured as `{cmd}`, expected `{expected}`"
                    )),
                    None => Some(format!(
                        "{language}: not configured (expected `{expected}`)"
                    )),
                },
            )
            .collect();
        assert!(wrong.is_empty(), "{wrong:#?}");
    }

    #[test]
    fn no_server_is_configured_under_two_names() {
        // `dlang` used to duplicate `d` with the same binary and no extension
        // routed to it — a config that could never be reached and would drift.
        let mgr = LspManager::new();
        assert!(
            !mgr.server_configs.contains_key("dlang"),
            "`dlang` is an unreachable alias of `d`"
        );
    }

    #[test]
    fn objective_c_uses_clangd() {
        // Not an approximation — clangd supports Objective-C directly.
        let mgr = LspManager::new();
        let (cmd, _) = mgr
            .server_configs
            .get("objective-c")
            .expect("objective-c configured");
        assert_eq!(cmd, "clangd");
    }

    #[tokio::test]
    async fn restart_language_removes_the_client_and_clears_the_cache() {
        let mut mgr = manager_with_no_servers();
        mgr.add_client("go".into(), LspClient::new("gopls".into(), vec![]));
        let _ = mgr
            .get_client_for_language("python", std::path::Path::new("/tmp"))
            .await;

        mgr.restart_language("go").await;
        assert!(!mgr.clients.contains_key("go"));
        assert!(
            mgr.unavailable.contains_key("python"),
            "unrelated cache kept"
        );
    }
}
