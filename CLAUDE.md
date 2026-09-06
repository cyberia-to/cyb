# CLAUDE.md — правила проекта cyb

## Архитектура

Bevy — корневой runtime. Один бинарник для всех платформ.

```
/                    Rust workspace (Cargo.toml, Makefile)
├── shell/           Bevy desktop runtime — package/bin `cyb`, all worlds
├── apps/            Leptos WASM web apps — loaded via WebView in Portal world
├── reference/       Спецификация, roadmap
├── graph/           Knowledge graph pages
├── docs/            Документация
```

Внешние зависимости:
- `~/cyber/nu/` — форк nushell (nu-protocol, nu-engine, sugarloaf и др.)
- `~/cyber/evy/forks/naga/` — форк naga (shader compiler)

## One Binary Rule

**WebView is only for web content (Portal world).** All other rendering — terminal, graph, 3D, UI — is pure Rust via Bevy + wgpu. No JS bundles, no external runtimes embedded in the binary.

**Android = desktop.** The Android build is the same app, same Bevy, same worlds, same terminal, same nushell. The only Android-specific addition is `nu_plugin_android` which exposes hardware APIs (GPS, camera, sensors, etc.) that desktop doesn't have. There is no separate Android UI or Android terminal — it's one codebase.

## Worlds

| World     | Hotkey | Description                        |
|-----------|--------|------------------------------------|
| Splash    | —      | Boot screen                        |
| Spells    | Cmd+1  | AI agent / spell runner            |
| Graph     | Cmd+2  | Knowledge graph (mir engine)       |
| Sense     | Cmd+3  | Sensor / perception view           |
| Terminal  | Cmd+4  | Nushell terminal (sugarloaf)       |
| Portal    | Cmd+5  | Leptos WASM web UI via WebView     |
| Interface | Cmd+6  | Native Bevy UI                     |

## Сборка

```bash
# Bevy shell (desktop):
make build                         # debug build
make run                           # release run
make dmg                           # macOS release + DMG

# Leptos apps:
make apps                          # trunk build --release

# Android:
make android                       # full: rust + assets + apk

# Dev:
make dev                           # cargo run -p cyb
```

## Проверка после изменений

- **Fleet (обязателен перед коммитом)**: `make fleet` — мульти-окружение:
  N изолированных тел (свои HOME) + детерминированный mockchain
  (`harness/mockchain.py`), ассерты на boot/identity/graph/networks/
  beacon/prover/vault/orphans. Хук pre-commit гоняет его сам;
  `SKIP_FLEET=1` для docs-only коммитов. `FLEET_SKIP_BUILD=1` если
  бинарь свежий.
- **Shell**: `cargo check -p cyb`
- **Apps**: `cd apps && trunk build --release`
- **Release**: `make dmg`
- **Android**: `make android`

## Релизы

- **`make ship`** — каждая версия уходит релизом на GitHub (как erga):
  бамп версии, коммит через fleet-ворота, тег, dmg чистого дерева
  (без `*` в маркере версии), notes из git log, установка локально.
  `V=0.3.0 T="headline"` для явных значений. ship сам пушит master+tags.

## Git

- **Push только по запросу** (исключение: `make ship` пушит по определению)
- **Атомарные коммиты**

## Планы и решения

`.claude/plans/` — утверждённые планы.
`reference/roadmap.md` — общий roadmap.
