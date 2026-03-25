# CLAUDE.md — правила проекта cyb

## Архитектура

Четыре реализации одного робота, bevy — корневой runtime:

```
/                    Rust workspace (Cargo.toml, Makefile)
├── bevy/            Bevy shell — нативный UI, оркестрирует все режимы
├── react/           React app (cyb-ts) — web UI через WebView
├── leptos/          Leptos WASM — portal UI через WebView
├── nu/              Nushell — scripting/terminal
├── vendor/          Внешние зависимости (nushell submodule и др.)
├── reference/       Спецификация, roadmap
├── graph/           Knowledge graph pages
├── docs/            Документация
```

## Сборка

```bash
# React (web UI):
cd react && deno install
cd react && deno task start        # dev server (HTTPS, HMR)
cd react && deno task build        # production build

# Bevy shell (desktop):
make build                         # debug build
make dmg                           # macOS release + DMG

# Android:
ANDROID_HOME=/opt/homebrew/share/android-commandlinetools make android

# Всё вместе:
make dev                           # dev server + bevy shell
```

## Проверка после изменений

- **React**: `cd react && deno task build`
- **Desktop**: `make dmg`
- **Android**: `make android`

## Git

- **Push только по запросу**
- **Атомарные коммиты**

## Планы и решения

`.claude/plans/` — утверждённые планы.
`reference/roadmap.md` — общий roadmap.

## React стек (react/)

- Runtime: Deno 2, Bundler: Rspack 1.7
- Framework: React 18, TypeScript 5
- `DENO_NO_PACKAGE_JSON=1` во всех deno tasks

## React структура (react/src/)

- `components/` — UI компоненты
- `containers/` — контейнеры страниц
- `pages/` — страницы
- `features/` — фичи
- `services/` — сервисы (IPFS, backend, scripting)
- `redux/` — Redux store
- `utils/` — утилиты
- `contexts/` — React context providers
- `hooks/` — кастомные хуки

## Безопасность

- Iframe: sandbox для IPFS/gateway
- CSP: в `react/src/index.html`
- Scripting: DOMPurify санитизация
- Secrets: localStorage (unencrypted)
