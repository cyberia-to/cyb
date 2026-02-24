# CLAUDE.md — правила проекта cyb-ts

## Рабочий процесс

Для проверки после коммита — запускать `deno task build` (как CI), а не dev server.
Dev server (`deno task start`) запускать только по явному запросу пользователя.

### Проверка после изменений

После каждого изменения кода — **обязательно пересобирать и проверять результат**:
- **Web (cyb-ts)**: `deno task build` — полный production билд
- **Desktop (cyb-shell)**: `cd cyb && make dmg` — release билд + DMG
- Не оставлять только `cargo check` — нужен полный `make dmg` чтобы убедиться что артефакт обновился
- DMG находится в `cyb/target/release/cyb.dmg`

## Git

- **Push только по запросу** — никогда не пушить без явного указания пользователя
- **Атомарные коммиты** — один коммит на одну функцию/изменение, не смешивать несвязанные правки

## Планы и решения

Все утверждённые планы сохраняются в `.claude/plans/`, а не во временные папки.
Файл `reference/roadmap.md` — общий roadmap проекта.

## Стек

- Runtime: **Deno 2** (task runner, build, dev server, lint)
- Bundler: **Rspack 1.7** (Rust-based)
- Package manager: **Deno** (`deno install` для node_modules)
- Test runner: отсутствует (Jest удалён, тесты в src/ временно не запускаются)
- Framework: React 18, TypeScript 5

## Сборка

```bash
# Установка зависимостей:
deno install

# Все команды через Deno:
deno task start        # dev server (HTTPS, HMR)
deno task build        # production build
deno task build-ipfs   # IPFS production build
deno task lint         # eslint
deno task serve        # serve production build
```

Важно:
- `DENO_NO_PACKAGE_JSON=1` используется во всех deno tasks чтобы Deno не парсил package.json
- `package.json scripts` удалены (кроме `generate-graphql-types`) — всё в `deno.json`
- Yarn больше не нужен — `react-force-graph` заменён на `react-force-graph-3d` (без aframe/VR/AR)

## Структура

- `src/components/` — переиспользуемые UI компоненты
- `src/containers/` — контейнеры страниц (legacy, мигрировать в pages/features)
- `src/pages/` — страницы
- `src/features/` — фичи (feature-sliced подход)
- `src/services/` — сервисы (IPFS, backend, scripting, etc.)
- `src/redux/` — Redux store
- `src/utils/` — утилиты (encoding.ts — замена Buffer для Web APIs)
- `src/contexts/` — React context providers
- `src/hooks/` — кастомные хуки
- `src/generated/` — сгенерированные GraphQL типы

## Безопасность

- Iframe: sandbox атрибут обязателен для любого IPFS/gateway контента
- CSP: Content-Security-Policy задана в `src/index.html`
- Scripting output: DOMPurify санитизация Rune script content_result в `src/services/scripting/services/postProcessing.ts`
- Secrets: хранятся в localStorage (unencrypted) — ключ `secrets`

## Node polyfills

- `@rspack/plugin-node-polyfill` удалён — был глобальный полифилл всех Node API
- `src/utils/encoding.ts` — замена Buffer.from() на Web APIs (toHex, toBase64, toBytes, fromBytes)
- `ProvidePlugin` точечно инжектит `process` и `Buffer` для сторонних пакетов
- `resolve.fallback` в rspack конфиге — полифиллы для сторонних пакетов (crypto, stream, etc.)
