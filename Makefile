# VibeCody — Developer Makefile
#
# Usage:
#   make setup           — Install all prerequisites (Rust, Node, system libs)
#   make ui              — Run VibeUI in dev mode
#   make cli             — Build VibeCLI release binary
#   make build-mobile    — Build vibemobile (iPhone .app + Android APK)
#   make build-watch     — Build vibewatch (watchOS + Wear OS)
#   make build-all       — Build desktop + mobile + watch
#   make test            — Run all tests
#   make check           — Fast type-check (Rust + TypeScript)
#   make help            — Show all targets

.PHONY: help setup ui cli app test check lint clean build doctor \
        mobile-setup mobile-ios mobile-ios-ipa mobile-android mobile-android-bundle \
        mobile-clean watch-ios watch-ios-archive watch-wear watch-wear-bundle \
        watch-clean build-mobile build-watch build-all

# Ensure ~/.cargo/bin is in PATH (fixes npm rustup shadowing on Linux)
export PATH := $(HOME)/.cargo/bin:$(PATH)

# ── Mobile / Watch toolchain locations ────────────────────────────────────────
FLUTTER          ?= flutter
XCODEBUILD       ?= xcodebuild
GRADLE_WEAR      := ./gradlew
MOBILE_DIR       := vibemobile
WATCH_IOS_DIR    := vibewatch
WATCH_IOS_PROJ   := VibeCodyWatch.xcodeproj
WATCH_IOS_SCHEME := VibeCodyWatch
WATCH_WEAR_DIR   := vibewatch/VibeCodyWear

help: ## Show available targets
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*## "}; {printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2}'

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
	@echo "Optional: Ollama (local AI), Docker (container sandbox), JDK 17/21 (watch-wear), Flutter (mobile-*)"
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

# ── Development ────────────────────────────────────────────────────────────────

ui: ## Run VibeUI in dev mode (Vite + Tauri)
	cd vibeui && npm run tauri:dev

app: ## Run VibeCLI App in dev mode
	cd vibeapp && npm run tauri:dev

cli: ## Build VibeCLI release binary → target/release/vibecli
	cargo build --release -p vibecli
	@echo ""
	@ls -lh target/release/vibecli 2>/dev/null || ls -lh target/release/vibecli.exe 2>/dev/null
	@echo ""
	@echo "Binary: target/release/vibecli"

cli-run: ## Build and run VibeCLI with TUI
	cargo run --release -p vibecli -- --tui

# ── Testing ────────────────────────────────────────────────────────────────────

test: ## Run all workspace tests
	cargo test --workspace

test-fast: ## Run tests excluding collab crate (faster)
	cargo test --workspace --exclude vibe-collab

test-cli: ## Run VibeCLI tests only
	cargo test -p vibecli

test-ai: ## Run vibe-ai tests only
	cargo test -p vibe-ai

test-core: ## Run vibe-core tests only
	cargo test -p vibe-core

# ── Quality ────────────────────────────────────────────────────────────────────

check: ## Fast type-check (Rust + TypeScript, no codegen)
	cargo check --workspace --exclude vibe-collab
	cd vibeui && npx tsc --noEmit

lint: ## Run clippy + TypeScript check
	cargo clippy --workspace --exclude vibe-collab -- -D warnings
	cd vibeui && npx tsc --noEmit

fmt: ## Format all Rust code
	cargo fmt --all

fmt-check: ## Check Rust formatting without modifying
	cargo fmt --all -- --check

# ── Building ───────────────────────────────────────────────────────────────────

build: ## Build everything (CLI + VibeUI + VibeCLI App)
	cargo build --release -p vibecli
	cd vibeui && npm run tauri:build
	cd vibeapp && npm run tauri:build

build-ui: ## Build VibeUI for production
	cd vibeui && npm run tauri:build

build-app: ## Build VibeCLI App for production
	cd vibeapp && npm run tauri:build

# ── Mobile (Flutter — iPhone + Android phone) ─────────────────────────────────

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

mobile-clean: ## Clean Flutter mobile build artifacts
	cd $(MOBILE_DIR) && $(FLUTTER) clean

# ── Watch — watchOS (Xcode) + Wear OS (Gradle) ────────────────────────────────

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

build-all: build build-mobile build-watch ## Build everything: desktop + mobile + watch

# ── Cleanup ────────────────────────────────────────────────────────────────────

clean: mobile-clean watch-clean ## Remove build artifacts (Rust + UI + mobile + watch)
	cargo clean
	rm -rf vibeui/dist vibeui/node_modules/.vite
	rm -rf vibeapp/dist vibeapp/node_modules/.vite

# ── Docker ─────────────────────────────────────────────────────────────────────

docker: ## Build Docker image (VibeCLI static binary)
	docker build -t vibecli:latest .

docker-run: ## Run VibeCLI in Docker with Ollama sidecar
	docker compose up -d
