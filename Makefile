# Cyb - Cross-Platform Build System
# Usage: make help

.PHONY: all setup build clean help
.PHONY: setup-node setup-rust setup-java setup-android setup-ios setup-linux
.PHONY: web dev dev-tauri build-web build-tauri
.PHONY: wasm wasm-build wasm-copy
.PHONY: macos linux ios android
.PHONY: install-ios install-android
.PHONY: test lint icons

# ============================================================================
# Configuration
# ============================================================================

SHELL := /bin/bash
PROJECT_ROOT := $(shell pwd)
TAURI_DIR := $(PROJECT_ROOT)/src-tauri
UHASH_ROOT := $(realpath $(PROJECT_ROOT)/../universal-hash)

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
	@grep -E '^(build-web|build-tauri|wasm|macos|linux|ios|android|build):.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(BLUE)%-20s$(NC) %s\n", $$1, $$2}'
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

setup-node: ## Install Node.js and Yarn dependencies
	@echo -e "$(BLUE)[Setup]$(NC) Node.js & Yarn..."
ifdef IS_MACOS
	@command -v node >/dev/null || (echo -e "$(RED)[Error]$(NC) Node.js not found. Install via: brew install node" && exit 1)
else
	@command -v node >/dev/null || (echo -e "$(YELLOW)[Setup]$(NC) Installing Node.js..." && \
		curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && \
		sudo apt-get install -y nodejs)
endif
	@command -v yarn >/dev/null || npm install -g yarn
	@yarn install
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
	@yarn start

dev-tauri: setup-node setup-rust ## Start Tauri dev server (native desktop)
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
# Build Targets
# ============================================================================

build: build-web ## Build web production bundle

build-web: setup-node ## Build web production bundle
	@echo -e "$(BLUE)[Build]$(NC) Web production..."
	@yarn build
	@echo -e "$(GREEN)[Done]$(NC) Web: $(PROJECT_ROOT)/build/"

build-tauri: setup-node setup-rust ## Build Tauri production bundle (current platform)
	@echo -e "$(BLUE)[Build]$(NC) Tauri production..."
	@npx @tauri-apps/cli build
	@echo -e "$(GREEN)[Done]$(NC) Tauri: $(TAURI_DIR)/target/release/bundle/"

macos: setup-node setup-rust ## Build macOS app (.dmg)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Build]$(NC) macOS..."
	@npx @tauri-apps/cli build
	@echo -e "$(GREEN)[Done]$(NC) macOS: $(TAURI_DIR)/target/release/bundle/dmg/"
else
	@echo -e "$(RED)[Error]$(NC) macOS builds require macOS"
endif

linux: setup-node setup-rust setup-linux ## Build Linux app (.deb, .AppImage)
ifdef IS_LINUX
	@echo -e "$(BLUE)[Build]$(NC) Linux..."
	@npx @tauri-apps/cli build
	@echo -e "$(GREEN)[Done]$(NC) Linux .deb: $(TAURI_DIR)/target/release/bundle/deb/"
	@echo -e "$(GREEN)[Done]$(NC) Linux .AppImage: $(TAURI_DIR)/target/release/bundle/appimage/"
else
	@echo -e "$(RED)[Error]$(NC) Linux builds require Linux"
endif

ios: setup-node setup-rust setup-ios ## Build iOS app (macOS only)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Build]$(NC) iOS..."
	@cd $(TAURI_DIR) && [ -d "gen/apple" ] || npx @tauri-apps/cli ios init
	@if [ -f "$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj" ]; then \
		grep -q 'export PATH=.*cargo' $(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj || \
		sed -i '' 's/shellScript = "cargo/shellScript = "export PATH=\\"$$HOME\/.cargo\/bin:$$PATH\\" \&\& cargo/g' \
			$(TAURI_DIR)/gen/apple/cyb.xcodeproj/project.pbxproj; \
	fi
	@npx @tauri-apps/cli ios build
	@echo -e "$(GREEN)[Done]$(NC) iOS: $(TAURI_DIR)/gen/apple/build/"
else
	@echo -e "$(RED)[Error]$(NC) iOS builds require macOS with Xcode"
endif

android: setup-node setup-rust setup-android ## Build Android app (.apk, aarch64 only)
	@echo -e "$(BLUE)[Build]$(NC) Android..."
	@cd $(TAURI_DIR) && [ -d "gen/android" ] || \
		JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android init
	@echo "sdk.dir=$(ANDROID_HOME)" > $(TAURI_DIR)/gen/android/local.properties
	@JAVA_HOME=$(JAVA_HOME) ANDROID_HOME=$(ANDROID_HOME) NDK_HOME=$(NDK_HOME) \
		npx @tauri-apps/cli android build --target aarch64 2>&1 | grep -v "WebSocket" || true
	@echo -e "$(GREEN)[Done]$(NC) Android APK: $(TAURI_DIR)/gen/android/app/build/outputs/apk/"

# ============================================================================
# Install Targets
# ============================================================================

install-ios: ## Install iOS app to connected device (macOS only)
ifdef IS_MACOS
	@echo -e "$(BLUE)[Install]$(NC) iOS..."
	@IPA=$$(find $(TAURI_DIR)/gen/apple/build -name "*.ipa" 2>/dev/null | head -1); \
	if [ -f "$$IPA" ]; then \
		DEVICE=$$(xcrun devicectl list devices 2>/dev/null | grep -o '[0-9A-F\-]\{36\}' | head -1); \
		if [ -n "$$DEVICE" ]; then \
			xcrun devicectl device install app --device "$$DEVICE" "$$IPA"; \
			echo -e "$(GREEN)[Done]$(NC) iOS app installed"; \
		else \
			echo -e "$(RED)[Error]$(NC) No iOS device connected"; \
		fi \
	else \
		echo -e "$(RED)[Error]$(NC) iOS build not found. Run 'make ios' first."; \
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

ICON_SVG ?= $(PROJECT_ROOT)/src/image/robot.svg
ICON_BG ?= 1a1a2e

icons: ## Generate app icons from SVG (usage: make icons [ICON_SVG=path/to.svg] [ICON_BG=hex])
	@echo -e "$(BLUE)[Icons]$(NC) Generating from $(ICON_SVG)..."
	@if [ ! -f "$(ICON_SVG)" ]; then \
		echo -e "$(RED)[Error]$(NC) SVG not found: $(ICON_SVG)"; \
		exit 1; \
	fi
	@python3 -c "from PIL import Image; print('Pillow OK')" 2>/dev/null || \
		(echo -e "$(YELLOW)[Setup]$(NC) Installing Pillow..." && pip3 install Pillow >/dev/null)
ifdef IS_MACOS
	@# Render SVG to PNG using macOS Quick Look
	@rm -f /tmp/_cyb_icon_render.png
	@qlmanage -t -s 1024 -o /tmp "$(ICON_SVG)" >/dev/null 2>&1
	@mv "/tmp/$$(basename $(ICON_SVG)).png" /tmp/_cyb_icon_render.png
	@python3 -c "\
	from PIL import Image; \
	import os; \
	img = Image.open('/tmp/_cyb_icon_render.png').convert('RGBA'); \
	px = img.load(); \
	w, h = img.size; \
	[px.__setitem__((x,y), (px[x,y][0],px[x,y][1],px[x,y][2],0)) for y in range(h) for x in range(w) if px[x,y][0]>250 and px[x,y][1]>250 and px[x,y][2]>250]; \
	bbox = img.getbbox(); \
	robot = img.crop(bbox) if bbox else img; \
	bg = tuple(int('$(ICON_BG)'[i:i+2],16) for i in (0,2,4)) + (255,); \
	canvas = Image.new('RGBA', (1024,1024), bg); \
	tw = int(1024*0.75); ratio = tw/robot.width; th = int(robot.height*ratio); \
	r = robot.resize((tw,th), Image.LANCZOS); \
	canvas.paste(r, ((1024-tw)//2, (1024-th)//2), r); \
	d = '$(TAURI_DIR)/icons'; \
	sizes = {'icon.png':512,'32x32.png':32,'128x128.png':128,'128x128@2x.png':256, \
		'Square30x30Logo.png':30,'Square44x44Logo.png':44,'Square71x71Logo.png':71, \
		'Square89x89Logo.png':89,'Square107x107Logo.png':107,'Square142x142Logo.png':142, \
		'Square150x150Logo.png':150,'Square284x284Logo.png':284,'Square310x310Logo.png':310, \
		'StoreLogo.png':50}; \
	[canvas.resize((s,s),Image.LANCZOS).save(os.path.join(d,n),'PNG') for n,s in sizes.items()]; \
	canvas.save(os.path.join(d,'_master.png'),'PNG'); \
	canvas.save(os.path.join(d,'icon.ico'),format='ICO',sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)]); \
	print('  PNGs + .ico generated')"
	@# Generate .icns using macOS iconutil
	@ICONSET=/tmp/_cyb_icon.iconset && rm -rf $$ICONSET && mkdir -p $$ICONSET && \
		MASTER=$(TAURI_DIR)/icons/_master.png && \
		sips -z 16 16     $$MASTER --out $$ICONSET/icon_16x16.png >/dev/null && \
		sips -z 32 32     $$MASTER --out $$ICONSET/icon_16x16@2x.png >/dev/null && \
		sips -z 32 32     $$MASTER --out $$ICONSET/icon_32x32.png >/dev/null && \
		sips -z 64 64     $$MASTER --out $$ICONSET/icon_32x32@2x.png >/dev/null && \
		sips -z 128 128   $$MASTER --out $$ICONSET/icon_128x128.png >/dev/null && \
		sips -z 256 256   $$MASTER --out $$ICONSET/icon_128x128@2x.png >/dev/null && \
		sips -z 256 256   $$MASTER --out $$ICONSET/icon_256x256.png >/dev/null && \
		sips -z 512 512   $$MASTER --out $$ICONSET/icon_256x256@2x.png >/dev/null && \
		sips -z 512 512   $$MASTER --out $$ICONSET/icon_512x512.png >/dev/null && \
		cp $$MASTER $$ICONSET/icon_512x512@2x.png && \
		iconutil -c icns $$ICONSET -o $(TAURI_DIR)/icons/icon.icns && \
		rm -rf $$ICONSET
	@rm -f $(TAURI_DIR)/icons/_master.png /tmp/_cyb_icon_render.png
	@echo -e "$(GREEN)[Done]$(NC) Icons generated in $(TAURI_DIR)/icons/"
else
	@# Linux: use rsvg-convert (from librsvg2-bin)
	@command -v rsvg-convert >/dev/null || (echo -e "$(YELLOW)[Setup]$(NC) Installing rsvg-convert..." && \
		sudo apt-get install -y librsvg2-bin >/dev/null)
	@rsvg-convert -w 1024 -h 1024 "$(ICON_SVG)" -o /tmp/_cyb_icon_render.png
	@python3 -c "\
	from PIL import Image; \
	import os; \
	img = Image.open('/tmp/_cyb_icon_render.png').convert('RGBA'); \
	px = img.load(); \
	w, h = img.size; \
	[px.__setitem__((x,y), (px[x,y][0],px[x,y][1],px[x,y][2],0)) for y in range(h) for x in range(w) if px[x,y][0]>250 and px[x,y][1]>250 and px[x,y][2]>250]; \
	bbox = img.getbbox(); \
	robot = img.crop(bbox) if bbox else img; \
	bg = tuple(int('$(ICON_BG)'[i:i+2],16) for i in (0,2,4)) + (255,); \
	canvas = Image.new('RGBA', (1024,1024), bg); \
	tw = int(1024*0.75); ratio = tw/robot.width; th = int(robot.height*ratio); \
	r = robot.resize((tw,th), Image.LANCZOS); \
	canvas.paste(r, ((1024-tw)//2, (1024-th)//2), r); \
	d = '$(TAURI_DIR)/icons'; \
	sizes = {'icon.png':512,'32x32.png':32,'128x128.png':128,'128x128@2x.png':256, \
		'Square30x30Logo.png':30,'Square44x44Logo.png':44,'Square71x71Logo.png':71, \
		'Square89x89Logo.png':89,'Square107x107Logo.png':107,'Square142x142Logo.png':142, \
		'Square150x150Logo.png':150,'Square284x284Logo.png':284,'Square310x310Logo.png':310, \
		'StoreLogo.png':50}; \
	[canvas.resize((s,s),Image.LANCZOS).save(os.path.join(d,n),'PNG') for n,s in sizes.items()]; \
	canvas.save(os.path.join(d,'icon.ico'),format='ICO',sizes=[(16,16),(24,24),(32,32),(48,48),(64,64),(128,128),(256,256)]); \
	print('  PNGs + .ico generated')"
	@rm -f /tmp/_cyb_icon_render.png
	@echo -e "$(YELLOW)[Note]$(NC) .icns not generated (requires macOS iconutil)"
	@echo -e "$(GREEN)[Done]$(NC) Icons generated in $(TAURI_DIR)/icons/"
endif

# ============================================================================
# Quality Targets
# ============================================================================

test: setup-node ## Run tests
	@yarn test

lint: setup-node ## Run ESLint
	@yarn lint

clean: ## Clean build artifacts
	@rm -rf $(PROJECT_ROOT)/build
	@rm -rf $(TAURI_DIR)/target
	@rm -rf $(TAURI_DIR)/gen/android/app/build
	@rm -rf $(TAURI_DIR)/gen/apple/build
	@echo -e "$(GREEN)[Done]$(NC) Cleaned"
