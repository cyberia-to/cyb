# План миграции: Node.js/Webpack -> Deno 2/Rspack

**Дата:** 2026-02-19
**Статус:** черновик, ждёт утверждения
**Цель:** избавиться от Node.js, перейти на Rust-based тулчейн (Deno 2 + Rspack)

---

## Стратегия

Миграция в 5 фаз. Каждая фаза — самодостаточная, проект остаётся рабочим после каждой.
Порядок: сначала бандлер (быстрый win), потом рантайм, потом зачистка.

---

## Фаза 0: Подготовка (перед миграцией)

### 0.1 Закоммитить текущие изменения
- [ ] Закоммитить 6 незакоммиченных файлов (error handling, analytics API, CSS fix)
- [ ] Убедиться что master чистый

### 0.2 Починить pre-existing проблемы
- [ ] Убрать `reflect-metadata` из `tsconfig.json` types (пакет не установлен, TS ошибка)
- [ ] Прогнать `yarn lint` и починить критичные ошибки

### 0.3 Зафиксировать базовый бенчмарк
- [ ] Замерить время `yarn build` (cold build)
- [ ] Замерить время `yarn start` (dev server startup)
- [ ] Записать размер бандла (`build/` после production build)

---

## Фаза 1: Webpack -> Rspack

**Почему первым:** минимальные изменения в коде, API-совместим с webpack, мгновенный прирост скорости.

### 1.1 Установить Rspack
- [ ] `yarn add -D @rspack/core @rspack/cli`
- [ ] Удалить webpack-специфичные пакеты: `webpack`, `webpack-cli`, `webpack-dev-server`, `webpack-merge`

### 1.2 Конвертировать webpack.config.common.js -> rspack.config.common.js
- [ ] Заменить `require('webpack')` на `require('@rspack/core')`
- [ ] `NodePolyfillPlugin` — проверить совместимость с Rspack или заменить на `@rspack/plugin-node-polyfill` (если есть) или ручные fallback'и
- [ ] `NormalModuleReplacementPlugin` — встроен в Rspack (совместим)
- [ ] `DefinePlugin` — встроен в Rspack (совместим)
- [ ] `ProvidePlugin` — встроен в Rspack (совместим)
- [ ] `CleanWebpackPlugin` -> встроенный `output.clean: true` в Rspack
- [ ] `HTMLWebpackPlugin` -> `@rspack/plugin-html` или оставить html-webpack-plugin (совместим)
- [ ] `MiniCssExtractPlugin` -> встроенный CSS extract в Rspack (`experiments.css`)
- [ ] `WorkerUrlPlugin` — проверить совместимость, может потребовать замены на `new Worker(new URL(...), import.meta.url)` паттерн
- [ ] `BootloaderPlugin` (кастомный) — ревизия: прочитать `src/components/loader/webpack-loader.js`, адаптировать или заменить
- [ ] `esbuild-loader` -> встроенный SWC в Rspack (быстрее, нативный)

### 1.3 Конвертировать webpack.config.dev.js -> rspack.config.dev.js
- [ ] `webpack-merge` -> `@rspack/core` merge или вручную
- [ ] `ReactRefreshWebpackPlugin` -> `@rspack/plugin-react-refresh`
- [ ] Dev server конфиг — Rspack dev server API совместим

### 1.4 Конвертировать webpack.config.prod.js -> rspack.config.prod.js
- [ ] `TerserPlugin` -> встроенная минификация Rspack (SWC-based)
- [ ] `CssMinimizerPlugin` -> встроенная CSS минификация Rspack
- [ ] `WorkboxPlugin.InjectManifest` — оставить (совместим с Rspack)
- [ ] `NormalModuleReplacementPlugin` для Netlify mocks — совместим
- [ ] `splitChunks` — API идентичен webpack

### 1.5 Обновить package.json scripts
- [ ] `"start": "rspack serve --config=rspack.config.dev.js"`
- [ ] `"build": "rspack build --config=rspack.config.prod.js && yarn copy-public"`
- [ ] `"build-ipfs": "IPFS_DEPLOY=1 rspack build --config=rspack.config.prod.js --mode production"`

### 1.6 Обработать module rules
- [ ] `.tsx?/.jsx?` — SWC (встроен в Rspack, заменяет esbuild-loader)
- [ ] `.css` — встроенная обработка CSS в Rspack
- [ ] `.scss/.sass` — `sass-loader` (совместим) + встроенный CSS
- [ ] `.cozo` — `raw-loader` -> `asset/source` type (встроен)
- [ ] `.graphql/.gql` — `graphql-tag/loader` — проверить совместимость
- [ ] `.rn` — `asset/source` (уже используется, совместим)
- [ ] Шрифты, изображения, звуки — `asset/resource` (совместим)

### 1.7 Тестирование
- [ ] `yarn build` — production build проходит без ошибок
- [ ] `yarn start` — dev server стартует, HMR работает
- [ ] Ручное smoke-тестирование основных страниц
- [ ] Сравнить размер бандла с Фаза 0 бенчмарком
- [ ] Workers (SharedWorker, ServiceWorker) работают
- [ ] WASM модули (cozo, rune, transformers) загружаются

### 1.8 Зачистка
- [ ] Удалить `webpack.config.*.js` файлы
- [ ] Удалить webpack-зависимости из package.json
- [ ] Удалить неиспользуемые полифил-пакеты если Rspack их не требует
- [ ] Обновить Dockerfile (если webpack упоминается)

---

## Фаза 2: Node.js -> Deno 2 (рантайм разработки)

**Почему вторым:** после Rspack проект меньше зависит от Node-специфики.

### 2.1 Установить Deno 2
- [ ] `curl -fsSL https://deno.land/install.sh | sh`
- [ ] Создать `deno.json` конфиг (аналог tsconfig + package.json для Deno)

### 2.2 Настроить npm-совместимость
- [ ] `deno.json`: `"nodeModulesDir": "auto"` (Deno 2 поддерживает node_modules)
- [ ] Проверить что `deno install` (аналог `yarn install`) устанавливает все 330 пакетов
- [ ] Обработать возможные несовместимости (native addons, postinstall scripts)

### 2.3 Адаптировать скрипты
- [ ] `deno task start` вместо `yarn start`
- [ ] `deno task build` вместо `yarn build`
- [ ] `deno task lint` вместо `yarn lint`
- [ ] Задачи описать в `deno.json` -> `"tasks": {...}`

### 2.4 Адаптировать Rspack конфиги
- [ ] Rspack конфиги используют `require()` — конвертировать в ESM `import`
- [ ] `__dirname` -> `import.meta.dirname` (Deno)
- [ ] `process.env` -> `Deno.env.get()` в конфигах (или оставить через dotenv совместимость)
- [ ] `require.resolve()` для polyfill fallback'ов — заменить на import maps или прямые пути

### 2.5 Тестирование
- [ ] `deno task build` — build проходит
- [ ] `deno task start` — dev server работает
- [ ] Smoke-тест основных страниц

### 2.6 Обновить CI/CD
- [ ] `.github/workflows/` — заменить `node` setup на `deno` setup
- [ ] `netlify.toml` — обновить build command
- [ ] `netlify/prebuild/index.js` — адаптировать для Deno

### 2.7 Обновить Docker
- [ ] `Dockerfile`: `FROM denoland/deno:latest` вместо `FROM node:18.14`
- [ ] `docker-compose.yml`: убрать `NODE_OPTIONS`, обновить команды

---

## Фаза 3: Убрать Node-полифиллы из runtime-кода

**Почему третьим:** после перехода на Deno+Rspack можно чистить код.

### 3.1 Заменить Buffer на Uint8Array / Web API
Файлы с Buffer.from():
- [ ] `src/components/MusicalAddress/MusicalAddress.tsx` — `Buffer.from(sliceAddress)` -> `new TextEncoder().encode()`
- [ ] `src/features/ibc-history/tx/TracerTx.ts` — `Buffer.from(query).toString('base64')` -> `btoa()`
- [ ] `src/utils/utils.ts` — `Buffer.from(sha256(...))` -> `Uint8Array` от sha256
- [ ] `src/pages/Keys/ActionBar/actionBarConnect.tsx` — `Buffer.from(pubKey).toString('hex')` -> hex util
- [ ] `src/pages/teleport/hooks/useGetBalancesIbc.ts` — `Buffer.from(sha256(...))` -> Uint8Array
- [ ] `src/services/ipfs/utils/content.ts` — `Buffer.from(rawData)` -> Uint8Array
- [ ] `src/utils/ipfs/helpers.ts` — `Buffer.from(string)` -> TextEncoder
- [ ] `src/containers/portal/citizenship/ActionBar.tsx` — `Buffer.from(pubKey).toString('hex')` -> hex util
- [ ] `src/containers/portal/gift/ActionBarPortalGift.tsx` — `Buffer.from(message).toString('hex')` -> hex util
- [ ] Создать `src/utils/encoding.ts` с хелперами: `toHex()`, `fromHex()`, `toBase64()`, `fromBase64()`, `toBytes()`, `fromBytes()`

### 3.2 Убрать node-polyfill-webpack-plugin
- [ ] Удалить `NodePolyfillPlugin` из Rspack конфига
- [ ] Удалить `NormalModuleReplacementPlugin` для `node:` протокола
- [ ] Удалить resolve.fallback секцию (или оставить минимум если нужно для зависимостей)
- [ ] Удалить полифил-пакеты из package.json:
  - `buffer`, `process`, `crypto-browserify`, `stream-browserify`
  - `https-browserify`, `os-browserify`, `stream-http`, `constants-browserify`
  - `assert`, `browserify-zlib`, `path-browserify`

### 3.3 Разобраться с process.env в runtime-коде
- [ ] `src/constants/config.ts` — основной файл с `process.env.*`
  - Rspack DefinePlugin заменяет на compile-time значения, это ок
  - Но стоит явно типизировать: создать `src/env.d.ts` с типами env-переменных
- [ ] Остальные файлы с `process.env` — убедиться что всё заменяется DefinePlugin'ом

### 3.4 Тестирование
- [ ] Build проходит без полифиллов
- [ ] Все страницы с Buffer-зависимым кодом работают (Keys, Teleport, Portal, IBC)
- [ ] IPFS функции работают (content upload/download)
- [ ] Размер бандла уменьшился (полифиллы убраны)

---

## Фаза 4: Замена Jest -> Deno test

### 4.1 Миграция тестов
- [ ] Инвентаризация: найти все `*.test.ts` / `*.test.tsx` / `*.spec.ts` файлы
- [ ] Заменить `import { describe, it, expect } from '@jest/globals'` на Deno test API
  - `Deno.test()` или `import { describe, it } from "jsr:@std/testing/bdd"`
  - `import { expect } from "jsr:@std/expect"`
- [ ] Убрать `jest.config.js`
- [ ] Обновить `deno.json` с test настройками

### 4.2 Зачистка
- [ ] Удалить Jest зависимости: `jest`, `@types/jest`, `ts-jest`, `@testing-library/*`
- [ ] Удалить `jest.config.js`
- [ ] Обновить `tsconfig.json` — убрать `jest` из types

---

## Фаза 5: Финализация и оптимизация

### 5.1 Зачистка package.json
- [ ] Удалить все неиспользуемые после миграции зависимости
- [ ] Убрать `engines.node`, `packageManager: yarn`
- [ ] Удалить `yarn.lock` (если полностью на Deno)

### 5.2 Обновить документацию
- [ ] `README.md` — обновить инструкции установки и запуска
- [ ] `CLAUDE.md` — обновить стек
- [ ] `reference/roadmap.md` — отметить выполненное

### 5.3 Финальные бенчмарки
- [ ] Сравнить время cold build: Webpack vs Rspack
- [ ] Сравнить время dev server startup
- [ ] Сравнить размер бандла (без полифиллов)
- [ ] Сравнить размер node_modules / vendor

### 5.4 Оценить следующий шаг
- [ ] Rspack -> Vite? (если Rspack не устраивает)
- [ ] Deno встроенный линтер вместо ESLint?
- [ ] Deno fmt вместо Prettier?
- [ ] Storybook — проверить совместимость с Deno

---

## Риски и митигация

| Риск | Вероятность | Митигация |
|------|------------|-----------|
| npm-пакеты несовместимы с Deno 2 | Средняя | `nodeModulesDir: auto` решает большинство случаев |
| WASM модули не грузятся в Rspack | Низкая | Rspack поддерживает WASM как webpack |
| WorkerUrlPlugin не работает в Rspack | Средняя | Переписать на `new Worker(new URL(...), import.meta.url)` |
| BootloaderPlugin (кастомный) ломается | Высокая | Переписать под Rspack API (минимальный плагин) |
| Netlify build ломается | Средняя | Deno доступен в Netlify через `DENO_INSTALL` |
| Время миграции затягивается | Средняя | Каждая фаза независима, можно остановиться |

---

## Порядок выполнения (рекомендация)

```
Фаза 0 (подготовка)     ~1 день
  |
Фаза 1 (Rspack)         ~3-5 дней   <- самый большой win
  |
Фаза 3 (полифиллы)      ~2 дня      <- можно параллельно с Фазой 2
  |
Фаза 2 (Deno)           ~3-5 дней
  |
Фаза 4 (Jest -> Deno)   ~1-2 дня
  |
Фаза 5 (зачистка)       ~1 день
```

Фазы 2 и 3 можно менять местами или делать параллельно.
