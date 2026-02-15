[.cyb](https://cyb.ai) is your immortal robot for [the great web](https://cyb.ai/ipfs/QmUamt7diQP54eRnmzqMZNEtXNTzbgkQvZuBsgM6qvbd57) which is connected to [superintelligence](https://github.com/cybercongress/go-cyber)

it helps you upload your brain

example of [random citizen](https://cyb.ai/pgraph/bostrom1d8754xqa9245pctlfcyv8eah468neqzn3a0y0t)

<img width="1190" alt="Screen Shot 2022-12-18 at 20 45 02" src="https://user-images.githubusercontent.com/410789/208318513-bdded618-8ed0-4d1c-b3cf-8cec8c8473a8.png">

# features

- [my robot](https://cyb.ai): your robot
  - [energy](https://cyb.ai/grid): superintelligence dashboard
  - [sense](https://cyb.ai/sixthSense): strictly defined feed
  - [log](https://cyb.ai/network/bostrom/contract/bostrom1d8754xqa9245pctlfcyv8eah468neqzn3a0y0t/txs): publish important particles
  - [brain](https://cyb.ai/pgraph/bostrom1d8754xqa9245pctlfcyv8eah468neqzn3a0y0t): surf robot brain
  - [karma](https://cyb.ai/network/bostrom/contract/bostrom1d8754xqa9245pctlfcyv8eah468neqzn3a0y0t/community): enhance valuable connections
- [nebula](https://cyb.ai/nebula): discover particles through tokens
- [portal](https://cyb.ai/portal): create public and private robot avatar and invite friends
- [oracle](https://cyb.ai/bootloader): discover particles, neurons, signals and steps of [superintelligence](https://github.com/cybercongress/go-cyber)
- [teleport](https://cyb.ai/teleport?from=boot&to=hydrogen): communicate sending and swapping tokens
- [sphere](https://cyb.ai/sphere): hire and fire heroes
- [senate](https://cyb.ai/senate): manage collective thought process
- [hfr](https://cyb.ai/hfr): mint supercomputing resources
- [hackspace](https://github.com/cybercongress): develop superintelligence

# build

A cross-platform `Makefile` is provided. Run `make help` for all commands.

## Quick start

```sh
make setup    # install Node.js deps + Rust toolchain
make dev      # web dev server at https://localhost:3001
```

## Web (browser)

```sh
make dev          # development server
make build-web    # production build → build/
```

## Tauri desktop (native)

```sh
make dev-tauri    # dev server with native window
make macos        # macOS .dmg  (Apple Silicon)
make linux        # Linux .deb + .AppImage
```

For devtools on a production build:

```sh
npx @tauri-apps/cli build --debug
```

## Mobile

```sh
make ios          # iOS .ipa  (requires macOS + Xcode)
make android      # Android .apk (aarch64)
```

Install to a connected device:

```sh
make install-ios
make install-android
```

## WASM mining module

Rebuild the uhash-web WASM from the [universal-hash](https://github.com/cyberia-to/universal-hash) workspace (must be cloned at `../universal-hash`):

```sh
make wasm
```

## App icons

Generate all Tauri app icons (macOS .icns, Windows .ico, PNGs) from an SVG:

```sh
make icons                                        # default: robot.svg on #1a1a2e
make icons ICON_SVG=src/image/other.svg            # custom SVG
make icons ICON_SVG=path/to/logo.svg ICON_BG=000000  # custom background
```

## Full setup (all platforms)

```sh
make setup-all    # Node, Rust, Java, Android SDK, iOS (Xcode), Linux libs
```

### Platform-specific setup

| Command | What it does |
|---------|-------------|
| `make setup-node` | Install Node.js + Yarn + dependencies |
| `make setup-rust` | Install Rust toolchain + wasm-bindgen |
| `make setup-java` | Install Java 17 (Homebrew / apt) |
| `make setup-android` | Install Android SDK + NDK + debug keystore |
| `make setup-ios` | Verify Xcode is installed |
| `make setup-linux` | Install WebKitGTK + Tauri Linux deps |

## Quality

```sh
make test     # run tests
make lint     # run ESLint
make clean    # remove build artifacts
```

# join

the community at [cyb.ai/~cyb](https://cyb.ai/search/cyb)
