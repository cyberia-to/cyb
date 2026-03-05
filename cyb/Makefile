.PHONY: dev build run clean check test dmg

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

# Build macOS .dmg
dmg: release portal
	rm -rf target/release/cyb.app
	mkdir -p target/release/cyb.app/Contents/MacOS
	mkdir -p target/release/cyb.app/Contents/Resources
	cp target/release/cyb-shell target/release/cyb.app/Contents/MacOS/cyb-shell
	@if [ -d "cyb-portal/dist" ]; then \
		cp -r cyb-portal/dist target/release/cyb.app/Contents/MacOS/cyb-portal; \
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
	rm -f target/release/cyb.dmg
	hdiutil create -volname "cyb" -srcfolder target/release/cyb.app -ov -format UDZO target/release/cyb.dmg

# Full dev workflow
# Terminal 1: cd .. && deno task start     (cyb-ts HTTPS :3001 → Legacy mode)
# Terminal 2: cd cyb-portal && trunk serve (Leptos :8090 → Portal mode)
# Terminal 3: make dev                     (Bevy shell)
