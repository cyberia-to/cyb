# План: убрать Node.js зависимости (4 фазы)

## Контекст

После миграции на Deno 2 + Rspack, Node.js всё ещё требуется как рантайм для ESLint, serve, и как поставщик полифиллов (buffer, crypto, stream) для сторонних пакетов. Цель — сократить зависимость от Node до единственной точки (rspack), убрав всё остальное.

---

## Порядок: Фаза 2 → 4 → 3 → 1

Сначала простое и безопасное, потом самое большое (Biome).

---

## Фаза 2: Удалить неиспользуемые зависимости

5+ пакетов в package.json НЕ ИМПОРТИРУЮТСЯ нигде в src/.

### 2.1 Удалить из dependencies:
- `web3@1.2.4` — 0 импортов в src/
- `ethers@5.0.26` — 0 импортов (только тип ExternalProvider в window.d.ts)
- `@ethersproject/providers@5.7.2` — только тип в window.d.ts
- `secp256k1@3.7.1` — 0 импортов
- `ripemd160@2.0.2` — 0 импортов
- `ledger-cosmos-js@2.1.7` — 0 импортов
- `web3-utils@4.2.1` — 0 импортов

### 2.2 Заменить `@ethersproject/providers` в `src/types/window.d.ts`:
```typescript
// Вместо import { ExternalProvider } from '@ethersproject/providers'
interface EthereumProvider {
  request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
  on?: (event: string, handler: (...args: unknown[]) => void) => void;
  isMetaMask?: boolean;
}

declare global {
  interface Window extends KeplrWindow {
    ethereum?: EthereumProvider;
    cyb: any;
  }
}
```

### 2.3 Заменить `serve` в deno.json:
```
"serve": "deno run --allow-net --allow-read jsr:@std/http/file-server build"
```
Удалить `serve` из devDeps.

### Верификация
```bash
yarn install && deno task build
```

**Файлы:** `package.json`, `src/types/window.d.ts`, `deno.json`

---

## Фаза 4: tsconfig + мелочи

### 4.1 Заменить `NodeJS.Timeout` → `ReturnType<typeof setTimeout>` (2 файла):
- `src/services/ipfs/utils/utils-ipfs.ts:82`
- `src/services/QueueManager/QueueManager.test.ts:18`

### 4.2 Убрать `"types": ["node"]` из `tsconfig.json`
process.env продолжит работать — rspack DefinePlugin заменяет на литералы.

### 4.3 Убрать мёртвый `process.env.REACT_APP_OPENAI_API_KEY`
В `src/features/sense/ui/VoiceInteraction/VoiceInteraction.tsx` — не определён в DefinePlugin, всегда undefined.

### Верификация
```bash
deno task build
```

**Файлы:** `tsconfig.json`, 2 файла с NodeJS.Timeout, VoiceInteraction.tsx

---

## Фаза 3: Сократить Node полифиллы в rspack

12 полифиллов в `rspack.config.common.js` `resolve.fallback`. Большинство нужны только транзитивно через node_modules.

### 3.1 Метод: заменить `require.resolve(...)` на `false` по одному, проверять билд

Кандидаты на удаление (не импортируются в src/ напрямую):
1. `assert` → `false`
2. `https` (https-browserify) → `false`
3. `os` (os-browserify) → `false`
4. `http` (stream-http) → `false`
5. `constants` (constants-browserify) → `false`
6. `crypto` (crypto-browserify) → `false`

### 3.2 Остаются (нужны):
- `buffer` — @cosmjs/crypto и космос пакеты
- `stream` (stream-browserify) — IPFS/libp2p
- `readable-stream` — NormalModuleReplacementPlugin для node:stream
- `process` — ProvidePlugin для сторонних пакетов

### 3.3 Удалить пакеты из package.json
Те что прошли `false` тест: ожидаемо assert, https-browserify, os-browserify, stream-http, constants-browserify, возможно crypto-browserify.

### Верификация
```bash
deno task build && deno task build-ipfs
```

**Файлы:** `rspack.config.common.js`, `package.json`

---

## Фаза 1: ESLint + Prettier → Biome

Заменить ESLint (14 пакетов) + Prettier на Biome — Rust бинарник, lint + format.

### 1.1 Установить Biome
```bash
brew install biome
# или в package.json devDeps: "@biomejs/biome"
```

### 1.2 Создать `biome.json`

Перенести из `.eslintrc` + `.prettierrc`:

**Форматирование (из .prettierrc):**
- singleQuote → `"quoteStyle": "single"`
- tabWidth: 2 → `"indentWidth": 2`
- trailingComma: es5 → `"trailingCommas": "es5"`
- bracketSpacing: true
- useTabs: false → `"indentStyle": "space"`
- linebreak unix → `"lineEnding": "lf"`

**Линтинг (из .eslintrc):**
- `noUnusedImports: "error"` (unused-imports/no-unused-imports)
- `noUnusedVariables: "error"` (@typescript-eslint/no-unused-vars)
- `noRestrictedImports` с путями react-redux и @cybercongress/gravity
- `noDebugger: "warn"`
- `noParameterAssign: "error"` (с исключением для redux state — через biome-ignore)
- `noNestedTernary: "off"`

**Правила без эквивалента в Biome (просто теряем):**
- `import/no-unused-modules` — нет аналога
- `import/no-extraneous-dependencies` — нет аналога
- `jsx-a11y/*` — частичная поддержка
- Airbnb стиль — Biome покрывает основное

### 1.3 Конвертировать eslint-disable → biome-ignore (322 директивы)

Массовая замена:
- `eslint-disable-next-line import/no-unused-modules` (68) → удалить (нет аналога)
- `eslint-disable-next-line react-hooks/exhaustive-deps` (67) → `biome-ignore lint/correctness/useExhaustiveDependencies`
- `eslint-disable-next-line no-await-in-loop` (30) → удалить (нет аналога)
- `eslint-disable-next-line no-restricted-syntax` (27) → удалить (нет аналога)
- Остальные (130) → сконвертировать по одному или через `biome migrate eslint`

### 1.4 Обновить deno.json
```json
"lint": "biome check src/",
"lint:fix": "biome check --write src/",
"format": "biome format --write src/"
```

### 1.5 Обновить .vscode/settings.json
Убрать eslint, добавить biome extension.

### 1.6 Удалить файлы:
- `.eslintrc`
- `.eslintignore`
- `.prettierrc`

### 1.7 Удалить из package.json (16 пакетов):
- `eslint`, `eslint-config-airbnb`, `eslint-config-prettier`
- `eslint-import-resolver-typescript`, `eslint-import-resolver-alias`
- `eslint-plugin-import`, `eslint-plugin-jsx-a11y`, `eslint-plugin-prettier`
- `eslint-plugin-react`, `eslint-plugin-react-hooks`, `eslint-plugin-typescript`
- `eslint-plugin-unused-imports`
- `@typescript-eslint/eslint-plugin`, `@typescript-eslint/parser`
- `prettier`

### Верификация
```bash
biome check src/    # lint проходит
deno task build     # билд проходит
```

**Файлы:** `biome.json` (новый), `deno.json`, `.vscode/settings.json`, `package.json`, удалить `.eslintrc`, `.eslintignore`, `.prettierrc`, 100+ файлов с eslint-disable

---

## Итог

| До | После |
|----|-------|
| ESLint + 14 плагинов + Prettier (Node.js) | Biome (Rust бинарник) |
| 12 Node полифиллов | 3-4 (buffer, stream, process) |
| 7 неиспользуемых deps (web3, ethers, etc.) | 0 |
| `"types": ["node"]` в tsconfig | Убрано |
| serve (Node.js) | deno std/http |

Node.js остаётся ТОЛЬКО для rspack build/serve.
