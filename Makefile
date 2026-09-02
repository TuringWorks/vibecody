# VibeCody — Developer Makefile
#
# Every surface gets a consistent  build-<surface>  and  test-<surface>  target.
# Run `make help` for the full list, or `make help-surfaces` for the matrix.
#
#   Surface            Dev            Build               Test
#   ─────────────────  ─────────────  ──────────────────  ──────────────────
#   VibeCLI (Rust)     make cli-run   make build-cli      make test-cli
#   VibeCoder  (Tauri)    make ui        make build-ui       make test-ui
#   VibeAIChat (Tauri)    make aichat       make build-aichat      make test-aichat
#   VibeDesk   (Tauri)    make vibedesk     make build-vibedesk    make test-vibedesk
#   Agent SDK (TS)     —              make build-sdk      make test-sdk
#   vibe-indexer       —              make build-indexer  make test-indexer
#   vibe-memory        —              make build-memory   make test-memory
#   vibe-rl-py (uv)    —              make build-rl       make test-rl
#   VS Code ext        —              make build-vscode   make lint-vscode
#   JetBrains plugin   —              make build-jetbrains make test-jetbrains
#   Mobile (Flutter)   —              make build-mobile   make test-mobile
#   Watch (iOS/Wear)   —              make build-watch    make test-watch
#
# Aggregates:
#   make build         Desktop shells (cli + ui + app + vibedesk)
#   make build-apps    The three Tauri shells (ui + app + vibedesk)
#   make build-all     Desktop + mobile + watch
#   make test          Rust workspace tests (fast path)
#   make test-all      Every ecosystem's tests (Rust + Node + Flutter + Python)
#   make ci            Mirror the GitHub CI gate locally
#   make check / lint  Fast type-checks / linters

.PHONY: help help-surfaces setup doctor \
        ui aichat vibedesk cli cli-run \
        build build-apps build-cli build-ui build-aichat build-vibedesk \
        build-sdk build-indexer build-memory build-rl build-vscode build-jetbrains \
        test test-fast test-all test-rust \
        test-cli test-ai test-core test-indexer test-memory \
        test-ui test-aichat test-vibedesk test-sdk test-mobile test-rl test-jetbrains test-watch \
        check check-cli check-ui check-aichat check-vibedesk \
        lint lint-ui lint-sdk lint-vscode lint-vibedesk check-neovim \
        fmt fmt-check ci analyze-mobile icons icons-check \
        mobile-setup mobile-ios mobile-ios-ipa mobile-android mobile-android-bundle \
        mobile-clean watch-ios watch-ios-archive watch-wear watch-wear-bundle \
        watch-clean build-mobile build-watch \
        codesign-macos codesign-verify stage-artifacts \
        clean docker docker-run

# ── Host shell ────────────────────────────────────────────────────────────────
# Every recipe below is POSIX sh. On macOS and Linux that is what make already
# uses. Native Windows make (chocolatey, "Built for Windows32") instead takes
# SHELL from the first sh.exe on PATH — and chocolatey ships one: a UnxUtils
# port that reports itself as zsh and cannot fork, so every recipe with a pipe
# or a conditional dies with "abnormal program termination / fork failed".
#
# Git for Windows ships a real bash, but its installer deliberately keeps it
# off PATH (the same trap vibe_core::shell documents for the Rust side), so
# find it by globbing. `$(wildcard)` is pure make — no shell — which matters
# because the shell is the thing that is broken. Note the escaped space and
# the absence of $(firstword): these paths contain a space, so anything that
# splits on whitespace truncates them to "D:/Program".
#
# Override with `make GIT_BASH=/path/to/bash.exe` if Git lives elsewhere.
#
# MSYSTEM is set when make was launched from Git Bash itself; there the
# default shell and $(HOME) are already correct and none of this applies.
ifeq ($(OS)$(MSYSTEM),Windows_NT)
GIT_BASH ?=
ifeq ($(GIT_BASH),)
GIT_BASH := $(wildcard $(subst \,/,$(LOCALAPPDATA))/Programs/Git/bin/bash.exe)
endif
ifeq ($(GIT_BASH),)
GIT_BASH := $(wildcard C:/Program\ Files/Git/bin/bash.exe)
endif
ifeq ($(GIT_BASH),)
GIT_BASH := $(wildcard D:/Program\ Files/Git/bin/bash.exe)
endif
ifeq ($(GIT_BASH),)
GIT_BASH := $(wildcard C:/Program\ Files\ (x86)/Git/bin/bash.exe)
endif
ifeq ($(GIT_BASH),)
$(warning Git for Windows bash not found — recipes will run under whatever)
$(warning sh.exe is on PATH, which is usually chocolatey's UnxUtils port and)
$(warning cannot fork. Install Git for Windows, or pass GIT_BASH=<path>.)
else
SHELL := $(GIT_BASH)
.SHELLFLAGS := -c
# Put Git's POSIX tools ahead of the chocolatey shims so grep/awk/sed/cut are
# the ones these recipes were written against. `;` is the separator here —
# using `:` is what corrupted PATH below.
export PATH := $(patsubst %/bin/bash.exe,%/usr/bin,$(GIT_BASH));$(PATH)
endif
endif

# Executable suffix, so `--binary target/release/vibecli$(EXE)` names a file
# that exists on every host.
ifeq ($(OS),Windows_NT)
EXE := .exe
else
EXE :=
endif

# Ensure ~/.cargo/bin is in PATH (fixes npm rustup shadowing on Linux).
# Not on native Windows: $(HOME) is unset there and the separator is `;`, so
# this appended "/.cargo/bin:" to the *first* real entry and destroyed it —
# which is why Flutter reported "Unable to find git in your PATH".
ifneq ($(OS)$(MSYSTEM),Windows_NT)
export PATH := $(HOME)/.cargo/bin:$(PATH)
endif

# ── Toolchain locations ───────────────────────────────────────────────────────
CARGO            ?= cargo
NPM              ?= npm
UV               ?= uv
FLUTTER          ?= flutter
XCODEBUILD       ?= xcodebuild
GRADLE_JETBRAINS := ./gradlew
GRADLE_WEAR      := ./gradlew
MOBILE_DIR       := vibemobile
WATCH_IOS_DIR    := vibewatch
WATCH_IOS_PROJ   := VibeCodyWatch.xcodeproj
WATCH_IOS_SCHEME := VibeCodyWatch
WATCH_WEAR_DIR   := vibewatch/VibeCodyWear
SDK_DIR          := packages/agent-sdk
RL_DIR           := vibe-rl-py
VSCODE_DIR       := vscode-extension
JETBRAINS_DIR    := jetbrains-plugin

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-22s\033[0m %s\n", $$1, $$2}'

help-surfaces: ## Print the per-surface build/test matrix (from the header)
	@sed -n '4,28p' $(MAKEFILE_LIST)

# ── Setup ──────────────────────────────────────────────────────────────────────

setup: ## Install all prerequisites (Rust, Node, system libs, npm deps)
ifeq ($(OS),Windows_NT)
	@powershell -NoProfile -ExecutionPolicy Bypass -File scripts/setup.ps1
else
	@bash scripts/setup.sh
endif

doctor: ## Verify development environment is ready
	@echo "Checking development environment..."
	@echo ""
	@printf "  %-20s" "Rust:" && (rustc --version 2>/dev/null || echo "MISSING — run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh")
	@printf "  %-20s" "Cargo:" && (cargo --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "Node.js:" && (node --version 2>/dev/null || echo "MISSING — install from https://nodejs.org/")
	@printf "  %-20s" "npm:" && (npm --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "Git:" && (git --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "uv (vibe-rl-py):" && (uv --version 2>/dev/null || echo "not installed (needed for test-rl) — https://docs.astral.sh/uv/")
	@printf "  %-20s" "Ollama:" && (ollama --version 2>/dev/null || echo "not installed (optional)") | tail -1
	@printf "  %-20s" "Docker:" && (docker --version 2>/dev/null || echo "not installed (optional)")
	@printf "  %-20s" "JDK (watch-wear):" && \
		if [ -f vibewatch/VibeCodyWear/.java-version ]; then \
			pin=$$(cat vibewatch/VibeCodyWear/.java-version | tr -d '[:space:]'); \
			pin_major=$$(echo "$$pin" | cut -d. -f1); \
			if [ "$$pin_major" = "17" ] || [ "$$pin_major" = "21" ]; then \
				echo "pinned to $$pin via .java-version (compatible with AGP 8.7.3)"; \
			else \
				echo "pinned to $$pin — INCOMPATIBLE with AGP 8.7.3; run: cd vibewatch/VibeCodyWear && jenv local 21"; \
			fi; \
		else \
			ver=$$(java -version 2>&1 | awk -F'"' '/version/{print $$2}' | cut -d. -f1); \
			if [ -z "$$ver" ]; then \
				echo "MISSING — install: brew install openjdk@21 && cd vibewatch/VibeCodyWear && jenv local 21"; \
			elif [ "$$ver" = "17" ] || [ "$$ver" = "21" ]; then \
				echo "no pin; current java is $$ver (compatible)"; \
			else \
				echo "no pin; current java is $$ver — INCOMPATIBLE with AGP 8.7.3; run: cd vibewatch/VibeCodyWear && jenv local 21"; \
			fi; \
		fi
	@printf "  %-20s" "Flutter:" && (flutter --version 2>/dev/null | head -1 || echo "not installed (needed for mobile-*)")
	@echo ""
	@echo "Required: Rust, Cargo, Node.js, npm, Git"
	@echo "Optional: uv (vibe-rl-py), Ollama (local AI), Docker (container sandbox), JDK 17/21 (watch-wear), Flutter (mobile-*)"
ifeq ($(shell uname -s),Linux)
	@echo ""
	@echo "Linux — checking Tauri system dependencies..."
	@for dep in libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev; do \
		printf "  %-36s" "$$dep:" && (dpkg -s $$dep 2>/dev/null | grep -q "ok installed" && echo "OK" || echo "MISSING — run: make setup"); \
	done
endif
ifeq ($(shell uname -s),Darwin)
	@printf "  %-20s" "Xcode:" && (xcodebuild -version 2>/dev/null | head -1 || echo "not installed (needed for watch-ios + mobile-ios)")
endif

# ── node_modules guards (install on first use, only when missing) ─────────────

vibecoder/node_modules:
	cd vibecoder && $(NPM) install --no-audit --no-fund

vibeaichat/node_modules:
	cd vibeaichat && $(NPM) install --no-audit --no-fund

vibedesk/node_modules:
	cd vibedesk && $(NPM) install --no-audit --no-fund

$(SDK_DIR)/node_modules:
	cd $(SDK_DIR) && $(NPM) install --no-audit --no-fund

$(VSCODE_DIR)/node_modules:
	cd $(VSCODE_DIR) && $(NPM) install --no-audit --no-fund

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeCLI — Rust CLI + daemon + TUI (vibecli/vibecli-cli)
# ══════════════════════════════════════════════════════════════════════════════

cli: build-cli ## (alias) Build VibeCLI release binary

build-cli: ## Build VibeCLI release binary → target/release/vibecli
	$(CARGO) build --release -p vibecli
	@echo ""
	@ls -lh target/release/vibecli$(EXE)
	@echo ""
	@echo "Binary: target/release/vibecli$(EXE)"

cli-run: ## Build and run VibeCLI with TUI
	$(CARGO) run --release -p vibecli -- --tui

test-cli: ## Test VibeCLI crate
	$(CARGO) test -p vibecli

check-cli: ## Fast type-check VibeCLI crate
	$(CARGO) check -p vibecli

# ── Developer Excellence ──────────────────────────────────────────────────────
# Measurement over this repository itself. `devex` reports; `devex-gate` is the
# CI shape. The gate is deliberately NOT wired into `ci` — this repository tags
# releases irregularly, so the honest result today is exit 3 ("a required metric
# could not be measured here"), and a gate that starts life red teaches everyone
# to ignore it. Turn it on once releases are tagged from the pipeline.

.PHONY: devex devex-report devex-gate

devex: ## Developer Excellence scorecard for this repository
	$(CARGO) run --release -p vibecli -- --devex scorecard --path .

devex-report: ## The scorecard as a markdown briefing on stdout
	$(CARGO) run --release -p vibecli -- --devex report --path .

devex-gate: ## CI check: fail when lead time or change failure rate falls below "high"
	$(CARGO) run --release -p vibecli -- --devex gate --path . \
		--require-lead-time high \
		--require-change-failure-rate high

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeCoder — desktop editor (Tauri 2 + React)
# ══════════════════════════════════════════════════════════════════════════════

ui: vibecoder/node_modules ## Run VibeCoder in dev mode (Vite + Tauri)
	cd vibecoder && $(NPM) run tauri:dev

build-ui: vibecoder/node_modules ## Build VibeCoder for production (Tauri bundle)
	./scripts/tauri-build.sh vibecoder

test-ui: vibecoder/node_modules ## Test VibeCoder (vitest)
	cd vibecoder && $(NPM) test

check-ui: vibecoder/node_modules ## Type-check VibeCoder (tsc --noEmit)
	cd vibecoder && npx tsc --noEmit

lint-ui: vibecoder/node_modules ## Lint VibeCoder (eslint)
	cd vibecoder && $(NPM) run lint

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeAIChat — secondary Tauri shell (vibeaichat)
# ══════════════════════════════════════════════════════════════════════════════

aichat: vibeaichat/node_modules ## Run VibeAIChat in dev mode
	cd vibeaichat && $(NPM) run tauri:dev

build-aichat: vibeaichat/node_modules ## Build VibeAIChat for production (Tauri bundle)
	./scripts/tauri-build.sh vibeaichat

test-aichat: check-aichat ## Test VibeAIChat (typecheck only — no unit suite yet)

check-aichat: vibeaichat/node_modules ## Type-check VibeAIChat (tsc --noEmit)
	cd vibeaichat && npx tsc --noEmit

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeDesk — Tauri shell (vibedesk)
# ══════════════════════════════════════════════════════════════════════════════

vibedesk: vibedesk/node_modules ## Run VibeDesk in dev mode
	cd vibedesk && $(NPM) run tauri:dev

build-vibedesk: vibedesk/node_modules ## Build VibeDesk for production (Tauri bundle)
	./scripts/tauri-build.sh vibedesk

test-vibedesk: check-vibedesk lint-vibedesk parity-vibedesk ## Test VibeDesk (typecheck + guards)

check-vibedesk: vibedesk/node_modules ## Type-check VibeDesk (tsc --noEmit)
	cd vibedesk && npx tsc --noEmit

lint-vibedesk: vibedesk/node_modules ## Run VibeDesk no-inline-edit lint guard
	cd vibedesk && $(NPM) run lint:no-inline-edit

parity-vibedesk: vibedesk/node_modules ## Check every invoke() has a registered Tauri handler
	cd vibedesk && $(NPM) run lint:invoke-parity

# Needs a browser binary (`npx playwright install chromium`), so it is a local
# target rather than a CI matrix entry — the layout it guards only exists once a
# real engine has laid the box out.
lint-welcome: vibecoder/node_modules ## Check the welcome heading is not clipped on a short window
	cd vibecoder && $(NPM) run lint:welcome-layout

# ── Desktop apps aggregate (the three Tauri shells) ───────────────────────────

# Built one at a time, staging artifacts between each.
#
# All three shells bundle into the *shared* workspace target dir, and Tauri
# clears `target/release/bundle/dmg/` before writing its own .dmg — so a
# straight `build-ui build-aichat build-vibedesk` produces three DMGs and
# leaves exactly one on disk. The .app bundles survive (different
# subdirectory), which is what makes the loss easy to miss: you check the apps,
# see all three, and never notice two installers are gone.
#
# CI does not hit this because each shell builds on its own runner with its own
# target dir. It only bites local release builds.
build-apps: ## Build all three Tauri shells (ui + app + vibedesk), staging artifacts
	$(MAKE) build-ui && $(MAKE) stage-artifacts
	$(MAKE) build-aichat && $(MAKE) stage-artifacts
	$(MAKE) build-vibedesk && $(MAKE) stage-artifacts
	@echo ""
	@echo "Artifacts staged in dist/ — the bundle dirs keep only the last build's DMG:"
	@ls -1 dist/ 2>/dev/null || true

stage-artifacts: ## Copy freshly-built bundles into dist/ before the next build clears them
	@mkdir -p dist
	@find target/release/bundle vibecoder/src-tauri/target/release/bundle \
	      vibeaichat/src-tauri/target/release/bundle vibedesk/src-tauri/target/release/bundle \
	      \( -name '*.dmg' -o -name '*.deb' -o -name '*.AppImage' -o -name '*.msi' \) \
	      -type f -not -name 'rw.*' 2>/dev/null \
	  | while read -r f; do cp -f "$$f" dist/; done || true


test-apps: test-ui test-aichat test-vibedesk ## Test all three Tauri shells

# ── Voice: the speech sidecar ─────────────────────────────────────────────────
#
# Nothing built this for most of its life, so `tts_sidecar` was unset on every
# real machine and the daemon fell through to `say` — one process per utterance,
# no streaming, and the system default voice. The feature existed in the code
# and not on anyone's machine, and the symptom was "the assistant sounds
# mechanical".
#
# Installed beside the daemon binary, which is where `discover_sidecar` looks
# first: that copy is guaranteed to match the daemon that spawns it.

VIBECLI_BIN_DIR ?= $(HOME)/.local/bin

voice-sidecar: ## Build + install the streaming speech sidecar (macOS)
	@test "$$(uname)" = "Darwin" || { echo "voice-sidecar is macOS-only; other platforms use batch synthesis"; exit 0; }
	swiftc -O tools/voice-duplex/sidecar/tts.swift -o target/release/vibecli-tts
	@mkdir -p "$(VIBECLI_BIN_DIR)"
	install -m 0755 target/release/vibecli-tts "$(VIBECLI_BIN_DIR)/vibecli-tts"
	@echo "installed $(VIBECLI_BIN_DIR)/vibecli-tts"
	@"$(VIBECLI_BIN_DIR)/vibecli-tts" --list >/dev/null 2>&1 	  && echo "  verified: it runs and can enumerate voices" 	  || echo "  WARNING: installed but --list failed; the daemon will fall back to batch"

# Kokoro is opt-in and cannot be zero-config: it needs a Python environment the
# daemon cannot ship. This is the whole setup, in one target.
voice-kokoro: ## Install the neural (Kokoro) speech engine — Apple Silicon
	@test "$$(uname -m)" = "arm64" || { echo "Kokoro via MLX needs Apple Silicon"; exit 1; }
	command -v uv >/dev/null || { echo "needs uv: brew install uv"; exit 1; }
	test -d "$(HOME)/.vibecli/tts" || uv venv --python 3.12 "$(HOME)/.vibecli/tts"
	VIRTUAL_ENV="$(HOME)/.vibecli/tts" uv pip install -q mlx-audio "misaki[en]"
	# misaki downloads this spaCy model on *first use* by shelling out to `uv`,
	# and the daemon spawns the sidecar with no VIRTUAL_ENV — so that install
	# fails with "No virtual environment found" and takes the sidecar with it.
	# Fetch it now, when there is someone watching, not mid-conversation.
	VIRTUAL_ENV="$(HOME)/.vibecli/tts" uv pip install -q \
	  "en_core_web_sm @ https://github.com/explosion/spacy-models/releases/download/en_core_web_sm-3.8.0/en_core_web_sm-3.8.0-py3-none-any.whl"
	@env -u VIRTUAL_ENV "$(HOME)/.vibecli/tts/bin/python" -c \
	  "import spacy; spacy.load('en_core_web_sm')" \
	  || { echo "spaCy model missing — the sidecar would die on its first English sentence"; exit 1; }
	@mkdir -p "$(HOME)/.vibecli/sidecars"
	install -m 0755 tools/voice-duplex/sidecar/tts_kokoro.py "$(HOME)/.vibecli/sidecars/tts_kokoro.py"
	@"$(HOME)/.vibecli/sidecars/tts_kokoro.py" --selftest >/dev/null 2>&1 	  || { echo "sidecar selftest failed — not writing config"; exit 1; }
	./scripts/voice-config.sh kokoro
	@echo
	@echo "Restart the daemon for this to take effect."

voice-status: ## Which speech engine the daemon will actually use
	./scripts/voice-config.sh --status

# ── macOS code signing ────────────────────────────────────────────────────────
# `tauri build` signs the .app bundles when APPLE_SIGNING_IDENTITY is exported,
# but nothing signs the standalone binaries that ship in the tarballs. This
# target covers both and *verifies* every signature — a build that reports
# "signed" without checking is how an ad-hoc artifact reaches a release.
#
# The build targets above now export the identity themselves via
# scripts/tauri-build.sh, so a local `make build-vibedesk` is signed rather than
# ad-hoc. This stays because the standalone binaries still need it, and because
# re-signing an already-built tree is the faster loop.

codesign-macos: ## Sign + verify macOS release artifacts with a Developer ID cert
	./scripts/codesign-macos.sh

codesign-verify: ## Verify macOS artifact signatures without changing them
	./scripts/codesign-macos.sh --verify-only

# Signing is half the job. A Developer ID signature with no notarization is
# still refused on any machine that did not build it — `spctl` calls it
# "source=Unnotarized Developer ID" — so a build that stops after signing
# produces artifacts that work for exactly one person.
notarize-macos: ## Notarize + staple the built macOS artifacts (needs a notarytool keychain profile)
	./scripts/notarize-macos.sh

notarize-verify: ## Ask Gatekeeper whether the artifacts would actually open
	./scripts/notarize-macos.sh --verify-only


# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: Agent SDK — TypeScript (packages/agent-sdk)
# ══════════════════════════════════════════════════════════════════════════════

build-sdk: $(SDK_DIR)/node_modules ## Build Agent SDK (tsup → cjs/esm/dts)
	cd $(SDK_DIR) && $(NPM) run build

test-sdk: $(SDK_DIR)/node_modules ## Test Agent SDK (vitest)
	cd $(SDK_DIR) && $(NPM) test

lint-sdk: $(SDK_DIR)/node_modules ## Type-check Agent SDK (tsc --noEmit)
	cd $(SDK_DIR) && $(NPM) run lint

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: Rust services — vibe-indexer, vibe-memory
# ══════════════════════════════════════════════════════════════════════════════

build-indexer: ## Build vibe-indexer (release)
	$(CARGO) build --release -p vibe-indexer

test-indexer: ## Test vibe-indexer
	$(CARGO) test -p vibe-indexer

build-memory: ## Build vibe-memory (release)
	$(CARGO) build --release -p vibe-memory

test-memory: ## Test vibe-memory
	$(CARGO) test -p vibe-memory

test-ai: ## Test vibe-ai crate
	$(CARGO) test -p vibe-ai

test-core: ## Test vibe-core crate
	$(CARGO) test -p vibe-core

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: vibe-rl-py — Python RL sidecar (uv)
# ══════════════════════════════════════════════════════════════════════════════

build-rl: ## Build the vibe-rl Python wheel (uv build)
	@command -v $(UV) >/dev/null || (echo "✗ uv not found — https://docs.astral.sh/uv/" && exit 1)
	cd $(RL_DIR) && $(UV) build

test-rl: ## Test vibe-rl-py (uv run pytest)
	@command -v $(UV) >/dev/null || (echo "✗ uv not found — https://docs.astral.sh/uv/" && exit 1)
	cd $(RL_DIR) && $(UV) run --extra dev pytest

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: Editor plugins — VS Code, JetBrains, Neovim
# ══════════════════════════════════════════════════════════════════════════════

build-vscode: $(VSCODE_DIR)/node_modules ## Compile the VS Code extension (tsc -p .)
	cd $(VSCODE_DIR) && $(NPM) run compile

lint-vscode: $(VSCODE_DIR)/node_modules ## Lint the VS Code extension (eslint)
	cd $(VSCODE_DIR) && $(NPM) run lint

build-jetbrains: ## Build the JetBrains plugin (gradle buildPlugin)
	@[ -x $(JETBRAINS_DIR)/gradlew ] || (echo "✗ gradlew missing" && exit 1)
	cd $(JETBRAINS_DIR) && $(GRADLE_JETBRAINS) buildPlugin

test-jetbrains: ## Test the JetBrains plugin (gradle test)
	@[ -x $(JETBRAINS_DIR)/gradlew ] || (echo "✗ gradlew missing" && exit 1)
	cd $(JETBRAINS_DIR) && $(GRADLE_JETBRAINS) test

check-neovim: ## Lint the Neovim plugin (luacheck, if installed)
	@if command -v luacheck >/dev/null; then \
		cd neovim-plugin && luacheck lua; \
	else \
		echo "luacheck not installed — skipping (brew install luacheck)"; \
	fi

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: Mobile — Flutter (iPhone + Android phone)
# ══════════════════════════════════════════════════════════════════════════════

mobile-setup: ## Install Flutter deps + CocoaPods for vibemobile
	@command -v $(FLUTTER) >/dev/null || (echo "✗ Flutter not found — install from https://docs.flutter.dev/get-started/install" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) pub get
ifeq ($(shell uname -s),Darwin)
	cd $(MOBILE_DIR)/ios && pod install
endif

mobile-ios: mobile-setup ## Build vibemobile iOS .app (release, unsigned) → vibemobile/build/ios
	@[ "$$(uname -s)" = "Darwin" ] || (echo "✗ iOS builds require macOS" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) build ios --release --no-codesign
	@echo "✓ iOS .aichat: $(MOBILE_DIR)/build/ios/iphoneos/Runner.app"

mobile-ios-ipa: ## Build signed .ipa for iPhone (delegates to vibemobile/Makefile)
	$(MAKE) -C $(MOBILE_DIR) ios-ipa

mobile-android: ## Build vibemobile Android APK (release) → vibemobile/build/app/outputs/flutter-apk
	@command -v $(FLUTTER) >/dev/null || (echo "✗ Flutter not found" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) pub get && $(FLUTTER) build apk --release
	@echo "✓ APK: $(MOBILE_DIR)/build/app/outputs/flutter-apk/app-release.apk"

mobile-android-bundle: ## Build Android App Bundle (.aab) for Play Store
	@command -v $(FLUTTER) >/dev/null || (echo "✗ Flutter not found" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) pub get && $(FLUTTER) build appbundle --release
	@echo "✓ AAB: $(MOBILE_DIR)/build/app/outputs/bundle/release/app-release.aab"

test-mobile: ## Test vibemobile (flutter test)
	@command -v $(FLUTTER) >/dev/null || (echo "✗ Flutter not found" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) pub get && $(FLUTTER) test

analyze-mobile: ## Static-analyze vibemobile (dart analyze --fatal-infos)
	@command -v $(FLUTTER) >/dev/null || (echo "✗ Flutter not found" && exit 1)
	cd $(MOBILE_DIR) && $(FLUTTER) pub get && dart analyze --fatal-infos

mobile-clean: ## Clean Flutter mobile build artifacts
	cd $(MOBILE_DIR) && $(FLUTTER) clean

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: Watch — watchOS (Xcode) + Wear OS (Gradle)
# ══════════════════════════════════════════════════════════════════════════════

watch-ios: ## Build watchOS app (release, simulator) — vibewatch/VibeCodyWatch
	@[ "$$(uname -s)" = "Darwin" ] || (echo "✗ watchOS builds require macOS" && exit 1)
	@command -v $(XCODEBUILD) >/dev/null || (echo "✗ xcodebuild not found — install Xcode" && exit 1)
	cd $(WATCH_IOS_DIR) && $(XCODEBUILD) \
	  -project $(WATCH_IOS_PROJ) \
	  -scheme $(WATCH_IOS_SCHEME) \
	  -configuration Release \
	  -destination 'generic/platform=watchOS Simulator' \
	  CODE_SIGNING_ALLOWED=NO \
	  build
	@echo "✓ watchOS app built"

watch-ios-archive: ## Archive watchOS app for distribution (requires signing)
	@[ "$$(uname -s)" = "Darwin" ] || (echo "✗ watchOS builds require macOS" && exit 1)
	cd $(WATCH_IOS_DIR) && $(XCODEBUILD) archive \
	  -project $(WATCH_IOS_PROJ) \
	  -scheme $(WATCH_IOS_SCHEME) \
	  -configuration Release \
	  -destination 'generic/platform=watchOS' \
	  -archivePath build/VibeCodyWatch.xcarchive
	@echo "✓ Archive: $(WATCH_IOS_DIR)/build/VibeCodyWatch.xcarchive"

watch-wear: ## Build Wear OS APK (release) — vibewatch/VibeCodyWear
	@[ -x $(WATCH_WEAR_DIR)/gradlew ] || (echo "✗ gradlew missing — run setup" && exit 1)
	cd $(WATCH_WEAR_DIR) && $(GRADLE_WEAR) :app:assembleRelease
	@echo "✓ Wear OS APK: $(WATCH_WEAR_DIR)/app/build/outputs/apk/release/app-release-unsigned.apk"

watch-wear-bundle: ## Build Wear OS App Bundle (.aab)
	@[ -x $(WATCH_WEAR_DIR)/gradlew ] || (echo "✗ gradlew missing" && exit 1)
	cd $(WATCH_WEAR_DIR) && $(GRADLE_WEAR) :app:bundleRelease
	@echo "✓ Wear OS AAB: $(WATCH_WEAR_DIR)/app/build/outputs/bundle/release/app-release.aab"

test-watch: ## Test Wear OS unit tests (gradle test); watchOS tests need Xcode schemes
	@[ -x $(WATCH_WEAR_DIR)/gradlew ] || (echo "✗ gradlew missing — run setup" && exit 1)
	cd $(WATCH_WEAR_DIR) && $(GRADLE_WEAR) test
	@echo "✓ Wear OS unit tests passed (watchOS: run 'xcodebuild test' in $(WATCH_IOS_DIR) on macOS)"

watch-clean: ## Clean watchOS + Wear OS build artifacts
	-cd $(WATCH_WEAR_DIR) && $(GRADLE_WEAR) clean
	-rm -rf $(WATCH_IOS_DIR)/build

# ── Aggregate mobile + watch builds ───────────────────────────────────────────

build-mobile: mobile-android ## Build mobile binaries (Android always; iOS only on macOS)
ifeq ($(shell uname -s),Darwin)
	$(MAKE) mobile-ios
endif

build-watch: watch-wear ## Build watch binaries (Wear OS always; watchOS only on macOS)
ifeq ($(shell uname -s),Darwin)
	$(MAKE) watch-ios
endif

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Building
# ══════════════════════════════════════════════════════════════════════════════

build: build-cli build-ui build-aichat build-vibedesk ## Build all desktop shells (CLI + UI + App + VibeDesk)

build-all: build build-mobile build-watch ## Build everything: desktop + mobile + watch

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Testing
# ══════════════════════════════════════════════════════════════════════════════

test: ## Run all Rust workspace tests (fast path)
	$(CARGO) test --workspace

test-rust: test ## (alias) Run all Rust workspace tests

test-fast: ## Run Rust tests excluding the collab crate (faster)
	$(CARGO) test --workspace --exclude vibe-collab

test-all: test test-ui test-aichat test-vibedesk test-sdk ## Test every Node + Rust surface (mobile/rl run separately)
	@echo ""
	@echo "✓ Rust + Node surfaces tested. For platform-gated suites run: make test-mobile test-rl test-jetbrains"

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Evaluations
# ══════════════════════════════════════════════════════════════════════════════
#
# `eval-check` validates the suite files themselves and is safe in CI: it runs
# no agent and calls no provider. Everything below it costs tokens.

.PHONY: eval-check eval-list eval-offline eval-surfaces eval-models eval-full eval-gate

eval-check: ## Validate the eval suites (no agent, no provider, CI-safe)
	$(CARGO) test -p vibe-eval

eval-list: ## Show what a full eval run would execute
	$(CARGO) run --release -p vibecli -- --eval list

eval-offline: ## Run the zero-dependency capability suites (needs a provider)
	$(CARGO) run --release -p vibecli -- --eval run --tag offline \
		--binary target/release/vibecli$(EXE)

eval-surfaces: ## Run surface conformance only (static checks + live daemon probes)
	$(CARGO) run --release -p vibecli -- --eval run --suite surfaces

# Per-model tool-protocol conformance. Pass the model under test:
#   make eval-models MODEL=gpt-oss:20b PROVIDER=ollama
# A model that fails this is unreachable through the tool loop, which is a
# different problem from being bad at coding — and a different fix.
eval-models: ## Run model conformance for one model (MODEL=… PROVIDER=…)
	$(CARGO) run --release -p vibecli -- --eval run --suite models \
		--provider $(or $(PROVIDER),ollama) --model $(MODEL) \
		--binary target/release/vibecli$(EXE)

eval-full: ## Run every suite (slow, costs tokens)
	$(CARGO) run --release -p vibecli -- --eval run \
		--binary target/release/vibecli$(EXE)

eval-gate: ## Compare the latest run against BASELINE and fail on regression
	@test -n "$(BASELINE)" || { \
		echo "Set BASELINE to a run id: make eval-gate BASELINE=run-1754000000"; \
		echo "List them with: vibecli --eval runs"; exit 2; }
	$(CARGO) run --release -p vibecli -- --eval gate latest --baseline $(BASELINE)

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Quality (type-check, lint, format)
# ══════════════════════════════════════════════════════════════════════════════

check: ## Fast type-check (Rust workspace + UI/App/VibeDesk TypeScript)
	$(CARGO) check --workspace --exclude vibe-collab
	$(MAKE) check-ui check-aichat check-vibedesk

lint: ## Run clippy + UI TypeScript check
	$(CARGO) clippy --workspace --exclude vibe-collab -- -D warnings
	$(MAKE) check-ui

fmt: ## Format all Rust code
	$(CARGO) fmt --all

fmt-check: ## Check Rust formatting without modifying
	$(CARGO) fmt --all -- --check

# App icons are generated from assets/brand/ and committed, so a normal build
# needs neither this target nor librsvg. Run it after editing the brand mark.
icons: ## Regenerate every app icon from the shared brand mark
	python3 scripts/brand/gen_icons.py

icons-check: ## Fail if any committed icon is out of date with the brand mark
	python3 scripts/brand/gen_icons.py --check

# Mirror the GitHub CI gate (.github/workflows/ci.yml) locally.
ci: fmt-check ## Run the same checks CI does (Rust + VibeCoder + VibeAIChat + SDK + Mobile + Wear + JetBrains)
	@echo "── Rust: clippy + test ──────────────────────────────"
	$(CARGO) clippy --workspace
	$(CARGO) test --workspace --exclude vibe-memory --exclude vibe-broker
	@echo "── VibeCoder: lint + typecheck + test ──────────────────"
	$(MAKE) lint-ui check-ui test-ui
	@echo "── VibeAIChat: typecheck ───────────────────────────────"
	$(MAKE) check-aichat
	@echo "── Agent SDK: lint + test ───────────────────────────"
	$(MAKE) lint-sdk test-sdk
	@echo "── Mobile: analyze + test ───────────────────────────"
	@if command -v $(FLUTTER) >/dev/null; then \
		$(MAKE) analyze-mobile test-mobile; \
	else \
		echo "Flutter not installed — skipping mobile checks (CI runs them)"; \
	fi
	@echo "── Wear OS: release build ───────────────────────────"
	@if [ -n "$$ANDROID_HOME$$ANDROID_SDK_ROOT" ]; then \
		$(MAKE) watch-wear; \
	else \
		echo "Android SDK not found — skipping Wear OS build (CI runs it)"; \
	fi
	@echo "── JetBrains plugin: build + test ───────────────────"
	@if command -v java >/dev/null; then \
		$(MAKE) build-jetbrains test-jetbrains; \
	else \
		echo "JDK not found — skipping JetBrains plugin (CI runs it)"; \
	fi
	@echo ""
	@echo "✓ Local CI gate passed."

# ══════════════════════════════════════════════════════════════════════════════
# Cleanup
# ══════════════════════════════════════════════════════════════════════════════

clean: mobile-clean watch-clean ## Remove build artifacts (Rust + UI + App + VibeDesk + mobile + watch)
	$(CARGO) clean
	rm -rf vibecoder/dist vibecoder/node_modules/.vite
	rm -rf vibeaichat/dist vibeaichat/node_modules/.vite
	rm -rf vibedesk/dist vibedesk/node_modules/.vite
	rm -rf $(SDK_DIR)/dist

# ── Docker ─────────────────────────────────────────────────────────────────────

docker: ## Build Docker image (VibeCLI static binary)
	docker build -t vibecli:latest .

docker-run: ## Run VibeCLI in Docker with Ollama sidecar
	docker compose up -d

# ── Sandbox tiers (rootfs builder for Firecracker Tier-3) ─────────────────────

.PHONY: rootfs-firecracker rootfs-firecracker-clean

rootfs-firecracker: ## Build the Firecracker Tier-3 rootfs (BusyBox + bash, ≤20 MiB)
	@bash scripts/build-rootfs-firecracker.sh

rootfs-firecracker-clean: ## Remove built Firecracker rootfs image
	rm -rf target/firecracker-rootfs

.PHONY: sandbox-doctor sandbox-doctor-json

sandbox-doctor: ## Probe the host for sandbox-tier availability (Native/WASI/Hyperlight/Firecracker)
	@bash scripts/check-sandbox-tiers.sh

sandbox-doctor-json: ## Same as sandbox-doctor, but emit JSON for tooling
	@bash scripts/check-sandbox-tiers.sh --json
