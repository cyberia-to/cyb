.PHONY: dev fast build run clean check apps dmg android android-rust android-assets \
        android-jnilibs android-apk android-debug android-run android-log \
        ios ios-ipa

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

# The app version is the shell crate's version. The bundle carries it so
# "About" and `defaults read` show the release number, not a placeholder.
VERSION := $(shell sed -n 's/^version *= *"\(.*\)"/\1/p' shell/Cargo.toml | head -1)

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
# Optional multi-environment proof: N isolated bodies + mockchain.
# Not a commit hook — it opens GUI windows. FLEET_SKIP_BUILD=1 to trust the binary.
fleet:
	bash harness/fleet.sh

# Every version is a GitHub release, the erga way: bump, commit, tag,
# dmg, publish, install. V=x.y.z T="headline" to override.
ship:
	bash harness/ship.sh

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
	/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $(VERSION)" target/release/cyb.app/Contents/Info.plist
	/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $(VERSION)" target/release/cyb.app/Contents/Info.plist
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
# Android version = the crate version. versionCode must be a rising
# integer: major*10000 + minor*100 + patch (0.12.2 -> 1202). Without this
# every APK claimed versionCode 1 / 0.1.0 and no phone could tell two
# releases apart or update one to the next.
CYB_VER      := $(shell awk -F'"' '/^version/{print $$2; exit}' shell/Cargo.toml)
CYB_VER_CODE := $(shell echo "$(CYB_VER)" | awk -F. '{print $$1*10000 + $$2*100 + $$3}')
NDK_HOST     ?= $(shell find "$(NDK_HOME)/toolchains/llvm/prebuilt" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | head -1 | xargs -I{} basename {})
NDK_BIN      ?= $(NDK_HOME)/toolchains/llvm/prebuilt/$(NDK_HOST)/bin

# Full Android build: compile Rust → copy assets → assemble APK
android: android-rust android-apk
	@echo "APK: shell/gen/android/app/build/outputs/apk/release/app-release.apk"

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

# Release APK — Gradle signs with ~/.cyb-release.keystore when present.
android-apk: android-jnilibs
	cd shell/gen/android && ANDROID_HOME=$(ANDROID_HOME) JAVA_HOME=$(JAVA_HOME) ./gradlew assembleRelease \
		-PcybVersionCode=$(CYB_VER_CODE) -PcybVersionName=$(CYB_VER)

# Debug APK — same keystore as release, so the two upgrade each other.
android-debug: android-rust android-jnilibs
	cd shell/gen/android && ANDROID_HOME=$(ANDROID_HOME) JAVA_HOME=$(JAVA_HOME) ./gradlew assembleDebug \
		-PcybVersionCode=$(CYB_VER_CODE) -PcybVersionName=$(CYB_VER)

# Source to running app on a plugged-in phone, one command
android-run: android-debug
	$(ADB) install -r -d shell/gen/android/app/build/outputs/apk/debug/app-debug.apk
	$(ADB) shell am start -n ai.cyb.app/.MainActivity
	@echo "logs: make android-log"

# Live Rust logs from the device — panics land here and nowhere else
android-log:
	$(ADB) logcat -s cyb:V RustStdoutStderr:V

# ── iOS (TrollStore / jailbreak, unsigned IPA) ───────────────
# Device arm64. Min iOS 14 so TrollStore's 14–17.0 window can open it.
# ldid fakesigns; TrollStore re-signs on install. Not App Store.
# Needs an iPhoneOS SDK. This machine has no Xcode.app; unpack
# theos iPhoneOS16.5.sdk into ~/.cyb/sdks and keep the xcrun shim
# at ~/.cyb/ios-bin so rustc/cc can find it.
IOS_TARGET  ?= aarch64-apple-ios
IOS_APP     := target/$(IOS_TARGET)/release/cyb.app
IOS_IPA     := target/release/cyb-$(VERSION).ipa
IOS_SDKROOT ?= $(HOME)/.cyb/sdks/iPhoneOS16.5.sdk
IOS_XCRUN   ?= $(HOME)/.cyb/ios-bin

ios: ios-ipa

ios-ipa:
	@test -d "$(IOS_SDKROOT)" || { echo "missing iPhoneOS SDK at $(IOS_SDKROOT)"; exit 1; }
	@test -x "$(IOS_XCRUN)/xcrun" || { echo "missing xcrun shim at $(IOS_XCRUN)/xcrun"; exit 1; }
	IPHONEOS_DEPLOYMENT_TARGET=14.0 \
	IOS_SDKROOT=$(IOS_SDKROOT) \
	PATH="$(IOS_XCRUN):$(PATH)" \
	CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS="-C target-feature=+aes,+sha2 -C link-arg=-miphoneos-version-min=14.0" \
	$(CARGO) build --release --target $(IOS_TARGET) --bin cyb -p cyb
	rm -rf $(IOS_APP) Payload
	mkdir -p $(IOS_APP)
	cp target/$(IOS_TARGET)/release/cyb $(IOS_APP)/cyb
	cp shell/gen/ios/Info.plist $(IOS_APP)/Info.plist
	/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $(VERSION)" $(IOS_APP)/Info.plist
	/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $(CYB_VER_CODE)" $(IOS_APP)/Info.plist
	sips -z 120 120 shell/assets/icon-source.png --out $(IOS_APP)/AppIcon60x60@2x.png >/dev/null
	sips -z 180 180 shell/assets/icon-source.png --out $(IOS_APP)/AppIcon60x60@3x.png >/dev/null
	sips -z 152 152 shell/assets/icon-source.png --out $(IOS_APP)/AppIcon76x76@2x.png >/dev/null
	printf 'APPL????' > $(IOS_APP)/PkgInfo
	mkdir -p $(IOS_APP)/assets
	cp -R shell/assets/. $(IOS_APP)/assets/
	ldid -Sshell/gen/ios/entitlements.plist $(IOS_APP)/cyb
	rm -rf Payload
	mkdir Payload
	cp -R $(IOS_APP) Payload/cyb.app
	rm -f $(IOS_IPA)
	zip -qry $(IOS_IPA) Payload
	rm -rf Payload
	@echo "IPA: $(IOS_IPA)"
	@echo "TrollStore: open the ipa in TrollStore (iOS 14–17.0)"
	@echo "jailbreak: Filza / TrollStore / AppSync, same file"
