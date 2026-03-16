# VibeCody — Developer Makefile
#
# Usage:
#   make setup    — Install all prerequisites (Rust, Node, system libs)
#   make ui       — Run VibeUI in dev mode
#   make cli      — Build VibeCLI release binary
#   make test     — Run all tests
#   make check    — Fast type-check (Rust + TypeScript)
#   make help     — Show all targets

.PHONY: help setup ui cli app test check lint clean build doctor

# Ensure ~/.cargo/bin is in PATH (fixes npm rustup shadowing on Linux)
export PATH := $(HOME)/.cargo/bin:$(PATH)

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
	@echo ""
	@echo "Required: Rust, Cargo, Node.js, npm, Git"
	@echo "Optional: Ollama (local AI), Docker (container sandbox)"
ifeq ($(shell uname -s),Linux)
	@echo ""
	@echo "Linux — checking Tauri system dependencies..."
	@for dep in libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev; do \
		printf "  %-36s" "$$dep:" && (dpkg -s $$dep 2>/dev/null | grep -q "ok installed" && echo "OK" || echo "MISSING — run: make setup"); \
	done
endif

# ── Development ────────────────────────────────────────────────────────────────

ui: ## Run VibeUI in dev mode (Vite + Tauri)
	cd vibeui && npm run tauri:dev

app: ## Run VibeCLI App in dev mode
	cd vibeapp && npm run tauri dev

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

# ── Cleanup ────────────────────────────────────────────────────────────────────

clean: ## Remove build artifacts
	cargo clean
	rm -rf vibeui/dist vibeui/node_modules/.vite
	rm -rf vibeapp/dist vibeapp/node_modules/.vite

# ── Docker ─────────────────────────────────────────────────────────────────────

docker: ## Build Docker image (VibeCLI static binary)
	docker build -t vibecli:latest .

docker-run: ## Run VibeCLI in Docker with Ollama sidecar
	docker compose up -d
