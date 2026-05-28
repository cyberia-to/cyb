.PHONY: dev fast build run clean check apps dmg android android-rust android-assets android-apk

# Development: debug build (fastest iteration, no opt)
dev:
	cargo run -p cyb-shell

# Fast release-quality dev build (thin-LTO, 8 CUs — ~60s incremental vs 290s fat)
fast:
	cargo run --profile release-dev -p cyb-shell

# Build (debug)
build:
	cargo build -p cyb-shell

# Build release (fat-LTO, for DMG)
release:
	cargo build --release -p cyb-shell

# Run release binary
run:
	cargo run --release -p cyb-shell

# Check all (fast, no codegen)
check:
	cargo check --workspace

# Clean build artifacts
clean:
	cargo clean

RUSTUP_RUSTC := $(HOME)/.rustup/toolchains/stable-aarch64-apple-darwin/bin/rustc

# Build Leptos WASM apps
apps:
	cd apps && RUSTC=$(RUSTUP_RUSTC) trunk build --release

# Build macOS .dmg
dmg: release apps
	rm -rf target/release/cyb.app
	mkdir -p target/release/cyb.app/Contents/MacOS
	mkdir -p target/release/cyb.app/Contents/Resources
	cp target/release/cyb-shell target/release/cyb.app/Contents/MacOS/cyb-shell
	cp shell/assets/icon.icns target/release/cyb.app/Contents/Resources/icon.icns
	@if [ -d "apps/dist" ]; then \
		cp -r apps/dist target/release/cyb.app/Contents/MacOS/cyb-apps; \
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

KOTLIN_OUT   = shell/gen/android/app/src/main/kotlin/ai/cyb/app

# Full Android build: compile Rust → copy assets → assemble APK
android: android-rust android-assets android-apk
	@echo "APK: shell/gen/android/app/build/outputs/apk/release/app-release-unsigned.apk"

# Cross-compile Rust → libcyb_shell.so
android-rust:
	WRY_ANDROID_PACKAGE=ai.cyb.app \
	WRY_ANDROID_LIBRARY=cyb_shell \
	WRY_ANDROID_KOTLIN_FILES_OUT_DIR=$(PWD)/$(KOTLIN_OUT) \
	CC_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	CXX_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang++ \
	AR_aarch64_linux_android=$(NDK_BIN)/llvm-ar \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	cargo build --release --target $(ANDROID_TARGET) --lib -p cyb-shell --no-default-features --features android

# Copy apps build into Android assets
android-assets:
	@rm -rf shell/gen/android/app/src/main/assets/cyb-apps
	@if [ -d "apps/dist" ]; then \
		mkdir -p shell/gen/android/app/src/main/assets/cyb-apps; \
		cp -r apps/dist/* shell/gen/android/app/src/main/assets/cyb-apps/; \
		echo "Apps assets copied"; \
	else \
		echo "WARNING: apps/dist not found, run 'make apps' first"; \
	fi

# Copy .so and assemble APK via Gradle
android-apk:
	@mkdir -p shell/gen/android/app/src/main/jniLibs/arm64-v8a
	cp target/$(ANDROID_TARGET)/release/libcyb_shell.so \
		shell/gen/android/app/src/main/jniLibs/arm64-v8a/
	cd shell/gen/android && ./gradlew assembleRelease
