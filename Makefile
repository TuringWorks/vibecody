# VibeCody — Developer Makefile
#
# Every surface gets a consistent  build-<surface>  and  test-<surface>  target.
# Run `make help` for the full list, or `make help-surfaces` for the matrix.
#
#   Surface            Dev            Build               Test
#   ─────────────────  ─────────────  ──────────────────  ──────────────────
#   VibeCLI (Rust)     make cli-run   make build-cli      make test-cli
#   VibeCoder  (Tauri)    make ui        make build-ui       make test-ui
#   VibeApp (Tauri)    make app       make build-app      make test-app
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
        ui app vibedesk cli cli-run \
        build build-apps build-cli build-ui build-app build-vibedesk \
        build-sdk build-indexer build-memory build-rl build-vscode build-jetbrains \
        test test-fast test-all test-rust \
        test-cli test-ai test-core test-indexer test-memory \
        test-ui test-app test-vibedesk test-sdk test-mobile test-rl test-jetbrains test-watch \
        check check-cli check-ui check-app check-vibedesk \
        lint lint-ui lint-sdk lint-vscode lint-vibedesk check-neovim \
        fmt fmt-check ci analyze-mobile \
        mobile-setup mobile-ios mobile-ios-ipa mobile-android mobile-android-bundle \
        mobile-clean watch-ios watch-ios-archive watch-wear watch-wear-bundle \
        watch-clean build-mobile build-watch \
        clean docker docker-run

# Ensure ~/.cargo/bin is in PATH (fixes npm rustup shadowing on Linux)
export PATH := $(HOME)/.cargo/bin:$(PATH)

# ── Toolchain locations ───────────────────────────────────────────────────────
CARGO            ?= cargo
NPM              ?= npm
UV               ?= uv
FLUTTER          ?= flutter
XCODEBUILD       ?= xcodebuild
GRADLE           ?= gradle
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
	@bash scripts/setup.sh

doctor: ## Verify development environment is ready
	@echo "Checking development environment..."
	@echo ""
	@printf "  %-20s" "Rust:" && (rustc --version 2>/dev/null || echo "MISSING — run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh")
	@printf "  %-20s" "Cargo:" && (cargo --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "Node.js:" && (node --version 2>/dev/null || echo "MISSING — install from https://nodejs.org/")
	@printf "  %-20s" "npm:" && (npm --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "Git:" && (git --version 2>/dev/null || echo "MISSING")
	@printf "  %-20s" "uv (vibe-rl-py):" && (uv --version 2>/dev/null || echo "not installed (needed for test-rl) — https://docs.astral.sh/uv/")
	@printf "  %-20s" "Ollama:" && (ollama --version 2>/dev/null || echo "not installed (optional)")
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

vibeapp/node_modules:
	cd vibeapp && $(NPM) install --no-audit --no-fund

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
	@ls -lh target/release/vibecli 2>/dev/null || ls -lh target/release/vibecli.exe 2>/dev/null
	@echo ""
	@echo "Binary: target/release/vibecli"

cli-run: ## Build and run VibeCLI with TUI
	$(CARGO) run --release -p vibecli -- --tui

test-cli: ## Test VibeCLI crate
	$(CARGO) test -p vibecli

check-cli: ## Fast type-check VibeCLI crate
	$(CARGO) check -p vibecli

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeCoder — desktop editor (Tauri 2 + React)
# ══════════════════════════════════════════════════════════════════════════════

ui: vibecoder/node_modules ## Run VibeCoder in dev mode (Vite + Tauri)
	cd vibecoder && $(NPM) run tauri:dev

build-ui: vibecoder/node_modules ## Build VibeCoder for production (Tauri bundle)
	cd vibecoder && $(NPM) run tauri:build

test-ui: vibecoder/node_modules ## Test VibeCoder (vitest)
	cd vibecoder && $(NPM) test

check-ui: vibecoder/node_modules ## Type-check VibeCoder (tsc --noEmit)
	cd vibecoder && npx tsc --noEmit

lint-ui: vibecoder/node_modules ## Lint VibeCoder (eslint)
	cd vibecoder && $(NPM) run lint

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeApp — secondary Tauri shell (vibeapp)
# ══════════════════════════════════════════════════════════════════════════════

app: vibeapp/node_modules ## Run VibeApp in dev mode
	cd vibeapp && $(NPM) run tauri:dev

build-app: vibeapp/node_modules ## Build VibeApp for production (Tauri bundle)
	cd vibeapp && $(NPM) run tauri:build

test-app: check-app ## Test VibeApp (typecheck only — no unit suite yet)

check-app: vibeapp/node_modules ## Type-check VibeApp (tsc --noEmit)
	cd vibeapp && npx tsc --noEmit

# ══════════════════════════════════════════════════════════════════════════════
# SURFACE: VibeDesk — Tauri shell (vibedesk)
# ══════════════════════════════════════════════════════════════════════════════

vibedesk: vibedesk/node_modules ## Run VibeDesk in dev mode
	cd vibedesk && $(NPM) run tauri:dev

build-vibedesk: vibedesk/node_modules ## Build VibeDesk for production (Tauri bundle)
	cd vibedesk && $(NPM) run tauri:build

test-vibedesk: check-vibedesk lint-vibedesk ## Test VibeDesk (typecheck + no-inline-edit guard)

check-vibedesk: vibedesk/node_modules ## Type-check VibeDesk (tsc --noEmit)
	cd vibedesk && npx tsc --noEmit

lint-vibedesk: vibedesk/node_modules ## Run VibeDesk patent-distance guard (no-inline-edit)
	cd vibedesk && $(NPM) run lint:no-inline-edit

# ── Desktop apps aggregate (the three Tauri shells) ───────────────────────────

build-apps: build-ui build-app build-vibedesk ## Build all three Tauri shells (ui + app + vibedesk)

test-apps: test-ui test-app test-vibedesk ## Test all three Tauri shells

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
	@command -v $(GRADLE) >/dev/null || (echo "✗ gradle not found — install: brew install gradle" && exit 1)
	cd $(JETBRAINS_DIR) && $(GRADLE) buildPlugin

test-jetbrains: ## Test the JetBrains plugin (gradle test)
	@command -v $(GRADLE) >/dev/null || (echo "✗ gradle not found — install: brew install gradle" && exit 1)
	cd $(JETBRAINS_DIR) && $(GRADLE) test

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
	@echo "✓ iOS .app: $(MOBILE_DIR)/build/ios/iphoneos/Runner.app"

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
	@echo "✓ Wear OS APK: $(WATCH_WEAR_DIR)/app/build/outputs/apk/release/app-release.apk"

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

build: build-cli build-ui build-app build-vibedesk ## Build all desktop shells (CLI + UI + App + VibeDesk)

build-all: build build-mobile build-watch ## Build everything: desktop + mobile + watch

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Testing
# ══════════════════════════════════════════════════════════════════════════════

test: ## Run all Rust workspace tests (fast path)
	$(CARGO) test --workspace

test-rust: test ## (alias) Run all Rust workspace tests

test-fast: ## Run Rust tests excluding the collab crate (faster)
	$(CARGO) test --workspace --exclude vibe-collab

test-all: test test-ui test-app test-vibedesk test-sdk ## Test every Node + Rust surface (mobile/rl run separately)
	@echo ""
	@echo "✓ Rust + Node surfaces tested. For platform-gated suites run: make test-mobile test-rl test-jetbrains"

# ══════════════════════════════════════════════════════════════════════════════
# AGGREGATE: Quality (type-check, lint, format)
# ══════════════════════════════════════════════════════════════════════════════

check: ## Fast type-check (Rust workspace + UI/App/VibeDesk TypeScript)
	$(CARGO) check --workspace --exclude vibe-collab
	$(MAKE) check-ui check-app check-vibedesk

lint: ## Run clippy + UI TypeScript check
	$(CARGO) clippy --workspace --exclude vibe-collab -- -D warnings
	$(MAKE) check-ui

fmt: ## Format all Rust code
	$(CARGO) fmt --all

fmt-check: ## Check Rust formatting without modifying
	$(CARGO) fmt --all -- --check

# Mirror the GitHub CI gate (.github/workflows/ci.yml) locally.
ci: fmt-check ## Run the same checks CI does (Rust + VibeCoder + VibeApp + SDK + Mobile)
	@echo "── Rust: clippy + test ──────────────────────────────"
	$(CARGO) clippy --workspace
	$(CARGO) test --workspace --exclude vibe-memory --exclude vibe-broker
	@echo "── VibeCoder: lint + typecheck + test ──────────────────"
	$(MAKE) lint-ui check-ui test-ui
	@echo "── VibeApp: typecheck ───────────────────────────────"
	$(MAKE) check-app
	@echo "── Agent SDK: lint + test ───────────────────────────"
	$(MAKE) lint-sdk test-sdk
	@echo "── Mobile: analyze + test ───────────────────────────"
	@if command -v $(FLUTTER) >/dev/null; then \
		$(MAKE) analyze-mobile test-mobile; \
	else \
		echo "Flutter not installed — skipping mobile checks (CI runs them)"; \
	fi
	@echo ""
	@echo "✓ Local CI gate passed."

# ══════════════════════════════════════════════════════════════════════════════
# Cleanup
# ══════════════════════════════════════════════════════════════════════════════

clean: mobile-clean watch-clean ## Remove build artifacts (Rust + UI + App + VibeDesk + mobile + watch)
	$(CARGO) clean
	rm -rf vibecoder/dist vibecoder/node_modules/.vite
	rm -rf vibeapp/dist vibeapp/node_modules/.vite
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
