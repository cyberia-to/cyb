# Cyb - Cross-Platform Build System
# Usage: make help

.PHONY: all setup build clean help
.PHONY: setup-node setup-rust setup-java setup-android setup-ios setup-linux
.PHONY: web dev dev-tauri build-web build-tauri
.PHONY: wasm wasm-build wasm-copy
.PHONY: macos linux ios android
.PHONY: install-ios install-android
.PHONY: test lint icons
.PHONY: download-kubo
.PHONY: macos-release ios-release cyb-boot
.PHONY: setup-android-signing android-release

# ============================================================================
# Configuration
# ============================================================================

SHELL := /bin/bash
PROJECT_ROOT := $(shell pwd)
TAURI_DIR := $(PROJECT_ROOT)/src-tauri
UHASH_ROOT := $(realpath $(PROJECT_ROOT)/../universal-hash)
KUBO_VERSION ?= v0.34.1

# OS detection
UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  IS_MACOS := 1
  export JAVA_HOME ?= /opt/homebrew/opt/openjdk@17
  export ANDROID_HOME ?= $(HOME)/Library/Android/sdk
else
  IS_LINUX := 1
  export JAVA_HOME ?= /usr/lib/jvm/java-17-openjdk-amd64
  export ANDROID_HOME ?= $(HOME)/Android/Sdk
endif

export NDK_HOME ?= $(ANDROID_HOME)/ndk/26.1.10909125
export PATH := $(JAVA_HOME)/bin:$(ANDROID_HOME)/platform-tools:$(HOME)/.cargo/bin:$(PATH)

# GPU feature flags per platform
ifdef IS_MACOS
  CARGO_GPU_FEATURES := --features gpu-metal
endif
ifdef IS_LINUX
  CARGO_GPU_FEATURES := --features gpu-cuda,gpu-wgpu
endif

# Colors
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
RED := \033[0;31m
NC := \033[0m

# ============================================================================
# Default & Help
# ============================================================================

all: build ## Build all targets (web + Tauri)

help: ## Show this help
	@echo "Cyb Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Setup:"
	@grep -E '^setup[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "Development:"
	@grep -E '^(dev|dev-tauri|web):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "Build:"
	@grep -E '^(build-web|build-tauri|wasm|macos|macos-release|linux|ios|ios-release|android|android-release|build):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "Install:"
	@grep -E '^install[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "Assets:"
	@grep -E '^icons:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
	@echo ""
	@echo "Quality:"
	@grep -E '^(test|lint|clean):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'

# ============================================================================
# Setup Targets
# ============================================================================

setup: setup-node setup-rust ## Setup web + Tauri dev environment

setup-node: ## Install Node.js and Deno dependencies
	@echo -e "$(BLUE)[Setup]$(NC) Node.js & Deno..."
ifdef IS_MACOS
	@command -v node >/dev/null || (echo -e "$(RED)[Error]$(NC) Node.js not found. Install via: brew install node" && exit 1)
else
	@command -v node >/dev/null || (echo -e "$(YELLOW)[Setup]$(NC) Installing Node.js..." && \
		curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && \
		sudo apt-get install -y nodejs)
endif
	@command -v deno >/dev/null || (echo -e "$(RED)[Error]$(NC) Deno not found. Install via: brew install deno" && exit 1)
	@deno install
	@echo -e "$(GREEN)[Done]$(NC) Node.js ready ($$(node -v))"

setup-rust: ## Install Rust toolchain + Tauri CLI
	@echo -e "$(BLUE)[Setup]$(NC) Rust toolchain..."
	@command -v rustup >/dev/null || (curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y)
ifdef IS_MACOS
	@rustup target add aarch64-apple-ios 2>/dev/null || true
	@rustup target add aarch64-apple-darwin 2>/dev/null || true
	@rustup target add x86_64-apple-darwin 2>/dev/null || true
endif
	@rustup target add aarch64-linux-android 2>/dev/null || true
	@rustup target add wasm32-unknown-unknown 2>/dev/null || true
	@command -v wasm-bindgen >/dev/null || cargo install wasm-bindgen-cli
	@echo -e "$(GREEN)[Done]$(NC) Rust ready ($$(rustc --version 2>/dev/null | cut -d' ' -f2))"

setup-java: ## Install Java 17
	@echo -e "$(BLUE)[Setup]$(NC) Java..."
ifdef IS_MACOS
	@if [ ! -f "$(JAVA_HOME)/bin/java" ]; then \
		brew install openjdk@17 2>/dev/null || true; \
	fi
else
	@if ! command -v java >/dev/null || ! java -version 2>&1 | grep -q '17'; then \
		sudo apt-get update && sudo apt-get install -y openjdk-17-jdk; \
	fi
endif
	@echo -e "$(GREEN)[Done]$(NC) Java ready"

setup-android: setup-java ## Install Android SDK and NDK
	@echo -e "$(BLUE)[Setup]$(NC) Android SDK..."
	@mkdir -p $(ANDROID_HOME)
ifdef IS_MACOS
	@command -v sdkmanager >/dev/null || brew install --cask android-commandlinetools 2>/dev/null || true
	@SDKMGR=""; \
	if [ -f "/opt/homebrew/share/android-commandlinetools/cmdline-tools/latest/bin/sdkmanager" ]; then \
		SDKMGR="/opt/homebrew/share/android-commandlinetools/cmdline-tools/latest/bin/sdkmanager"; \
	elif [ -f "$(ANDROID_HOME)/cmdline-tools/latest/bin/sdkmanager" ]; then \
		SDKMGR="$(ANDROID_HOME)/cmdline-tools/latest/bin/sdkmanager"; \
	fi; \
	if [ -n "$$SDKMGR" ]; then \
		yes | JAVA_HOME=$(JAVA_HOME) $$SDKMGR --sdk_root=$(ANDROID_HOME) --licenses 2>/dev/null || true; \
		JAVA_HOME=$(JAVA_HOME) $$SDKMGR --sdk_root=$(ANDROID_HOME) \
			"platform-tools" "platforms;android-34" "build-tools;35.0.0" "ndk;26.1.10909125" 2>/dev/null || true; \
	fi
else
	@if [ ! -f "$(ANDROID_HOME)/cmdline-tools/latest/bin/sdkmanager" ]; then \
		echo -e "$(BLUE)[Setup]$(NC) Downloading Android command-line tools..."; \
		TMPZIP=$$(mktemp); \
		curl -fsSL "https://dl.google.com/android/repository/commandlinetools-linux-11076708_latest.zip" -o $$TMPZIP; \
		mkdir -p $(ANDROID_HOME)/cmdline-tools; \
		unzip -qo $$TMPZIP -d $(ANDROID_HOME)/cmdline-tools; \
		mv $(ANDROID_HOME)/cmdline-tools/cmdline-tools $(ANDROID_HOME)/cmdline-tools/latest 2>/dev/null || true; \
		rm -f $$TMPZIP; \
	fi
	@SDKMGR="$(ANDROID_HOME)/cmdline-tools/latest/bin/sdkmanager"; \
	if [ -f "$$SDKMGR" ]; then \
		yes | JAVA_HOME=$(JAVA_HOME) $$SDKMGR --sdk_root=$(ANDROID_HOME) --licenses 2>/dev/null || true; \
		JAVA_HOME=$(JAVA_HOME) $$SDKMGR --sdk_root=$(ANDROID_HOME) \
			"platform-tools" "platforms;android-34" "build-tools;35.0.0" "ndk;26.1.10909125" 2>/dev/null || true; \
	fi
endif
	@if [ ! -f "$(HOME)/.android/debug.keystore" ]; then \
		mkdir -p $(HOME)/.android; \
		keytool -genkey -v -keystore $(HOME)/.android/debug.keystore \
			-storepass android -alias androiddebugkey -keypass android \
			-keyalg RSA -keysize 2048 -validity 10000 \
			-dname "CN=Android Debug,O=Android,C=US" 2>/dev/null || true; \
	fi
	@echo -e "$(GREEN)[Done]$(NC) Android SDK ready"

setup-ios: ## Verify iOS build environment (macOS only, requires Xcode)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Setup]$(NC) iOS environment..."
	@command -v xcodebuild >/dev/null || (echo -e "$(RED)[Error]$(NC) Xcode not installed. Install from App Store." && exit 1)
	@echo -e "$(GREEN)[Done]$(NC) iOS ready"
else
	@echo -e "$(YELLOW)[Skip]$(NC) iOS builds require macOS with Xcode"
endif

setup-linux: ## Install Linux build dependencies (Ubuntu/Debian)
ifdef IS_LINUX
	@echo -e "$(BLUE)[Setup]$(NC) Linux dependencies..."
	@sudo apt-get update
	@sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
		build-essential curl wget file libssl-dev libayatana-appindicator3-dev
	@echo -e "$(GREEN)[Done]$(NC) Linux ready"
else
	@echo -e "$(YELLOW)[Skip]$(NC) Linux setup only needed on Linux"
endif

setup-all: setup setup-java setup-android setup-ios setup-linux ## Setup everything (all platforms)

# ============================================================================
# Development Targets
# ============================================================================

dev: setup-node ## Start web dev server (browser, port 3001)
	@echo -e "$(BLUE)[Dev]$(NC) Starting web dev server at https://localhost:3001"
	@deno task start

dev-tauri: setup-node setup-rust download-kubo ## Start Tauri dev server (native desktop)
	@echo -e "$(BLUE)[Dev]$(NC) Starting Tauri dev server..."
	@npx @tauri-apps/cli dev

web: dev ## Alias for dev

# ============================================================================
# WASM Targets
# ============================================================================

wasm: wasm-build wasm-copy ## Build uhash-web WASM and update npm package
	@echo -e "$(GREEN)[Done]$(NC) WASM updated in node_modules/uhash-web"

wasm-build: setup-rust ## Build uhash-web WASM from universal-hash workspace
	@echo -e "$(BLUE)[Build]$(NC) uhash-web WASM..."
	@if [ ! -d "$(UHASH_ROOT)" ]; then \
		echo -e "$(RED)[Error]$(NC) universal-hash not found at $(UHASH_ROOT)"; \
		echo "  Clone it: git clone https://github.com/cyberia-to/universal-hash ../universal-hash"; \
		exit 1; \
	fi
	@cd $(UHASH_ROOT) && cargo build -p uhash-web --release --target wasm32-unknown-unknown
	@wasm-bindgen $(UHASH_ROOT)/target/wasm32-unknown-unknown/release/uhash_web.wasm \
		--out-dir $(PROJECT_ROOT)/node_modules/uhash-web --target bundler
	@echo -e "$(GREEN)[Done]$(NC) WASM built"

wasm-copy: ## Copy WASM artifacts to node_modules/uhash-web
	@echo -e "$(BLUE)[Copy]$(NC) WASM to node_modules..."
	@mkdir -p $(PROJECT_ROOT)/node_modules/uhash-web
	@if [ -f "$(PROJECT_ROOT)/node_modules/uhash-web/uhash_web_bg.wasm" ]; then \
		echo -e "$(GREEN)[Done]$(NC) WASM files in place"; \
	else \
		echo -e "$(YELLOW)[Warn]$(NC) No WASM files found. Run 'make wasm-build' first."; \
	fi

# ============================================================================
# Kubo (IPFS) Sidecar
# ============================================================================

download-kubo: ## Download Kubo binary for current platform
	@echo -e "$(BLUE)[Kubo]$(NC) Downloading Kubo $(KUBO_VERSION)..."
	@$(TAURI_DIR)/scripts/download-kubo.sh $(KUBO_VERSION)
	@echo -e "$(GREEN)[Done]$(NC) Kubo binary ready"

# ============================================================================
# Build Targets
# ============================================================================

build: build-web ## Build web production bundle

build-web: setup-node ## Build web production bundle
	@echo -e "$(BLUE)[Build]$(NC) Web production..."
	@deno task build
	@echo -e "$(GREEN)[Done]$(NC) Web: $(PROJECT_ROOT)/build/"

build-tauri: setup-node setup-rust download-kubo ## Build Tauri production bundle (current platform)
	@echo -e "$(BLUE)[Build]$(NC) Tauri production..."
	@npx @tauri-apps/cli build
	@echo -e "$(GREEN)[Done]$(NC) Tauri: $(TAURI_DIR)/target/release/bundle/"

macos: setup-node setup-rust download-kubo ## Build macOS app (.dmg)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Build]$(NC) macOS (GPU: Metal)..."
	@npx @tauri-apps/cli build -- $(CARGO_GPU_FEATURES)
	@echo -e "$(GREEN)[Done]$(NC) macOS: $(TAURI_DIR)/target/release/bundle/dmg/"
else
	@echo -e "$(RED)[Error]$(NC) macOS builds require macOS"
endif

# Apple signing/notarization config (auto-detected from Keychain)
APPLE_SIGNING_IDENTITY ?= $(shell security find-identity -v -p codesigning 2>/dev/null | grep 'Developer ID Application' | head -1 | sed 's/.*"\(.*\)"/\1/')
PKG_SIGNING_IDENTITY ?= $(shell security find-identity -v -p basic 2>/dev/null | grep 'Developer ID Installer' | head -1 | sed 's/.*"\(.*\)"/\1/')
APPLE_API_KEY ?= $(shell ls ~/.appstoreconnect/private_keys/AuthKey_*.p8 2>/dev/null | head -1 | sed 's/.*AuthKey_\(.*\)\.p8/\1/')
APPLE_API_KEY_PATH ?= $(HOME)/.appstoreconnect/private_keys/AuthKey_$(APPLE_API_KEY).p8
APPLE_API_ISSUER ?= 7936a589-39b5-4a72-8ca0-417f7998b96c

macos-release: setup-node setup-rust download-kubo ## Build, sign, notarize macOS .app + .dmg + .pkg
ifdef IS_MACOS
	@if [ -z "$(APPLE_SIGNING_IDENTITY)" ]; then \
		echo -e "$(RED)[Error]$(NC) No Developer ID Application certificate found in Keychain"; \
		exit 1; \
	fi
	@if [ -z "$(APPLE_API_KEY)" ]; then \
		echo -e "$(RED)[Error]$(NC) No API key found in ~/.appstoreconnect/private_keys/"; \
		exit 1; \
	fi
	@if [ -z "$(APPLE_API_ISSUER)" ]; then \
		echo -e "$(RED)[Error]$(NC) APPLE_API_ISSUER not set. Get it from App Store Connect > Integrations"; \
		echo "  Usage: make macos-release APPLE_API_ISSUER=your-uuid"; \
		exit 1; \
	fi
	@echo -e "$(BLUE)[Build]$(NC) macOS release (sign + notarize)..."
	@echo -e "$(BLUE)[Build]$(NC) Identity: $(APPLE_SIGNING_IDENTITY)"
	@echo -e "$(BLUE)[Build]$(NC) API Key: $(APPLE_API_KEY)"
	@echo -e "$(BLUE)[Build]$(NC) Building web frontend with IS_TAURI..."
	@deno task build-tauri
	@APPLE_SIGNING_IDENTITY="$(APPLE_SIGNING_IDENTITY)" \
		APPLE_API_KEY="$(APPLE_API_KEY)" \
		APPLE_API_ISSUER="$(APPLE_API_ISSUER)" \
		APPLE_API_KEY_PATH="$(APPLE_API_KEY_PATH)" \
		cargo tauri build --target aarch64-apple-darwin --config '{"build":{"beforeBuildCommand":""}}' -- $(CARGO_GPU_FEATURES)
	@echo -e "$(GREEN)[Done]$(NC) Signed + notarized .app and .dmg"
	@echo -e "$(BLUE)[Build]$(NC) Building .pkg..."
	@PKG_SIGNING_IDENTITY="$(PKG_SIGNING_IDENTITY)" \
		APPLE_API_KEY="$(APPLE_API_KEY)" \
		APPLE_API_ISSUER="$(APPLE_API_ISSUER)" \
		APPLE_API_KEY_PATH="$(APPLE_API_KEY_PATH)" \
		$(TAURI_DIR)/scripts/macos-pkg.sh \
			--app-path "$(TAURI_DIR)/target/aarch64-apple-darwin/release/bundle/macos/cyb.app" \
			--output-dir "$(TAURI_DIR)/target/aarch64-apple-darwin/release/bundle/pkg"
	@echo -e "$(GREEN)[Done]$(NC) macOS release complete:"
	@echo -e "  .app: $(TAURI_DIR)/target/aarch64-apple-darwin/release/bundle/macos/cyb.app"
	@echo -e "  .dmg: $(TAURI_DIR)/target/aarch64-apple-darwin/release/bundle/dmg/"
	@echo -e "  .pkg: $(TAURI_DIR)/target/aarch64-apple-darwin/release/bundle/pkg/"
else
	@echo -e "$(RED)[Error]$(NC) macOS builds require macOS"
endif

cyb-boot: setup-rust ## Build, sign, notarize cyb-boot thin installer
ifdef IS_MACOS
	@if [ -z "$(APPLE_SIGNING_IDENTITY)" ]; then \
		echo -e "$(RED)[Error]$(NC) No Developer ID Application certificate found in Keychain"; \
		exit 1; \
	fi
	@if [ -z "$(APPLE_API_KEY)" ]; then \
		echo -e "$(RED)[Error]$(NC) No API key found in ~/.appstoreconnect/private_keys/"; \
		exit 1; \
	fi
	@echo -e "$(BLUE)[Build]$(NC) Building cyb-boot for macOS ARM..."
	@cd cyb-boot && cargo build --release --target aarch64-apple-darwin
	@echo -e "$(BLUE)[Sign]$(NC) Signing cyb-boot..."
	@codesign --force --options runtime --sign "$(APPLE_SIGNING_IDENTITY)" \
		cyb-boot/target/aarch64-apple-darwin/release/cyb-boot
	@echo -e "$(BLUE)[Notarize]$(NC) Submitting cyb-boot for notarization..."
	@ditto -c -k --keepParent cyb-boot/target/aarch64-apple-darwin/release/cyb-boot /tmp/cyb-boot-notarize.zip
	@xcrun notarytool submit /tmp/cyb-boot-notarize.zip \
		--key "$(APPLE_API_KEY_PATH)" \
		--key-id "$(APPLE_API_KEY)" \
		--issuer "$(APPLE_API_ISSUER)" \
		--wait
	@rm -f /tmp/cyb-boot-notarize.zip
	@echo -e "$(GREEN)[Done]$(NC) cyb-boot signed + notarized:"
	@echo -e "  cyb-boot/target/aarch64-apple-darwin/release/cyb-boot"
else
	@echo -e "$(RED)[Error]$(NC) macOS builds require macOS"
endif

# Apple Distribution signing config (for iOS App Store)
APPLE_DISTRIBUTION_IDENTITY ?= $(shell security find-identity -v -p codesigning 2>/dev/null | grep 'Apple Distribution' | head -1 | sed 's/.*"\(.*\)"/\1/')
APPLE_TEAM_ID ?= 38BRD9SJV7

ios-release: setup-node setup-rust setup-ios ## Build signed iOS .ipa for App Store
ifdef IS_MACOS
	@if [ -z "$(APPLE_DISTRIBUTION_IDENTITY)" ]; then \
		echo -e "$(RED)[Error]$(NC) No Apple Distribution certificate found in Keychain"; \
		echo -e "$(YELLOW)[Hint]$(NC) Install from Apple Developer portal or use Xcode automatic signing"; \
		exit 1; \
	fi
	@echo -e "$(BLUE)[Build]$(NC) iOS release (signed for App Store Connect)..."
	@echo -e "$(BLUE)[Build]$(NC) Identity: $(APPLE_DISTRIBUTION_IDENTITY)"
	@cd $(TAURI_DIR) && [ -d "gen/apple" ] || npx @tauri-apps/cli ios init
	@if [ -f "$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj" ]; then \
		grep -q 'export PATH=.*cargo' $(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj || \
		sed -i '' 's/shellScript = "cargo/shellScript = "export PATH=\\"$$HOME\/.cargo\/bin:$$PATH\\" \&\& cargo/g' \
			$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj; \
	fi
	@npx @tauri-apps/cli ios build --export-method app-store-connect -- --features gpu-metal
	@echo -e "$(GREEN)[Done]$(NC) iOS .ipa built (signed for App Store Connect)"
	@echo -e "  Upload with: xcrun altool --upload-app -f <ipa-path> --apiKey $(APPLE_API_KEY) --apiIssuer $(APPLE_API_ISSUER)"
else
	@echo -e "$(RED)[Error]$(NC) iOS builds require macOS with Xcode"
endif

linux: setup-node setup-rust setup-linux download-kubo ## Build Linux app (.deb, .AppImage)
ifdef IS_LINUX
	@echo -e "$(BLUE)[Build]$(NC) Linux (GPU: CUDA + WGPU)..."
	@npx @tauri-apps/cli build -- $(CARGO_GPU_FEATURES)
	@echo -e "$(GREEN)[Done]$(NC) Linux .deb: $(TAURI_DIR)/target/release/bundle/deb/"
	@echo -e "$(GREEN)[Done]$(NC) Linux .AppImage: $(TAURI_DIR)/target/release/bundle/appimage/"
else
	@echo -e "$(RED)[Error]$(NC) Linux builds require Linux"
endif

ios: setup-node setup-rust setup-ios ## Build iOS app (macOS only, no Kubo — uses gateway)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Build]$(NC) iOS..."
	@cd $(TAURI_DIR) && [ -d "gen/apple" ] || npx @tauri-apps/cli ios init
	@if [ -f "$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj" ]; then \
		grep -q 'export PATH=.*cargo' $(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj || \
		sed -i '' 's/shellScript = "cargo/shellScript = "export PATH=\\"$$HOME\/.cargo\/bin:$$PATH\\" \&\& cargo/g' \
			$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj; \
	fi
	@npx @tauri-apps/cli ios build -- --features gpu-metal
	@echo -e "$(GREEN)[Done]$(NC) iOS: $(TAURI_DIR)/gen/apple/build/"
else
	@echo -e "$(RED)[Error]$(NC) iOS builds require macOS with Xcode"
endif

android: setup-node setup-rust setup-android ## Build Android app (.apk, aarch64 only, no Kubo — uses gateway)
	@echo -e "$(BLUE)[Build]$(NC) Android..."
	@cd $(TAURI_DIR) && [ -d "gen/android" ] || \
		JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android init
	@echo "sdk.dir=$(ANDROID_HOME)" > $(TAURI_DIR)/gen/android/local.properties
	@JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android build --target aarch64 2>&1 | grep -v "WebSocket" || true
	@echo -e "$(GREEN)[Done]$(NC) Android APK: $(TAURI_DIR)/gen/android/app/build/outputs/apk/"

ANDROID_KEYSTORE ?= $(HOME)/.android/cyb-release.keystore
ANDROID_KEY_ALIAS ?= cyb-release

setup-android-signing: ## Generate Android release keystore + key.properties
	@if [ -f "$(ANDROID_KEYSTORE)" ]; then \
		echo -e "$(YELLOW)[Skip]$(NC) Keystore already exists: $(ANDROID_KEYSTORE)"; \
	else \
		echo -e "$(BLUE)[Setup]$(NC) Generating release keystore..."; \
		mkdir -p $(HOME)/.android; \
		keytool -genkeypair -v \
			-keystore "$(ANDROID_KEYSTORE)" \
			-alias "$(ANDROID_KEY_ALIAS)" \
			-keyalg RSA -keysize 2048 -validity 10000 \
			-storepass changeit -keypass changeit \
			-dname "CN=Cyb,O=Cyberia,C=US"; \
		echo -e "$(GREEN)[Done]$(NC) Keystore: $(ANDROID_KEYSTORE)"; \
		echo -e "$(YELLOW)[Important]$(NC) Change the default passwords! Edit key.properties after generation."; \
	fi
	@echo -e "$(BLUE)[Setup]$(NC) Writing key.properties..."
	@echo "storeFile=$(ANDROID_KEYSTORE)" > $(TAURI_DIR)/gen/android/key.properties
	@echo "storePassword=changeit" >> $(TAURI_DIR)/gen/android/key.properties
	@echo "keyAlias=$(ANDROID_KEY_ALIAS)" >> $(TAURI_DIR)/gen/android/key.properties
	@echo "keyPassword=changeit" >> $(TAURI_DIR)/gen/android/key.properties
	@echo -e "$(GREEN)[Done]$(NC) key.properties written to $(TAURI_DIR)/gen/android/key.properties"

android-release: setup-node setup-rust setup-android ## Build signed Android APK
	@if [ ! -f "$(TAURI_DIR)/gen/android/key.properties" ]; then \
		echo -e "$(RED)[Error]$(NC) key.properties not found. Run 'make setup-android-signing' first."; \
		exit 1; \
	fi
	@echo -e "$(BLUE)[Build]$(NC) Android release (signed APK)..."
	@cd $(TAURI_DIR) && [ -d "gen/android" ] || \
		JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android init
	@echo "sdk.dir=$(ANDROID_HOME)" > $(TAURI_DIR)/gen/android/local.properties
	@JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android build --target aarch64 --apk 2>&1 | grep -v "WebSocket" || true
	@echo -e "$(GREEN)[Done]$(NC) Signed APK: $(TAURI_DIR)/gen/android/app/build/outputs/apk/"

# ============================================================================
# Install Targets
# ============================================================================

install-ios: ## Install iOS app to connected device (macOS only)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Install]$(NC) iOS..."
	@IPA=$$(find $(TAURI_DIR)/gen/apple/build -name "*.ipa" 2>/dev/null | head -1); \
	if [ -f "$$IPA" ]; then \
		DEVICE=$$(xcrun devicectl list devices 2>/dev/null | grep 'available' | awk '{for(i=1;i<=NF;i++) if($$i ~ /^[0-9A-F][0-9A-F0-9-]*[0-9A-F]$$/ && length($$i)==36) print $$i}' | head -1); \
		if [ -n "$$DEVICE" ]; then \
			xcrun devicectl device install app --device "$$DEVICE" "$$IPA"; \
			echo -e "$(GREEN)[Done]$(NC) iOS app installed"; \
		else \
			echo -e "$(RED)[Error]$(NC) No iOS device connected"; \
		fi \
	else \
		echo -e "$(RED)[Error]$(NC) iOS build not found. Run 'make ios' or 'make ios-release' first."; \
	fi
else
	@echo -e "$(RED)[Error]$(NC) iOS install requires macOS"
endif

install-android: ## Install Android app to connected device
	@echo -e "$(BLUE)[Install]$(NC) Android..."
	@APK=$$(find $(TAURI_DIR)/gen/android -name "*.apk" -path "*/release/*" 2>/dev/null | head -1); \
	if [ -f "$$APK" ]; then \
		$(ANDROID_HOME)/platform-tools/adb install "$$APK"; \
		echo -e "$(GREEN)[Done]$(NC) Android app installed"; \
	else \
		echo -e "$(RED)[Error]$(NC) Android APK not found. Run 'make android' first."; \
	fi

# ============================================================================
# Asset Targets
# ============================================================================

ICON_PNG ?= $(TAURI_DIR)/icons/icon.png

icons: ## Generate all app icons using Tauri's icon generator (usage: make icons [ICON_PNG=path/to/1024x1024.png])
	@echo -e "$(BLUE)[Icons]$(NC) Generating from $(ICON_PNG)..."
	@if [ ! -f "$(ICON_PNG)" ]; then \
		echo -e "$(RED)[Error]$(NC) Source PNG not found: $(ICON_PNG)"; \
		echo -e "$(YELLOW)[Hint]$(NC) Provide a 1024x1024 PNG: make icons ICON_PNG=path/to/icon.png"; \
		exit 1; \
	fi
	@npx @tauri-apps/cli icon "$(ICON_PNG)" --output $(TAURI_DIR)/icons
	@echo -e "$(GREEN)[Done]$(NC) Icons generated in $(TAURI_DIR)/icons/"

# ============================================================================
# Quality Targets
# ============================================================================

test: setup-node ## Run tests
	@deno task test

lint: setup-node ## Run linter
	@deno task lint

clean: ## Clean build artifacts
	@rm -rf $(PROJECT_ROOT)/build
	@rm -rf $(TAURI_DIR)/target
	@rm -rf $(TAURI_DIR)/gen/android/app/build
	@rm -rf $(TAURI_DIR)/gen/apple/build
	@echo -e "$(GREEN)[Done]$(NC) Cleaned"
