# VibeCody Mobile — iOS sideload helpers
# ──────────────────────────────────────────────────────────────────────────────
# First-time setup:
#   1. cp ios/Signing.xcconfig.template ios/Signing.xcconfig
#   2. Edit ios/Signing.xcconfig — set your DEVELOPMENT_TEAM (10-char ID)
#      Find it at: https://developer.apple.com/account → Membership
#   3. make ios-setup
#   4. make ios-run          (cable to iPhone) or
#      make ios-ipa          (build .ipa for AltStore/Sideloadly)
# ──────────────────────────────────────────────────────────────────────────────

FLUTTER       := flutter
XCODEBUILD    := xcodebuild
WORKSPACE     := ios/Runner.xcworkspace
SCHEME        := Runner
BUILD_DIR     := build/ios
ARCHIVE_PATH  := $(BUILD_DIR)/Runner.xcarchive
EXPORT_DIR    := $(BUILD_DIR)/ipa
EXPORT_OPTS   := ios/ExportOptions.plist

# Read team ID from Signing.xcconfig if present
SIGNING_CFG   := ios/Signing.xcconfig
ifneq ($(wildcard $(SIGNING_CFG)),)
  DEVELOPMENT_TEAM := $(shell grep -E '^DEVELOPMENT_TEAM' $(SIGNING_CFG) | sed 's/.*= *//')
else
  DEVELOPMENT_TEAM ?= XXXXXXXXXX
endif

.PHONY: help ios-setup ios-run ios-run-debug ios-ipa ios-resign ios-devices ios-clean

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*##' $(MAKEFILE_LIST) | \
	  awk 'BEGIN{FS=":.*## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

# ── Setup ─────────────────────────────────────────────────────────────────────

ios-setup: ## Install Flutter + CocoaPods dependencies
	@echo "→ Installing Flutter dependencies..."
	$(FLUTTER) pub get
	@echo "→ Installing CocoaPods..."
	cd ios && pod install
	@echo ""
	@if [ ! -f $(SIGNING_CFG) ]; then \
	  echo "⚠  No ios/Signing.xcconfig found."; \
	  echo "   Run: cp ios/Signing.xcconfig.template ios/Signing.xcconfig"; \
	  echo "   Then fill in your DEVELOPMENT_TEAM (10-char Apple Team ID)."; \
	else \
	  echo "✓ Signing config found (DEVELOPMENT_TEAM=$(DEVELOPMENT_TEAM))"; \
	fi

# ── Run on device ─────────────────────────────────────────────────────────────

ios-devices: ## List connected iOS devices
	$(FLUTTER) devices

ios-run: ## Build release + install on connected iPhone (cable required)
	@[ "$(DEVELOPMENT_TEAM)" != "XXXXXXXXXX" ] || \
	  (echo "✗ Set DEVELOPMENT_TEAM in ios/Signing.xcconfig first" && exit 1)
	$(FLUTTER) run --release

ios-run-debug: ## Build debug + install on connected iPhone (faster, hot-reload)
	@[ "$(DEVELOPMENT_TEAM)" != "XXXXXXXXXX" ] || \
	  (echo "✗ Set DEVELOPMENT_TEAM in ios/Signing.xcconfig first" && exit 1)
	$(FLUTTER) run

# ── IPA build ─────────────────────────────────────────────────────────────────

ios-ipa: ios-setup ## Build .ipa for AltStore / Sideloadly (no cable needed after)
	@[ "$(DEVELOPMENT_TEAM)" != "XXXXXXXXXX" ] || \
	  (echo "✗ Set DEVELOPMENT_TEAM in ios/Signing.xcconfig first" && exit 1)
	@echo "→ Building Flutter release..."
	$(FLUTTER) build ios --release --no-codesign
	@echo "→ Archiving with xcodebuild..."
	$(XCODEBUILD) archive \
	  -workspace $(WORKSPACE) \
	  -scheme $(SCHEME) \
	  -configuration Release \
	  -archivePath $(ARCHIVE_PATH) \
	  DEVELOPMENT_TEAM=$(DEVELOPMENT_TEAM) \
	  CODE_SIGN_STYLE=Automatic \
	  | xcpretty 2>/dev/null || cat
	@echo "→ Exporting IPA..."
	@sed 's/$${DEVELOPMENT_TEAM}/$(DEVELOPMENT_TEAM)/g' $(EXPORT_OPTS) \
	  > /tmp/ExportOptions_resolved.plist
	$(XCODEBUILD) -exportArchive \
	  -archivePath $(ARCHIVE_PATH) \
	  -exportPath $(EXPORT_DIR) \
	  -exportOptionsPlist /tmp/ExportOptions_resolved.plist \
	  | xcpretty 2>/dev/null || cat
	@echo ""
	@echo "✓ IPA ready: $(EXPORT_DIR)/vibecody_mobile.ipa"
	@echo "  → Drag into AltStore or Sideloadly to install"

# ── Re-sign ───────────────────────────────────────────────────────────────────

ios-resign: ## Re-sign the existing IPA (refreshes 7-day free-account expiry)
	@IPA=$$(ls -t $(EXPORT_DIR)/*.ipa 2>/dev/null | head -1); \
	[ -n "$$IPA" ] || (echo "✗ No IPA found — run make ios-ipa first" && exit 1); \
	echo "→ Re-signing $$IPA..."; \
	$(XCODEBUILD) -exportArchive \
	  -archivePath $(ARCHIVE_PATH) \
	  -exportPath $(EXPORT_DIR) \
	  -exportOptionsPlist /tmp/ExportOptions_resolved.plist \
	  | xcpretty 2>/dev/null || cat; \
	echo "✓ Re-signed: $$IPA"

# ── Clean ─────────────────────────────────────────────────────────────────────

ios-clean: ## Remove build artifacts
	rm -rf $(BUILD_DIR)
	$(FLUTTER) clean
