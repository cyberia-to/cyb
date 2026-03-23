.PHONY: dev build run clean check test dmg android android-rust android-assets android-apk

# Development: run cyb-shell
# Portal mode (Cmd+2): cd cyb-portal && trunk serve  (Leptos :8090)
# Legacy mode (Cmd+3): cd .. && deno task start       (cyb-ts :3001)
dev:
	cargo run -p cyb-shell

# Build all workspace members (debug)
build:
	cargo build -p cyb-shell
	cargo build -p cyb-services

# Build release
release:
	cargo build --release -p cyb-shell
	cargo build --release -p cyb-services

# Run release binary
run:
	cargo run --release -p cyb-shell

# Check all (fast, no codegen)
check:
	cargo check --workspace

# Run tests
test:
	cargo test -p cyb-services

# Clean build artifacts
clean:
	cargo clean

# Build cyb-portal (Leptos WASM)
portal:
	cd cyb-portal && trunk build --release

# Build web app (React/TypeScript via Rspack)
# Rspack exits 1 on size warnings — ignore, check build/ exists instead
web:
	-cd .. && ~/.deno/bin/deno task build
	@test -f ../build/index.html || (echo "ERROR: web build failed" && exit 1)

# Build macOS .dmg
dmg: release portal web
	rm -rf target/release/cyb.app
	mkdir -p target/release/cyb.app/Contents/MacOS
	mkdir -p target/release/cyb.app/Contents/Resources
	cp target/release/cyb-shell target/release/cyb.app/Contents/MacOS/cyb-shell
	cp cyb-shell/assets/icon.icns target/release/cyb.app/Contents/Resources/icon.icns
	@if [ -d "cyb-portal/dist" ]; then \
		cp -r cyb-portal/dist target/release/cyb.app/Contents/MacOS/cyb-portal; \
	fi
	@if [ -d "../build" ]; then \
		cp -r ../build target/release/cyb.app/Contents/MacOS/cyb-web; \
	fi
	/usr/libexec/PlistBuddy -c "Add :CFBundleName string cyb" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string cyb" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string cyb-shell" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleIdentifier string ai.cyb.app" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string 0.1.0" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string 0.1.0" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundlePackageType string APPL" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :LSMinimumSystemVersion string 13.0" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :NSHighResolutionCapable bool true" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string icon" target/release/cyb.app/Contents/Info.plist
	rm -f target/release/cyb.dmg
	hdiutil create -volname "cyb" -srcfolder target/release/cyb.app -ov -format UDZO target/release/cyb.dmg

# ── Android ──────────────────────────────────────────────────
ANDROID_HOME ?= $(HOME)/Library/Android/sdk
NDK_VERSION  ?= $(shell ls $(ANDROID_HOME)/ndk 2>/dev/null | sort -V | tail -1)
NDK_HOME     ?= $(ANDROID_HOME)/ndk/$(NDK_VERSION)
ANDROID_TARGET ?= aarch64-linux-android
ANDROID_API  ?= 24
NDK_HOST     ?= $(shell ls $(NDK_HOME)/toolchains/llvm/prebuilt/ 2>/dev/null | head -1)
NDK_BIN      ?= $(NDK_HOME)/toolchains/llvm/prebuilt/$(NDK_HOST)/bin

KOTLIN_OUT   = cyb-shell/gen/android/app/src/main/kotlin/ai/cyb/app

# Full Android build: compile Rust → copy assets → assemble APK
android: android-rust android-assets android-apk
	@echo "APK: cyb-shell/gen/android/app/build/outputs/apk/release/app-release-unsigned.apk"

# Cross-compile Rust → libcyb_shell.so
android-rust: web
	WRY_ANDROID_PACKAGE=ai.cyb.app \
	WRY_ANDROID_LIBRARY=cyb_shell \
	WRY_ANDROID_KOTLIN_FILES_OUT_DIR=$(PWD)/$(KOTLIN_OUT) \
	CC_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	CXX_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang++ \
	AR_aarch64_linux_android=$(NDK_BIN)/llvm-ar \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	cargo build --release --target $(ANDROID_TARGET) --lib -p cyb-shell --no-default-features --features android

# Copy web build into Android assets (strip sourcemaps)
android-assets:
	@rm -rf cyb-shell/gen/android/app/src/main/assets/cyb-web
	@if [ -d "../build" ]; then \
		mkdir -p cyb-shell/gen/android/app/src/main/assets/cyb-web; \
		cp -r ../build/* cyb-shell/gen/android/app/src/main/assets/cyb-web/; \
		find cyb-shell/gen/android/app/src/main/assets/cyb-web -name "*.map" -delete; \
		echo "Assets copied ($$(du -sh cyb-shell/gen/android/app/src/main/assets/cyb-web | cut -f1))"; \
	else \
		echo "WARNING: ../build not found, run 'make web' first"; \
	fi

# Copy .so and assemble APK via Gradle
android-apk:
	@mkdir -p cyb-shell/gen/android/app/src/main/jniLibs/arm64-v8a
	cp target/$(ANDROID_TARGET)/release/libcyb_shell.so \
		cyb-shell/gen/android/app/src/main/jniLibs/arm64-v8a/
	cd cyb-shell/gen/android && ./gradlew assembleRelease

# Dev with live reload: starts dev server + Bevy shell
# Web changes hot-reload instantly via HMR, no rebuilds needed
dev:
	@echo "Starting dev server + Bevy shell (live reload)..."
	@echo "Web HMR on https://localhost:3001 → WebView auto-updates"
	@cd .. && ~/.deno/bin/deno task start &
	@sleep 3
	cargo run -p cyb-shell

# Full dev workflow (manual)
# Terminal 1: cd .. && deno task start     (cyb-ts HTTPS :3001 → Legacy mode)
# Terminal 2: cd cyb-portal && trunk serve (Leptos :8090 → Portal mode)
# Terminal 3: cargo run -p cyb-shell       (Bevy shell)
