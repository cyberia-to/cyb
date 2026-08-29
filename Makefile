.PHONY: dev fast build run clean check apps dmg android android-rust android-assets \
        android-jnilibs android-apk android-debug android-run android-log

# Resolve the toolchain by absolute path instead of trusting PATH. Homebrew
# ships its own cargo and rustc, and a `rustc` picked up from PATH here can be
# Homebrew's even when `cargo` is rustup's — mixing the two poisons target/
# with artifacts each rejects ("found crate compiled by an incompatible
# version of rustc"), and Homebrew's rustc has no cross targets at all, so an
# Android build under it dies with "can't find crate for `core`".
RUSTUP_RUSTC ?= $(shell $(HOME)/.cargo/bin/rustup which rustc 2>/dev/null)
RUSTUP_BIN   := $(dir $(RUSTUP_RUSTC))
CARGO        ?= $(RUSTUP_BIN)cargo
export RUSTC := $(RUSTUP_RUSTC)

# Fail loudly rather than building half the workspace with the wrong compiler.
ifeq ($(RUSTUP_RUSTC),)
$(error cannot resolve the rustup toolchain — is rustup installed?)
endif

# Development: debug build (fastest iteration, no opt)
dev:
	$(CARGO) run -p cyb

# Fast release-quality dev build (thin-LTO, 8 CUs — ~60s incremental vs 290s fat)
fast:
	$(CARGO) run --profile release-dev -p cyb

# Build (debug)
build:
	$(CARGO) build -p cyb

# Build release (fat-LTO, for DMG)
release:
	$(CARGO) build --release -p cyb

# Run release binary
run:
	$(CARGO) run --release -p cyb

# Check all (fast, no codegen)
check:
	$(CARGO) check --workspace

# Clean build artifacts
clean:
	$(CARGO) clean

# Build Leptos WASM apps
apps:
	cd apps && RUSTC=$(RUSTUP_RUSTC) trunk build --release

# Build macOS .dmg
dmg: release apps
	rm -rf target/release/cyb.app
	mkdir -p target/release/cyb.app/Contents/MacOS
	mkdir -p target/release/cyb.app/Contents/Resources
	cp target/release/cyb target/release/cyb.app/Contents/MacOS/cyb
	cp shell/assets/icon.icns target/release/cyb.app/Contents/Resources/icon.icns
	@if [ -d "apps/dist" ]; then \
		cp -r apps/dist target/release/cyb.app/Contents/MacOS/cyb-apps; \
	fi
	/usr/libexec/PlistBuddy -c "Add :CFBundleName string cyb" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleDisplayName string cyb" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleExecutable string cyb" target/release/cyb.app/Contents/Info.plist
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
JAVA_HOME    ?= $(shell /usr/libexec/java_home -v 17 2>/dev/null || echo /opt/homebrew/opt/openjdk@17)
# With both an emulator and a phone attached, pick one: make android-run ADB_SERIAL=<id>
ADB_SERIAL   ?=
ADB          ?= $(ANDROID_HOME)/platform-tools/adb $(if $(ADB_SERIAL),-s $(ADB_SERIAL))
# Discover NDK via find — never `ls` (colorized aliases inject ANSI into paths).
NDK_HOME     ?= $(shell find "$(ANDROID_HOME)/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -1)
NDK_VERSION  ?= $(notdir $(NDK_HOME))
ANDROID_TARGET ?= aarch64-linux-android
ANDROID_API  ?= 24
NDK_HOST     ?= $(shell find "$(NDK_HOME)/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -1 | xargs -I{} basename {})
NDK_BIN      ?= $(NDK_HOME)/toolchains/llvm/prebuilt/$(NDK_HOST)/bin

# Full Android build: compile Rust → copy assets → assemble APK
android: android-rust android-apk
	@echo "APK: shell/gen/android/app/build/outputs/apk/release/app-release-unsigned.apk"

# Cross-compile Rust → libcyb.so
android-rust:
	CC_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	CXX_aarch64_linux_android=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang++ \
	AR_aarch64_linux_android=$(NDK_BIN)/llvm-ar \
	CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$(NDK_BIN)/aarch64-linux-android$(ANDROID_API)-clang \
	$(CARGO) build --release --target $(ANDROID_TARGET) --lib -p cyb

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

# Stage the cross-compiled library where Gradle expects it. Stripped: the
# full .so stays in target/ for symbolising panics; the APK carries ~1/3.
android-jnilibs:
	@mkdir -p shell/gen/android/app/src/main/jniLibs/arm64-v8a
	$(NDK_BIN)/llvm-strip --strip-debug \
		-o shell/gen/android/app/src/main/jniLibs/arm64-v8a/libcyb.so \
		target/$(ANDROID_TARGET)/release/libcyb.so
	cp $(NDK_BIN)/../sysroot/usr/lib/aarch64-linux-android/libc++_shared.so \
		shell/gen/android/app/src/main/jniLibs/arm64-v8a/

# Release APK — unsigned, for distribution. `adb install` rejects it.
android-apk: android-jnilibs
	cd shell/gen/android && ANDROID_HOME=$(ANDROID_HOME) JAVA_HOME=$(JAVA_HOME) ./gradlew assembleRelease

# Debug APK — Gradle signs it with the local debug key, so a device accepts it
android-debug: android-rust android-jnilibs
	cd shell/gen/android && ANDROID_HOME=$(ANDROID_HOME) JAVA_HOME=$(JAVA_HOME) ./gradlew assembleDebug

# Source to running app on a plugged-in phone, one command
android-run: android-debug
	$(ADB) install -r shell/gen/android/app/build/outputs/apk/debug/app-debug.apk
	$(ADB) shell am start -n ai.cyb.app/.MainActivity
	@echo "logs: make android-log"

# Live Rust logs from the device — panics land here and nowhere else
android-log:
	$(ADB) logcat -s cyb:V RustStdoutStderr:V
