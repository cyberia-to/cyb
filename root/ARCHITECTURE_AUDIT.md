# Architecture Audit — cyb-ts

**Date:** 2026-02-19

This document consolidates five independent audit reports covering the full `cyb-ts` codebase. Each section was produced by a dedicated audit agent. No content has been summarised or omitted.

---

## Table of Contents

1. [Project Structure Audit](#1-project-structure-audit)
2. [Dependency Audit](#2-dependency-audit)
3. [React Component Architecture Audit](#3-react-component-architecture-audit)
4. [Services & Backend Architecture Audit](#4-services--backend-architecture-audit)
5. [Security & Performance Audit](#5-security--performance-audit)

---

## 1. Project Structure Audit

# cyb-ts Codebase Audit — Full Report

**Total TS/JS source files: 1,052** (347 `.ts`, 518 `.tsx`, 52 `.js`, 117 `.jsx` — still ~16% unconverted JSX/JS)

---

## 1. Directory Tree — `src/` (2 levels, file counts)

```
src/                         (5 files at root)
├── components/              1 root file, 71 subdirs
│   ├── PascalCase dirs (40): AmountDenom, ArrowToggle, Button, CIDResolver, DonutChart...
│   └── camelCase dirs (31): account, actionBar, appMenu, atoms, battery, btnGrd, buttons...
├── constants/               9 files, 0 subdirs
├── containers/              0 root files, 29 subdirs
│   ├── Objects (1), Search (6), application (5), blok (3), brain (4)
│   ├── energy (3), forceGraph (2), gol (2), governance (13), help (2)
│   ├── home (3), ipfs (13), ipfsSettings (1), market (6), mint (6)
│   ├── movie (2), nebula (2), oracle (2), parameters (2), portal (8+)
│   ├── sigma (3), story (3), taverna (2), temple (2), testKeplre (1)
│   ├── trollBox (1), txs (6), warp (6), wasm (2)
├── contexts/                11 files, 3 subdirs (backend, chain, scripting)
├── features/                0 root files, 13 subdirs
│   ├── TimeFooter (2), TimeHistory (3), adviser (4), cyberlinks (0+), cybernet (3)
│   ├── ibc-history (6), ipfs (0+), particle (2), passport (3), rank (1)
│   ├── sense (2), staking (2), studio (6)
├── fonts/                   1 file
├── generated/               1 file (graphql.ts — 15,589 lines, auto-generated)
├── hocs/                    2 files (withAccount.tsx, withIpfsAndKeplr.tsx)
├── hooks/                   27 files, 5 subdirs (contract, dom, governance, react, warp)
├── i18n/                    1 file
├── image/                   112 files, 3 subdirs (new_icons, statusTx, temple)
├── layouts/                 3 files, 1 subdir (ui)
├── pages/                   0 root files, 11 subdirs
│   ├── Brain (2), Hub (1), Keys (3), Portal (2), Settings (1)
│   ├── Social (3), Sphere (2+deep), oracle (3+), redirects (1)
│   ├── robot (3+deep), teleport (3+deep)
├── redux/                   3 files, 3 subdirs (actions, features, reducers)
├── routing/                 1 file
├── services/                0 root files, 14 subdirs
│   ├── CozoDb (4), QueueManager (6), backend (deep), community (3)
│   ├── graphql (1), ipfs (2), lcd (3), neuron (2), passports (1)
│   ├── relayer (7), scripting (5), service-worker (1), soft.js (2+), transactions (1)
├── sounds/                  5 files
├── stories/                 2 files (Colors.mdx, Introduction.mdx — Storybook root docs)
├── style/                   7 files, 1 subdir (libs)
├── types/                   15 files
├── utils/                   20 files, 9 subdirs
└── websockets/              2 files
```

---

## 2. Largest Files by Line Count (Top 20)

| Rank | Lines | File |
|------|-------|------|
| 1 | **15,589** | `src/generated/graphql.ts` *(auto-generated — do not edit)* |
| 2 | 2,427 | `src/utils/finalResultGoL.js` |
| 3 | 1,266 | `src/features/adviser/Adviser/typeit.js` |
| 4 | 767 | `src/containers/portal/gift/ActionBarPortalGift.tsx` |
| 5 | 726 | `src/containers/txs/Activites.jsx` |
| 6 | 685 | `src/containers/portal/gift/index.tsx` |
| 7 | 670 | `src/containers/portal/citizenship/index.tsx` |
| 8 | 625 | `src/components/TableTxsInfinite/component/RenderValue.tsx` |
| 9 | 574 | `src/features/sense/redux/sense.redux.ts` |
| 10 | 499 | `src/services/QueueManager/QueueManager.ts` |
| 11 | 470 | `src/features/ibc-history/tx/TracerTx.ts` |
| 12 | 456 | `src/pages/Sphere/pages/components/ActionBarContainer/ActionBarContainer.tsx` |
| 13 | 443 | `src/utils/utils.ts` |
| 14 | 441 | `src/containers/Search/ActionBarContainer.tsx` |
| 15 | 437 | `src/features/cybernet/ui/pages/Subnet/tabs/SubnetNeurons/SubnetNeuronsTable/SubnetNeuronsTable.tsx` |
| 16 | 430 | `src/containers/warp/Warp.tsx` |
| 17 | 425 | `src/containers/portal/release/index.tsx` |
| 18 | 412 | `src/pages/robot/_refactor/account/actionBar.tsx` |
| 19 | 400 | `src/pages/teleport/swap/swap.tsx` |
| 20 | 378 | `src/features/ipfs/Drive/index.tsx` |

Notable: `finalResultGoL.js` (2,427 lines) is only referenced by `src/containers/gol/getGolHooks.jsx` — a near-dead GoL module. `typeit.js` is a vendored third-party library copy sitting inside a feature dir.

---

## 3. Duplicate/Redundant Directories

### A. containers/ vs pages/ — same concepts, split across both

| Concept | containers/ | pages/ | Notes |
|---------|-------------|--------|-------|
| Portal | `containers/portal/` (70+ files) | `pages/Portal/Map/` (2 files) | containers/portal is the real impl; pages/Portal is just a Map subpage |
| Brain | `containers/brain/` (4 files) | `pages/Brain/Brain.tsx` (2 files) | pages/Brain is the new refactored shell |
| Oracle | `containers/oracle/` (2 .jsx files) | `pages/oracle/` (14 files) | pages/oracle is the new version; containers/oracle is legacy |
| Sigma | `containers/sigma/` (hooks + context) | `features/cybernet/ui/pages/Sigma/` | Sigma concept split 3 ways |

### B. IPFS split across 4 locations

```
src/containers/ipfs/               — IPFS content viewer (ipfs.tsx + hooks)
src/containers/ipfsSettings/       — 1 stale file (infoIpfsNode.tsx), 0 root files
src/features/ipfs/                 — Drive UI + hooks + ipfsSettings (active)
src/features/ipfs/ipfsSettings/    — The authoritative settings impl
```
`containers/ipfsSettings/ipfsComponents/infoIpfsNode.tsx` is a lone duplicate of the same file in `features/ipfs/ipfsSettings/ipfsComponents/infoIpfsNode.tsx`.

### C. Number-formatting — 4 separate components doing similar work

```
src/components/formatNumber/formatNumber.js       — old .js version
src/components/FormatNumberTokens/               — newer .tsx version
src/components/numberCurrency/index.jsx          — another variant
src/components/AmountDenom/AmountDenom.tsx        — yet another
```

### D. `pages/Sphere/pages/containers/` — containers INSIDE pages

`src/pages/Sphere/pages/containers/Heroes/` and `.../HeroDetails/` — the `containers/` subdirectory naming pattern is used recursively inside a page, creating a 5-level deep hierarchy that replicates the top-level `containers/` concept locally.

### E. services/ fragmentation — multiple thin LCD wrappers

```
src/services/community/lcd.ts
src/services/passports/lcd.ts
src/services/transactions/lcd.tsx
src/services/lcd/                  — separate lcd utils/mapping
```
Four separate `lcd.ts` files across four service directories, each wrapping similar Cosmos LCD calls with no shared base.

### F. `features/cybernet/ui/pages/` — a full pages tree inside a feature

`src/features/cybernet/ui/pages/` contains 9 full page components (Main, Subnet, Delegates, Delegate, Subnets, Verses, Verse, MyStake, Sigma). This is the `pages/` pattern duplicated inside a feature.

---

## 4. Dead Code Indicators

### Containers with 0 external importers (only in router.tsx)

| Container | Import count | Location | Assessment |
|-----------|-------------|----------|------------|
| `containers/ipfsSettings` | **0** | nowhere | Dead — superseded by `features/ipfs/ipfsSettings` |
| `containers/trollBox` | 1 | `router.tsx` only | Near-dead — commented-out pubsub, hardcoded test data |
| `containers/testKeplre` | 1 | `router.tsx` only | Dead — explicitly a test fixture with `/* eslint-disable */` |
| `containers/movie` | 1 | `router.tsx` only | Likely dead route |
| `containers/story` | 1 | `router.tsx` only | Likely dead route |
| `containers/blok` | 1 | `router.tsx` only | Only used as route, no feature imports |

### Large files used only within dead modules

- `src/utils/finalResultGoL.js` (2,427 lines) — only imported by `containers/gol/getGolHooks.jsx`, which itself is only imported from `pages/robot/Layout/useMenuCounts.tsx`. GoL (Game of Links) was a past campaign — this data file is essentially frozen.

### Features with only 1 importer

| Feature | Importer count | Single importer |
|---------|---------------|-----------------|
| `features/TimeFooter` | 1 | `src/layouts/Main.tsx` |
| `features/rank` | 1 | `src/utils/search/utils.ts` (contains TODO: "move rank to features/rank") |
| `features/studio` | 1 | `src/router.tsx` |

### Empty stub directories (0 files at root, only nested subdirs)

```
src/components/atoms/          — only has Triangle/ and glass/ subdirs; no index
src/components/appMenu/        — only has CircularMenu/, MobileMenu/, SubMenu/; no index
src/components/search/         — only has Spark/ subdir; no index
src/components/buttons/        — only has AddFile/, ButtonIcon/, ButtonsGroup/; no index
src/features/cyberlinks/       — 0 root files
src/features/ipfs/             — 0 root files
src/pages/oracle/              — 0 root files
src/pages/Portal/              — 0 root files (only Map/ subdir)
```

### `_refactor` directory in production source

`src/pages/robot/_refactor/` — 20 files with a mix of `.jsx`/`.tsx`/`.ts`, including a file named `useGetTsxByAddress.legacy.ts`. This directory is a stale in-progress refactor committed into the active source tree.

---

## 5. Naming Inconsistencies

### File count by convention (TS/JS files only)

| Convention | Count | Examples |
|-----------|-------|---------|
| PascalCase | **403** | `ActionBar.tsx`, `Sphere.tsx`, `QueueManager.ts` |
| camelCase | **403** | `actionBar.tsx`, `analytics.tsx`, `utils.ts` |
| kebab-case | **6** | `js-ipfs.ts`, `service-worker.ts`, `utils-ipfs.ts`, `polling-status-subscription.ts`, `webpack-loader.js`, `QueueManager.fake-timer.test.ts` |

### Identical concept, different casing in different directories

```
src/containers/portal/components/ActionBar/   — PascalCase dir
src/components/actionBar/                     — camelCase dir

src/containers/sigma/                         — camelCase dir
src/features/cybernet/ui/pages/Sigma/         — PascalCase dir

src/containers/portal/pasport/                — typo: "pasport" (missing 's')
src/features/passport/                        — correct spelling
src/services/passports/                       — correct, plural
```

### Directory naming — mixed PascalCase vs camelCase at same hierarchy level

Within `src/components/` alone, 40 dirs are PascalCase and 31 are camelCase with no rule separating them:
```
PascalCase: AmountDenom, ArrowToggle, BandwidthBar, Button, CIDResolver ...
camelCase:  account, actionBar, appMenu, atoms, battery, btnGrd ...
```

Within `src/pages/`:
```
PascalCase: Brain, Hub, Keys, Portal, Settings, Social, Sphere
lowercase:  oracle, redirects, robot, teleport
```

### `.jsx` files still present (117 total) — incomplete migration

Notable unconverted files:
```
src/containers/txs/Activites.jsx        (726 lines — note: "Activites" is also a typo)
src/containers/portal/RocketSpacePussy.jsx
src/containers/portal/mainPortal.jsx
src/containers/portal/stateComponent/index.jsx  (and 7 more JSX in stateComponent/)
src/containers/portal/components/...  (8 more .jsx files)
src/containers/oracle/index.jsx
src/containers/oracle/useGetStatisticsCyber.jsx
src/pages/robot/_refactor/account/tabs/heroes.jsx
```

### Typos in directory/file names

| Typo | Location | Should be |
|------|----------|-----------|
| `pasport` | `src/containers/portal/pasport/` | `passport` |
| `Activites` | `src/containers/txs/Activites.jsx` | `Activities` |
| `testKeplre` | `src/containers/testKeplre/` | `testKeplr` or delete |
| `AvataImgIpfs` | `src/containers/portal/components/avataIpfs/AvataImgIpfs.tsx` | `AvatarImgIpfs` |
| `avataIpfs` | `src/containers/portal/components/avataIpfs/` | `avatarIpfs` |

---

## 6. Circular Dependency Risks

### Direct violations of dependency direction

The expected clean dependency flow is:
`services -> hooks -> components -> features -> pages -> containers (routes)`

Actual violations found:

**`features/` importing from `pages/`** (features should not depend on pages):
```
src/features/studio/ActionBar.tsx
  -> imports InputMemo from 'src/pages/teleport/components/Inputs'

src/features/sense/ui/SenseViewer/SenseViewer.tsx
  -> imports useRobotContext from 'src/pages/robot/robot.context'

src/features/sense/ui/Sense.tsx
  -> imports useRobotContext from 'src/pages/robot/robot.context'

src/features/sense/ui/SenseRoutingWrapper.tsx
  -> imports useRobotContext from 'src/pages/robot/robot.context'

src/features/sense/ui/ActionBar/ActionBar.tsx
  -> imports ibcDenomAtom from 'src/pages/teleport/bridge/bridge'

src/features/staking/getHeroesHook.ts
  -> imports BondStatusKey from 'src/pages/Sphere/types/bondStatus'

src/features/cybernet/ui/pages/Sigma/Sigma.tsx
  -> imports AccountInput from 'src/pages/teleport/components/Inputs'

src/features/cybernet/ui/pages/Main/Banner/Banner.tsx
  -> imports TypingText from 'src/containers/temple/pages/play/PlayBanerContent'

src/features/cyberlinks/graph/GraphActionBar/GraphActionBar.tsx
  -> imports useGraphLimit from 'src/pages/robot/Brain/useGraphLimit'

src/features/cyberlinks/GraphNew/GraphNew.tsx
  -> imports useGraphLimit from 'src/pages/robot/Brain/useGraphLimit'
```

**`components/` importing from `containers/`** (components should be self-contained):
```
src/components/HydrogenBalance/HydrogenBalance.tsx
  -> imports useGetBalanceBostrom from 'src/containers/sigma/hooks'

src/components/SearchItem/searchItem.tsx
  -> imports LinksType from 'src/containers/Search/types'

src/components/search/Spark/Meta/Meta.tsx
  -> imports PREFIXES from 'src/containers/ipfs/components/metaInfo'

src/components/search/Spark/Meta/Links/Links.tsx
  -> imports LinksTypeFilter from 'src/containers/Search/types'

src/components/search/Spark/Spark.tsx
  -> imports LinksType from 'src/containers/Search/types'

src/components/search/Spark/LeftMeta/Creator/Creator.tsx
  -> imports useGetCreator from 'src/containers/ipfs/hooks/useGetCreator'

src/components/ContentItem/contentItem.tsx
  -> imports LinksType from 'src/containers/Search/types'

src/components/account/account.tsx
  -> imports AvataImgIpfs from 'src/containers/portal/components/avataIpfs'
```

**`hooks/` importing from `pages/`**:
```
src/hooks/useHub.ts
  -> imports setChannels/setNetworks/setTokens from 'src/pages/Hub/redux/hub'
```

**`containers/` importing from `pages/`** (bidirectional risk):
```
src/containers/energy/index.tsx        -> 'src/pages/...'
src/containers/sigma/index.tsx         -> 'src/pages/...'
src/containers/warp/Warp.tsx           -> 'src/pages/...'
src/containers/temple/Temple.tsx       -> 'src/pages/...'
src/containers/taverna/index.tsx       -> 'src/pages/...'
(and 4 more containers)
```

**`pages/` importing from `containers/`** (bidirectional risk):
```
src/pages/Sphere/Sphere.context.tsx    -> containers/sigma/hooks/utils
src/pages/robot/Layout/useMenuCounts.tsx -> containers/gol/getGolHooks
src/pages/robot/Robot.tsx              -> containers/gol/table
```

This creates multiple **bidirectional cycles** between `containers/` and `pages/`, and between `features/` and `pages/`.

---

## 7. Root Configuration Files

| File | Size | Last Modified | Status |
|------|------|--------------|--------|
| `rspack.config.common.js` | 6,256 bytes | Feb 19 2026 | **Active** — primary bundler config |
| `rspack.config.dev.js` | 1,498 bytes | Feb 19 2026 | **Active** — dev server |
| `rspack.config.prod.js` | 1,386 bytes | Feb 19 2026 | **Active** — prod build |
| `package.json` | 8,075 bytes | Feb 19 2026 | **Active** |
| `tsconfig.json` | 570 bytes | Feb 19 2026 | **Active** |
| `deno.json` | 624 bytes | Feb 19 2026 | **Active** — used to run rspack tasks |
| `docker-compose.yml` | 624 bytes | Feb 19 2026 | **Active** |
| `netlify.toml` | 48 bytes | Sep 8 2025 | **Active** (tiny — only a prebuild plugin reference) |
| `codegen.ts` | 687 bytes | Sep 8 2025 | **Active** — GraphQL type generation |
| `postcss.config.js` | 110 bytes | Sep 8 2025 | **Active** — postcss-preset-env |
| `commitlint.config.js` | 117 bytes | Sep 8 2025 | **Active** — enforces scoped commit messages |

### Notable: `webpack/` directory — partially dead

`webpack/BundleInfoPlugin.js` exists but the `BootloaderPlugin` in `rspack.config.common.js` is commented out with:
```js
// TODO: BootloaderPlugin uses deep webpack internals (compilation.addChunk, chunk.moveModule)
// that are not compatible with Rspack. Disabled until rewritten.
```
The project **migrated from webpack to rspack**, but `package.json` still lists `@storybook/react-webpack5`, `eslint-import-resolver-webpack`, and `webpack-bundle-analyzer` as devDependencies — these are webpack-era remnants. `BundleInfoPlugin.js` is referenced in `rspack.config.prod.js` (still active), but the main `BootloaderPlugin` (webpack-specific) is dead.

---

## Summary of Highest-Priority Issues

1. **`containers/ipfsSettings/`** — has exactly 1 file, 0 importers, is superseded by `features/ipfs/ipfsSettings/`. Safe to delete.

2. **`containers/testKeplre/` and `containers/trollBox/`** — test fixtures / commented-out prototypes committed into production routing. Should be removed.

3. **8 `components/` files importing from `containers/`** — breaks component reusability. The types (`LinksType`, `LinksTypeFilter`) should be extracted to `src/types/`.

4. **10 `features/` files importing from `pages/`** — `features/sense` is tightly coupled to `pages/robot/robot.context` (3 files). `features/staking` imports a type from `pages/Sphere/types/`. These should be inverted.

5. **`pages/robot/_refactor/`** — in-progress refactor in production source, including a `.legacy.ts` file and `.jsx` files that predate the migration.

6. **`src/utils/finalResultGoL.js`** (2,427 lines) — frozen Game of Links data used only via the near-dead `gol` container. Candidate for archival.

7. **`containers/` vs `pages/` for oracle and brain** — two competing implementations exist simultaneously with no clear deprecation path.

8. **`services/soft.js/`** — a directory literally named `soft.js` containing `.ts` files. The `.js` in the name is a confusing leftover from when the package was called `soft.js`.


---

## 2. Dependency Audit

# Dependency Audit: cyb-ts

**Codebase:** 1,053 source files — `node_modules` total: **3.3 GB**

---

## 1. UNUSED DEPENDENCIES

### In `dependencies` (runtime — confirmed unused in `src/`)

| Package | Notes |
|---|---|
| `@ai-sdk/openai` | Zero imports in src. `@openrouter/ai-sdk-provider` + `ai` is used instead in `LLMSpark.tsx` |
| `@cosmjs/cosmwasm-stargate` | Zero imports in src. `@cosmjs/stargate` is heavily used instead |
| `big.js` | Zero imports in src. `bignumber.js` is used in 62+ files |
| `blockstore-core` | Zero imports in src (only pulled transitively by helia) |
| `datastore-core` | Zero imports in src (only pulled transitively) |
| `detectrtc` | Zero imports in src at all |
| `ethers` | Only `window.d.ts` type declaration — not actually imported in app code |
| `web3` | `1.2.4` installed, zero runtime imports in src |
| `web3-utils` | Zero imports in src |
| `secp256k1` | Zero imports in src |
| `ripemd160` | Zero imports in src |
| `query-string` | Zero imports in src (v6, very old) |
| `readable-stream` | Only referenced in rspack fallback config — not directly imported in src |
| `redux-observable` | Zero imports in src (RC release `3.0.0-rc.2` — never left RC) |
| `worker-url` | Zero imports via `from 'worker-url'`; workers use native `new URL('./worker.ts', import.meta.url)` instead |
| `read-chunk` | Zero imports in src |
| `ipfs-http-client` | Zero imports in src (superseded by `kubo-rpc-client` which IS used) |
| `videostream` | Only in `VideoPlayer.tsx.inactive` (disabled file) |
| `sinon` | Zero test imports in src — a test mocking library with no usages |
| `ledger-cosmos-js` | Zero imports in src (`@ledgerhq/hw-transport-webusb` is used) |
| `long` | Only 2 files: `msgs.ts` and `actionBar.bridge.tsx` — worth confirming it isn't pulled via `cosmjs-types` already |
| `crypto` | Zero direct imports in src; used only as a polyfill alias in rspack config (`crypto-browserify`) |
| `cjs-to-es6` | A CJS→ES6 migration CLI tool, not a runtime library. Zero imports in src |

### In `devDependencies` (build-time — confirmed unused)

| Package | Notes |
|---|---|
| `swr` | Zero imports in src at all; `@tanstack/react-query` is used everywhere instead |
| `it-all` | Zero imports in src |
| `uint8arrays` | Zero imports in src (used by transitively by ipfs packages) |
| `history` | v4.10.1 installed; zero `from 'history'` imports in src — react-router v6 bundles its own history |
| `esbuild-loader` | Listed but project has migrated to rspack with `builtin:swc-loader`; never referenced in rspack configs |
| `redux-devtools-extension` | Zero imports in src — RTK's `configureStore` handles devtools natively |
| `react-iframe` | Not imported in src via `from 'react-iframe'`; a custom `<Iframe>` component in `src/components/Iframe` is used |
| `react-json-editor-ajrm` | Not imported via package name in src |
| `@babel/helper-compilation-targets` | Not directly referenced; a transitive dep of @babel/* |
| `webpack-merge` | Used in rspack configs (`require('webpack-merge')`), but `webpack-bundle-analyzer` and `webpack-merge` are webpack packages used with rspack |
| `prop-types` | Zero imports in src — TypeScript project, prop-types not used |

---

## 2. DEPENDENCIES IN THE WRONG SECTION

### Should be in `devDependencies` (currently in `dependencies`)

These are ESLint, type, storybook, build, or test packages that should never be in the production bundle:

| Package | Why it's wrong |
|---|---|
| `eslint-import-resolver-alias` | ESLint resolver plugin — pure dev tool |
| `eslint-plugin-react-hooks` | ESLint plugin — pure dev tool |
| `eslint-plugin-unused-imports` | ESLint plugin — pure dev tool |
| `@storybook/addon-designs` | Storybook addon — dev/build only |
| `sinon` | Test mocking library |
| `cjs-to-es6` | One-time migration CLI tool |
| `raw-loader` | Webpack/rspack loader — build tool |
| `dotenv` | Used only in `rspack.config.common.js` at build time, not in runtime src |
| `@types/file-saver` | TypeScript type-only package |
| `@types/history` | TypeScript type-only package |
| `@types/ramda` | TypeScript type-only package |
| `@types/react-dom` | TypeScript type-only (and `react-dom` v18 does NOT ship types itself, but this should be devDep) |
| `@types/react-router-dom` | TypeScript type-only; also `react-router-dom` v6 already ships its own types — this may be redundant |

### Should be in `dependencies` (currently in `devDependencies`)

| Package | Why it's wrong |
|---|---|
| `axios` | Used in 12+ runtime src files (`utils/axios.ts`, `polling-status-subscription.ts`, etc.) |
| `uuid` | Used in 10+ runtime src files (imported as `{ v4 as uuidv4 }`) |
| `cosmjs-types` | Used in 5+ runtime src files for type imports (`Coin`, `MsgSend`, `DelegationResponse`) |
| `redux` | Used in runtime `redux/types.d.ts` and `redux/features/pocket.ts` (imports `Dispatch`, `AnyAction`) |
| `@keplr-wallet/types` | Used in `types/window.d.ts`, `utils.ts`, `signerClient.tsx`, `relay.ts` — runtime type dependency |

---

## 3. DUPLICATE FUNCTIONALITY

### Multiple IPFS client implementations (3 parallel implementations)
All three live under `src/services/ipfs/node/impl/`:
- `js-ipfs.ts` — uses `ipfs-core` (deprecated, replaced by Helia)
- `kubo.ts` — uses `kubo-rpc-client` (connects to external Kubo daemon)
- `helia.ts` — uses `helia` + `@helia/unixfs` + `@libp2p/*` (in-browser node)

Three separate IPFS packages (`ipfs-core`, `kubo-rpc-client`, `helia`) are all installed and maintained simultaneously. The `js-ipfs`/`ipfs-core` path is the legacy one.

### Multiple AI/LLM SDKs with overlapping purpose
- `openai` (7.2 MB) — installed but **zero imports** in src
- `@ai-sdk/openai` — installed but **zero imports** in src
- `ai` (Vercel AI SDK) — used only in 1 file (`LLMSpark.tsx`)
- `@openrouter/ai-sdk-provider` — used only in 1 file (`LLMSpark.tsx`)
- `@xenova/transformers` — used in 1 file (`mlApi.ts`)

Three separate AI packages installed for one feature file.

### Multiple graph visualization libraries
- `react-force-graph` (21 MB) — used in `CyberlinksGraph.tsx`
- `@cosmograph/react` (5.9 MB) — used in `GraphNew.tsx`

Two distinct force-graph libraries coexist.

### Multiple state management layers
- `@reduxjs/toolkit` + `react-redux` (Redux, used in 31 files) — the primary store
- `@tanstack/react-query` (used in 55 files) — server-state / async queries
- Redux `redux-observable` with `rxjs` — also present but `redux-observable` has zero src imports (RC release, never shipped stable)
- `swr` — installed but zero imports (a third data-fetching solution, never actually used)

### Multiple numeric big-number libraries
- `bignumber.js` — used extensively (70+ files)
- `big.js` — zero imports in src (redundant, unused)

### Multiple HTTP clients
- `axios` — used in 12 files
- native `fetch` — used in 21+ files
- `graphql-request` — used in 2 files

### Multiple markdown renderers
- `react-markdown` — used in 3 files (for rendering)
- `@milkdown/*` suite (`@milkdown/kit`, `@milkdown/react`, `@milkdown/theme-nord`, `@milkdown/plugin-automd`) — used in the Studio editor feature

These serve different purposes (display vs. editing) so the duplication is more justifiable, but both `react-markdown` + `rehype-*` + `remark-*` plugins overlap in the markdown pipeline.

### Multiple Ethereum/Web3 libraries (all largely unused)
- `ethers` (10 MB) — only in `window.d.ts` declaration
- `web3` v1.2.4 (8.4 MB) — zero imports
- `web3-utils` — zero imports
- `@ethersproject/providers` — only in `window.d.ts`
- `@uniswap/sdk` — used in 1 file (`useGetTokensInfo.jsx`)

All of these are blockchain-legacy packages, mostly unused.

### Lodash vs Ramda (two general-purpose utility libs)
- `lodash` — used in 13 files
- `ramda` — used in 6 files

Both provide overlapping FP/utility functions; the codebase uses both inconsistently.

---

## 4. OUTDATED / DEPRECATED / RISKY PACKAGES

| Package | Installed | Issue |
|---|---|---|
| `@cosmjs/launchpad` | `^0.27.1` | **Officially deprecated** by CosmJS team; replaced by `@cosmjs/stargate`. Still imported in 33 files — migration needed |
| `@cybercongress/gravity` | `0.0.15` | **Explicitly marked deprecated** in `.eslintrc` (`"Lib is deprecated"`), yet still in `dependencies`. Imported in 70+ files |
| `ipfs-core` | `0.18.0` | The `js-ipfs` project was **sunset**; superseded by Helia. Used in `js-ipfs.ts` impl |
| `ipfs-http-client` | `60.0.0` | Superseded by `kubo-rpc-client`. Zero imports in src — dead weight |
| `web3` | `1.2.4` | **Very old** (2019); web3.js v4 is the current version. Zero imports anyway |
| `redux-observable` | `3.0.0-rc.2` | **Stuck as RC** — never released stable v3. Zero imports in src |
| `react-helmet` | `6.1.0` | Unmaintained (last release 2020). Used in 8 files. `react-helmet-async` is the active fork |
| `@rjsf/core` | `^3.2.1` | Very old (v3 — current is v5). Used in only 1 file |
| `react-json-view` | `^1.21.3` | Unmaintained, has known React 18 issues. Used in ~1 file |
| `codemirror` | `5.58.2` | **CodeMirror v5** — major version behind (v6 is current). Used via `react-codemirror2` |
| `@uniswap/sdk` | `^3.0.3` | Outdated (v3, current is v4 as `@uniswap/sdk-core`). Used in 1 file |
| `history` | `4.10.1` | v4 — react-router v6 ships its own `history` v5 internally |
| `query-string` | `6.14.1` | v6 — current is v9 (ESM). Only used in 0 files anyway |
| `ledger-cosmos-js` | `2.1.7` | Abandoned; zero imports in src |
| `@confio/relayer` | `0.4.0` | 65 MB, old pinned version. Used in relayer services (3 files) |

---

## 5. HEAVY DEPENDENCIES (SIZE ANALYSIS)

Full `node_modules`: **3.3 GB** — extremely large for a frontend app.

| Package | Size | Notes |
|---|---|---|
| `node-datachannel` | **698 MB** | Pulled in by `@libp2p/webrtc`. Contains prebuilt native binaries for many platforms. A browser app almost certainly only needs the WASM/JS version |
| `@storybook/*` | **325 MB** | All storybook packages. Only needed for UI dev — should never be in prod installs |
| `@babel/*` | **219 MB** | Babel toolchain. Project uses rspack + SWC loader — babel should be largely unnecessary |
| `onnxruntime-node` | **92 MB** | Node.js native ONNX runtime. Not imported anywhere in src. Pulled as an optional dep of `@xenova/transformers` (which itself is only used in 1 file) |
| `@confio/relayer` | **90 MB** (shown as 65M direct) | Massive for a relayer utility used in 3 files |
| `@cybercongress/*` | **82 MB** | Includes deprecated `@cybercongress/gravity` |
| `plotly.js` | **71 MB** | Pulled by `@cybercongress/gravity` (the deprecated lib) — not directly imported |
| `onnxruntime-web` | **66 MB** | Also from `@xenova/transformers` |
| `jscodeshift` | **48 MB** | Pulled by `cjs-to-es6` and `ts-migrate` — codemod tools, not runtime code |
| `5to6-codemod` | **20 MB** | Another codemod tool pulled by `cjs-to-es6` |
| `super-three` / `three` | **65 MB combined** | 3D library — not directly imported in active src. Likely from `aframe` or `@cybercongress/gravity` |
| `aframe` | **28 MB** | Not directly imported in src |
| `ts-migrate` | Listed in devDeps | Brings in `jscodeshift` + `ts-migrate-plugins` — one-time migration tool |
| `@ledgerhq/*` | **35 MB** | `@ledgerhq/hw-transport-webusb` has zero imports in src (only `ledger-cosmos-js` had it, which is also unused) |

**Root causes of bloat:**
1. `@libp2p/webrtc` -> `node-datachannel` (698 MB of native binaries for a browser app)
2. `@xenova/transformers` -> `onnxruntime-node` + `onnxruntime-web` (158 MB for 1 worker file)
3. `@cybercongress/gravity` (deprecated) -> `plotly.js`, `three`, `aframe` (200+ MB of unused visualization libs)
4. `cjs-to-es6` + `ts-migrate` in `dependencies` — brings in `jscodeshift`, `5to6-codemod` (100+ MB of migration tooling)

---

## 6. IMPORT PATTERNS

**Mostly consistent, with mixed use of absolute vs. relative:**

| Pattern | Count | Notes |
|---|---|---|
| Absolute `src/` alias | **575 files** | Primary pattern — `import X from 'src/components/...'` using the rspack alias |
| Relative imports | **144 files** | Used alongside absolute, especially in service/feature files |
| Deep relative (`../../../../`) | **33 files** | Found mostly in `features/cybernet/`, `features/sense/`, `containers/` — should use `src/` alias instead |
| `components/` alias | **1 file** | Almost entirely abandoned alias |
| Barrel `index` imports | **43 barrel files**, **6 explicit `/index` imports** | Low barrel usage — most imports go directly to the file |

**Notable inconsistency:** The `.eslintrc` has the `alias` resolver commented out (the `"alias"` block is commented), yet `src/` alias works via rspack. This means ESLint's import resolver only uses the TypeScript resolver — the `components/` alias in rspack is essentially dead.

**JSX/TSX import mix:** The codebase has both `.jsx` and `.tsx` files (not fully migrated to TypeScript). `.jsx` files still use `prop-types` patterns though `prop-types` itself shows zero imports.

---

## 7. CIRCULAR IMPORTS

Found **4 real circular import cycles** (2 are files importing themselves — likely re-export barrel file issues):

### Self-imports (file imports its own default export — likely a barrel re-export conflict)
```
src/features/cybernet/ui/components/SubnetPreview/SubnetPreview.tsx
  -> imports SubnetPreview from './SubnetPreview'  (itself)

src/pages/Portal/Map/Map.tsx
  -> imports Map from './Map'  (itself)
```
Both files appear to have a default export that is re-imported in the same file — likely an accidental circular default export. The component renders as both the default and named export.

### Genuine mutual circular dependencies
```
src/features/cybernet/ui/cybernet.context.tsx
  -> imports useQueryCybernetContract  (from useQueryCybernetContract.refactor.ts)
src/features/cybernet/ui/useQueryCybernetContract.refactor.ts
  -> imports useCybernet  (from cybernet.context.tsx)
```

```
src/features/sense/ui/Sense.tsx
  -> imports ActionBar, ActionBarLLM  (from ActionBar/ActionBar.tsx)
src/features/sense/ui/ActionBar/ActionBar.tsx
  -> imports AdviserProps type  (from ../Sense)
```

```
src/services/CozoDb/cozoDb.ts
  -> imports fetchInitialEmbeddings  (from migrations/migrations.ts)
src/services/CozoDb/migrations/migrations.ts
  -> imports DB_VERSION, CybCozoDb  (from ../cozoDb)
```

```
src/pages/Social/Social.tsx
  -> imports Socials  (from ./Socials)
src/pages/Social/Socials.tsx
  -> imports HUB_LINK, Social  (from ./Social)
```

---

## SUMMARY: TOP PRIORITIES

**Highest impact (size / correctness):**

1. **`@libp2p/webrtc`** — its `node-datachannel` dep alone is 698 MB. If WebRTC peering isn't actively used, removing this one package saves ~700 MB from `node_modules`. If needed, investigate `@libp2p/webrtc-direct` or ensure only the browser build is used.

2. **`@cybercongress/gravity`** — marked deprecated in `.eslintrc` yet imported in 70+ files. Pulls in `plotly.js` (71 MB), `three`/`super-three` (65 MB), `aframe` (28 MB). Migrating away from this single package would save 200+ MB and remove a major security surface.

3. **`cjs-to-es6` + `ts-migrate`** in `dependencies` — these are one-time migration CLI tools. Moving them out of `dependencies` entirely (they shouldn't even be in `devDependencies` permanently) removes ~80 MB of `jscodeshift` + codemod tooling from installs.

4. **`@xenova/transformers`** — used in 1 worker file but brings in `onnxruntime-node` (92 MB) and `onnxruntime-web` (66 MB). This is a browser app — `onnxruntime-node` should be excluded via `optionalDependencies` or `.npmrc`.

5. **`@cosmjs/launchpad`** (deprecated) imported in 33 files — active migration needed to `@cosmjs/stargate`.

6. **Move ESLint plugins out of `dependencies`:** `eslint-import-resolver-alias`, `eslint-plugin-react-hooks`, `eslint-plugin-unused-imports`, `@storybook/addon-designs` — these contaminate production installs.

7. **Move `axios`, `uuid`, `cosmjs-types`, `redux`, `@keplr-wallet/types`** from `devDependencies` to `dependencies` — they are used at runtime.

8. **Fix 4 circular import cycles** — the `cozoDb <-> migrations` and `cybernet.context <-> useQueryCybernetContract` ones are real runtime risks; the self-importing `SubnetPreview` and `Map` components are likely bugs.


---

## 3. React Component Architecture Audit

# React Component Architecture Audit — `cyb-ts/src`

---

## 1. Component Size — Flagged (over 300 lines)

Total component files scanned: all `.tsx`/`.jsx` under `src/`. Total SLOC across all files: **58,377**.

| Lines | File |
|-------|------|
| 767 | `containers/portal/gift/ActionBarPortalGift.tsx` |
| 726 | `containers/txs/Activites.jsx` |
| 685 | `containers/portal/gift/index.tsx` |
| 670 | `containers/portal/citizenship/index.tsx` |
| 625 | `components/TableTxsInfinite/component/RenderValue.tsx` |
| 456 | `pages/Sphere/pages/components/ActionBarContainer/ActionBarContainer.tsx` |
| 441 | `containers/Search/ActionBarContainer.tsx` |
| 437 | `features/cybernet/ui/pages/Subnet/tabs/SubnetNeurons/SubnetNeuronsTable/SubnetNeuronsTable.tsx` |
| 430 | `containers/warp/Warp.tsx` |
| 425 | `containers/portal/release/index.tsx` |
| 412 | `pages/robot/_refactor/account/actionBar.tsx` |
| 400 | `pages/teleport/swap/swap.tsx` |
| 378 | `features/ipfs/Drive/index.tsx` |
| 375 | `features/ibc-history/historyContext.tsx` |
| 375 | `containers/energy/component/actionBar.tsx` |
| 374 | `containers/portal/citizenship/ActionBar.tsx` |
| 372 | `pages/teleport/bridge/bridge.tsx` |
| 361 | `containers/temple/pages/play/PlayBanerContent.jsx` |
| 354 | `components/ledger/stageActionBar.tsx` |
| 348 | `containers/portal/release/ActionBarRelease.tsx` |
| 347 | `containers/warp/ActionBar.tsx` |
| 337 | `pages/teleport/send/send.tsx` |
| 325 | `features/cybernet/ui/pages/Subnet/useCurrentSubnetGrades.tsx` |
| 325 | `containers/portal/components/currentGift/index.jsx` |
| 301 | `containers/trollBox/index.jsx` |

**Key observations:**

- `ActionBarPortalGift.tsx` (767 lines) is the worst offender. It mixes multi-wallet signing logic (Keplr, MetaMask), 15+ step states, dispatch calls, IPFS writes, and all render branches in one file. It should be split into at minimum: a state machine hook, separate signing sub-components, and a thin render shell.
- `Activites.jsx` (726 lines, `.jsx`) is essentially a giant `switch`-less if-chain rendering 20+ message types. Every message type should be its own component in a registry map.
- `citizenship/index.tsx` (670 lines) uses `connect()` from react-redux (legacy pattern) alongside modern hooks. It manages sounds, multi-step wizard logic, and rendering all in one component.
- `historyContext.tsx` (375 lines) is a Context provider doing too much — it handles DB reads, WebSocket polling, tx tracing, retry loops, and state — all inside the provider body. This is a service layer masquerading as React.

---

## 2. State Management Patterns

Four patterns are in concurrent use, often within the same component. They are **not consistent**.

### a) Redux (RTK) — 27 files
Used via `useSelector`/`useDispatch`. Slices found under `src/redux/features/` (pocket, warp, currentAccount, ibcDenom) and `src/features/*/redux/`. Usage is reasonable but narrow — primarily for wallet/account state, IBC denom cache, bandwidth, and sense notifications.

```ts
// Typical usage — src/containers/portal/gift/index.tsx
const { defaultAccount } = useSelector((store: RootState) => store.pocket);
const dispatch = useDispatch();
```

**Problem:** Some files still use the legacy `connect()` HOC from react-redux (7 files), mixing the old and new Redux APIs:
```tsx
// containers/portal/citizenship/index.tsx, containers/Search/ActionBarContainer.tsx
import { connect } from 'react-redux';
```

### b) React Context — 16 context files in `src/contexts/` + 10+ feature-level contexts
See section 5 for full list. Heavily used for backend services, query clients, signing, device, analytics.

### c) React Query (`@tanstack/react-query`) — 287 matches across 98 files
This is the dominant server-state fetching pattern and is used consistently in newer code. The `queryClient.tsx` context wraps `QueryClientProvider`. Hooks like `useQuery`, `useMutation`, `useInfiniteQuery` appear everywhere in `features/` and `hooks/`.

### d) Local `useState` — Dominant everywhere
Every component manages its own loading flags, step state, selected addresses, etc. There is no shared local-component state library (e.g. Zustand, Jotai). This leads to deep prop-drilling of state setters (see section 3).

**Consistency verdict:** Inconsistent. New features use React Query + Contexts. Older portal/container code uses Redux `connect()` + direct `useState` chains. The `_refactor` folder in `pages/robot/` signals that modernisation was started but not finished.

---

## 3. Props Drilling — Components with More Than 8 Props

The Python/awk heuristic on `type Props = {}` blocks hit shell limitations with this codebase's style. Direct inspection of the largest files confirms the following worst offenders:

**`ActionBarPortalGift.tsx`** — 11 props passed from parent:
```ts
type Props = {
  addressActive?: { bech32: string };
  citizenship: Nullable<Citizenship>;
  updateTxHash?: (data: TxHash) => void;
  isClaimed: any;               // typed as any
  selectedAddress?: string;
  currentGift: any;             // typed as any
  activeStep: any;              // typed as any
  setStepApp: any;              // typed as any
  setLoadingGift: any;          // typed as any
  loadingGift: any;             // typed as any
  progressClaim: number;
  currentBonus: number;
};
```
11 props, 6 of them typed `any`. This is both a props-drilling issue and a TypeScript quality issue in one place.

**`ActionBarRelease.tsx`** — 10+ props:
```ts
type Props = {
  txHash, updateTxHash, selectedAddress, isRelease, loadingRelease,
  addressActive, currentRelease, redirectFunc, callback, availableRelease
  // + totalGift, totalRelease passed but not in type definition
}
```
Note `totalGift` and `totalRelease` are destructured in the function signature but absent from the `Props` type — a TypeScript hole.

**`portal/gift/index.tsx`** passes ~12 props down into `ActionBarPortalGift` and ~8 into `ActionBarRelease` at the call site. This is a classic props-drilling anti-pattern — the parent is acting as a data bus for children that should pull their data from contexts or hooks directly.

**`containers/Search/ActionBarContainer.tsx`** (441 lines, class component):
```tsx
// Props passed via connect() + parent + withIpfsAndKeplr HOC wrapping
signer: any, signingClient: any, ipfsApi, senseApi, queryClient, defaultAccount, ...
```
This component receives injected props from 3 sources: Redux `connect()`, the `withIpfsAndKeplr` HOC, and the parent.

---

## 4. Legacy Patterns

### a) Class Components — 4 files
```
src/components/ErrorBoundary/ErrorBoundary.tsx  (intentional — React requires this)
src/pages/robot/_refactor/account/actionBar.tsx
src/containers/story/story.jsx
src/containers/Search/ActionBarContainer.tsx
```

`ActionBarContainer.tsx` is a 441-line class component with constructor state, `componentDidMount`, `componentDidUpdate`, and `this.timeOut` / `this.transport` instance variables. It should be a functional component. It also carries the `/* eslint-disable */` escape at line 1.

`story.jsx` similarly uses class syntax.

### b) HOCs — 3 active HOCs
```
src/hocs/withAccount.tsx        — wraps Redux defaultAccount into props (any-typed)
src/hocs/withIpfsAndKeplr.tsx   — injects 5 context values as props (any-typed)
src/components/helpers/withRouter.js  — legacy router HOC
src/components/helpers/withDevice.tsx — injects device context as prop
```

`withIpfsAndKeplr.tsx` injects `ipfsApi`, `signer`, `signingClient`, `senseApi`, `queryClient` as props. Every one of these is now available via a custom hook (`useBackend()`, `useSigningClient()`, `useQueryClient()`). The HOC is a compatibility shim that should be removed and callers updated to use hooks directly.

```tsx
// withIpfsAndKeplr.tsx - both args typed `any`
const withIpfsAndKeplr = (Component: React.ComponentType) =>
  function WithIpfsAndKeplr(props: any) { ... }
```

### c) Legacy Redux `connect()` — 7 files
Files still using `connect()` from `react-redux`:
```
containers/portal/citizenship/index.tsx
containers/portal/citizenship/ActionBar.tsx
containers/portal/mainPortal.jsx
containers/Search/ActionBarContainer.tsx
containers/market/index.jsx
containers/help/index.jsx
contexts/queryClient.tsx  (surprisingly)
```

### d) `.jsx` files — untyped JavaScript components
```
containers/txs/Activites.jsx       (726 lines, no types)
containers/portal/citizenship/index.tsx (uses connect())
containers/trollBox/index.jsx      (301 lines)
containers/temple/pages/play/PlayBanerContent.jsx (361 lines)
containers/portal/components/currentGift/index.jsx (325 lines)
containers/market/index.jsx
containers/help/index.jsx
containers/portal/mainPortal.jsx
containers/story/story.jsx
```

### e) `React.memo` — barely used
Only **2 files** use `React.memo`:
```
src/components/EPubView/EPubView.tsx
src/components/Slider/Slider.tsx
```
For a codebase of this size (58k lines), this is very low. Most components will re-render on every parent render.

### f) `withRouter` legacy helper
`src/components/helpers/withRouter.js` — a custom re-implementation of the removed `withRouter` HOC from React Router v5. In v6 you use `useNavigate`/`useParams` hooks directly.

---

## 5. Context Providers

### Global contexts in `src/contexts/`

| File | Purpose | Well-structured? |
|------|---------|-----------------|
| `analytics.tsx` | Plausible analytics tracking | Yes — clean, memoized, value wrapped |
| `appData.tsx` | Market data, block height, filter particles | Acceptable, `filterParticles` not memoized stably |
| `backend/backend.tsx` | IPFS, DB, sync workers, senseApi | Large (247 lines) but reasonably well-memoized |
| `chain/chain.tsx` | Chain info, RPC client | OK |
| `chain/useGetRpcClient.ts` | RPC health-checking | Standalone hook, fine |
| `device.tsx` | Mobile/desktop detection | Small and clean |
| `hub.tsx` | Hub contract data | Delegates to Redux, small |
| `ibcDenom.tsx` | IBC denom traces cache | OK |
| `networks.tsx` | Network list | OK |
| `previousPage.tsx` | Navigation history | Simple |
| `queryClient.tsx` | `CyberClient` (read) | Uses `connect()` — legacy |
| `queryCyberClient.tsx` | `cyber-ts` RPC hooks | Renders `null` while loading — blocks tree |
| `relayer.tsx` | IBC relayer state | Uses `connect()` — legacy |
| `scripting/scripting.tsx` | Rune scripting engine | OK |
| `signerClient.tsx` | Keplr signing client | Well-structured, good memoization |

### Feature-level contexts (outside `src/contexts/`)

```
features/ibc-history/historyContext.tsx      (375 lines — too large)
features/adviser/context.tsx
features/sense/ui/SenseAdviser.context.tsx
features/cybernet/ui/cybernet.context.tsx
features/cybernet/ui/pages/Subnet/subnet.context.tsx
features/studio/studio.context.tsx
pages/robot/robot.context.tsx
pages/Sphere/Sphere.context.tsx
pages/teleport/Teleport.context.tsx
containers/sigma/SigmaContext.tsx
websockets/context.tsx
components/Select/selectContext.tsx
```

**Problem — `queryCyberClient.tsx`:**
```tsx
if (!rpcClient) {
  return null;   // Blocks the entire React tree below it
}
const hooks = createRpcQueryHooks({ rpc: rpcClient });
```
`createRpcQueryHooks` is called on every render (not memoized). If `rpcClient` is briefly undefined after a reconnect, the entire subtree unmounts.

**Problem — `historyContext.tsx`:**
The provider body contains 375 lines of logic: Dexie DB queries, WebSocket polling, fetch calls, retry loops, and status tracing. The context value object is constructed inline without `useMemo`:
```tsx
<HistoryContext.Provider value={{   // new object on every render
  ibcHistory,
  changeHistory,
  addHistoriesItem,
  pingTxsIbc,
  useGetHistoriesItems,
  updateStatusByTxHash,
  traceHistoryStatus,
}}>
```
Every consumer re-renders on every parent state change because the value reference always changes.

**Potentially unused / consolidation candidates:**
- `previousPage.tsx` — niche usage, could be a simple hook with `useRef`
- `hub.tsx` — thin wrapper over Redux; questionable whether it needs to be a context
- `queryCyberClient.tsx` and `queryClient.tsx` serve similar purposes (read-only client access) and could be merged

---

## 6. Custom Hooks

### `src/hooks/` — 37 hooks, sizes:

| Lines | Hook |
|-------|------|
| 224 | `useGetMarketData.ts` |
| 142 | `getBalances.ts` |
| 140 | `useHub.ts` |
| 112 | `useGetWarpPools.ts` |
| 97 | `useQueueIpfsContent.ts` |
| 90 | `useGetTotalSupply.ts` |
| 73 | `useWaitForTransaction.ts` |
| 73 | `useTracesNetworks.ts` |
| 70 | `useAllDenomTraces.ts` |
| 67 | `useParticle.ts` |

**`useGetMarketData.ts` (224 lines) — doing too much:**
This hook fetches total supply, pool list, calculates pool balances (via `await` inside a loop with `for...in`), derives prices, calculates market cap for LP tokens, and persists to `localStorage` — all in one hook with 5 separate `useEffect` chains. Each effect has a `try/catch` that calls `console.log('error', error)` and resets state silently. This is a data pipeline, not a hook, and should be a service function called from a single effect.

**`useHub.ts` (140 lines):** Uses both `useQuery` (React Query) AND manual `useState`/`useEffect` with `useDispatch` for the same data — duplicating state with different fetch mechanisms.

**`useGetWarpPools.ts` (112 lines):** Similar issue — mixes React Query with manual state.

**`useQueueIpfsContent.ts` (97 lines):** Manages a processing queue inside React state — this is service-layer logic in a hook.

**Naming issue:** `getBalances.ts` is named like a utility function but is a React hook (uses `useState`, `useEffect`). Hook files should be prefixed `use`.

**`useMediaQuery.js`** — `.js` extension, no TypeScript.

---

## 7. Error Handling

### Error Boundaries
Only **one** `ErrorBoundary` component exists (`components/ErrorBoundary/ErrorBoundary.tsx`) and it is mounted in only **4 places**: `index.tsx` (root), `router.tsx`, `CyberlinksGraphContainer.tsx`, and the ErrorBoundary file itself. For a 58k-line app with complex async flows, this is very thin coverage.

The recent commits show granular ErrorBoundary wrappers were added (`f4dbd053`), which is good progress.

### `try/catch` patterns — 165 total catch blocks
Of those, **118 catch blocks contain a `console.log` or `console.warn`** and then swallow the error or reset state silently. Examples:

```ts
// useGetMarketData.ts — repeated 4 times
} catch (error) {
  console.log('error', error);
  setDataTotal([]);   // silently resets state, no user feedback
}

// ActionBarPortalGift.tsx — repeated 3 times
} catch (error) {
  console.log('error', error);
  setStepApp(STEP_INFO.STATE_PROVE);   // silently goes back a step
}

// portal/gift/index.tsx
} catch (error) {
  console.log('error', error);
  return null;
}
```

No error is surfaced to the user in these cases. The adviser/toast system (`useAdviser`) exists but is not connected to these catch blocks.

### `console.log` production leaks — 331 occurrences across 129 files
This includes debug logging in `backend.tsx`, `citizenship/index.tsx` (10 instances), and active transaction handlers. Many are raw `console.log('executeResponseResult', executeResponseResult)` left in from development.

### TypeScript suppressions — 17 occurrences across 11 files
```
// @ts-ignore — 1 file (TracerTx.ts)
// @ts-nocheck — used in test files and relayer
// @ts-expect-error — relayer files
```

---

## 8. TypeScript Quality

### `any` usage
| Scope | Count | Files |
|-------|-------|-------|
| `.tsx` files | **44 occurrences in 34 files** | Mostly in HOCs, portal components |
| `.ts` files | **61 occurrences in 29 files** | Mostly in generated code, scripting types |
| **Total** | ~105 | |

Worst single-file offender: `ActionBarPortalGift.tsx` with **6 `any` props** in `type Props`:
```ts
isClaimed: any;
currentGift: any;
activeStep: any;
setStepApp: any;
setLoadingGift: any;
loadingGift: any;
```

Other notable cases:
```ts
// withAccount.tsx
const withAccount = (Component) => (props: any) => { ... }
// — Component is untyped, props is any

// withIpfsAndKeplr.tsx
function WithIpfsAndKeplr(props: any) { ... }

// containers/Search/ActionBarContainer.tsx
class ActionBarContainer extends Component<Props, any>
// — state is any

// signerClient.tsx
getSignClientByChainId: () => {},  // typed as () => void but returns Promise
```

### Missing `eslint-disable` suppressions covering hook rules — 60 instances
```
// eslint-disable-next-line react-hooks/exhaustive-deps
```
Found 60 times across the codebase. Each one is a potential stale closure bug. The worst cluster is in `portal/citizenship/index.tsx` with 10 instances.

### Untyped `.jsx` files
9+ component files in `.jsx` with no TypeScript at all (see section 4d). `Activites.jsx` at 726 lines is the most critical.

### Unsafe property access patterns
```ts
// historyContext.tsx
item.timeoutTimestamp!   // non-null assertion
data?.result?.sync_info?.latest_block_time  // deep optional chaining without null guard on use
```

---

## 9. Re-render Risks

### Inline function definitions in JSX — 129 occurrences across 55 files
Every `onClick={() => someHandler()}` creates a new function reference on every render, causing child components to see new props even if nothing changed. The worst offenders:

```tsx
// containers/portal/citizenship/ActionBar.tsx — 17 inline arrow functions in JSX
// containers/portal/gift/ActionBarPortalGift.tsx — 16 inline arrow functions
// containers/energy/component/actionBar.tsx — 5 inline functions
```

Example from `ActionBarPortalGift.tsx`:
```tsx
<BtnGrd onClick={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)} text="prove address" />
<BtnGrd onClick={() => setStepApp(STEP_INFO.STATE_CLAIME)} text="go to claim" />
// ... 14 more identical patterns
```
None of these are wrapped in `useCallback`. Since `BtnGrd` is not `React.memo`-ized, every parent re-render creates new prop references.

### Context value objects created without memoization
`historyContext.tsx`:
```tsx
// value object is a NEW object literal on every render
<HistoryContext.Provider value={{
  ibcHistory, changeHistory, addHistoriesItem, pingTxsIbc,
  useGetHistoriesItems, updateStatusByTxHash, traceHistoryStatus,
}}>
```
Every component consuming `useIbcHistory()` will re-render on every state change in `HistoryContextProvider`, even if the consumed field didn't change.

### `useCallback`/`useMemo` usage — 419 occurrences across 140 files
Usage is present but inconsistent. Many components use `useMemo` for derived values (good) but then define event handlers inline in JSX (contradictory). The `useGetMarketData.ts` hook uses `useCallback` for `getMarketDataPool` but lists `dataTotal` and `poolsTotal` in its deps with `// eslint-disable-next-line react-hooks/exhaustive-deps`, hiding potential stale closures.

### `React.memo` — only 2 files
`EPubView.tsx` and `Slider.tsx`. No other components are memoized. Given the inline-function patterns above, list items, table rows, and action bar buttons will all re-render on every ancestor state change.

### `createRpcQueryHooks` called inside render
```tsx
// queryCyberClient.tsx — called on every render, not memoized
const hooks = createRpcQueryHooks({ rpc: rpcClient });
return <QueryClientContext.Provider value={{ rpc: data, hooks }}>
```
`hooks` is a new object on every render, and `{ rpc: data, hooks }` is also a new object — every consumer re-renders on every provider render.

### Large state objects in context causing cascade re-renders
`appData.tsx` computes `filterParticles` inline:
```tsx
const filterParticles = filterContractQuery?.data?.map((item) => item[1]);
// Not memoized — new array on every render
const valueMemo = useMemo(() => ({
  ...
  filterParticles: filterParticles || [],  // [] is also a new reference if undefined
}), [resultMarketData, dataTotal, blockHeight, filterParticles]);
```
`filterParticles` is a new array reference on every render (the `.map()` is not memoized), so `filterParticles` will always be a new dependency for `valueMemo`, defeating the memoization.

---

## Summary Priority Table

| Severity | Issue | Impacted Files |
|----------|-------|---------------|
| Critical | `historyContext.tsx` context value not memoized — cascades re-renders to all IBC consumers | 1 context + all consumers |
| Critical | `queryCyberClient.tsx` returns `null` blocking entire tree; `createRpcQueryHooks` not memoized | 1 context |
| Critical | `ActionBarPortalGift.tsx` (767 lines) — 11 props, 6 typed `any`, all logic in one file | 1 component |
| Critical | `Activites.jsx` (726 lines) — untyped JS, no splitting of message-type components | 1 component |
| High | Legacy `connect()` HOC in 7 files alongside modern hook-based Redux | 7 files |
| High | `ActionBarContainer.tsx` (441 lines) — class component, `/* eslint-disable */`, mixed patterns | 1 component |
| High | 105 `any` usages in typed files — especially 6 props of `ActionBarPortalGift` | 34+ files |
| High | 331 `console.log` calls including in transaction handlers and backend | 129 files |
| High | 118 catch blocks swallow errors via `console.log` — no user-facing feedback | many files |
| High | `useGetMarketData.ts` (224 lines) — data pipeline in a hook, 4 silent error swallows | 1 hook |
| Medium | 9 `.jsx` files (726–301 lines) with no TypeScript | 9 files |
| Medium | 129 inline arrow functions in JSX with no `useCallback` wrapping | 55 files |
| Medium | `React.memo` used in only 2 files — no memoized list/table/button components | whole codebase |
| Medium | 60 `react-hooks/exhaustive-deps` suppressions — potential stale closures | 60 locations |
| Medium | `withIpfsAndKeplr` and `withAccount` HOCs still in use — inject typed-as-`any` props | 3 HOCs |
| Low | `previousPage.tsx` context — could be a `useRef`-based hook | 1 context |
| Low | `getBalances.ts` — hook named without `use` prefix | 1 file |
| Low | `useMediaQuery.js` — JavaScript hook file | 1 file |
| Low | `filterParticles` not memoized before entering `useMemo` in `appData.tsx` | 1 context |


---

## 4. Services & Backend Architecture Audit

# Services Architecture Audit: `cyb-ts`

---

## 1. Service Inventory (`src/services/`)

| Service | What It Does | Approx Size |
|---|---|---|
| `CozoDb/` | WebAssembly-backed graph/relational DB (`cyb-cozo-lib-wasm`) running inside a dedicated Web Worker. Handles all local persistent storage via IndexedDB. Has its own migration system, schema, command factory, and DTO mapping. | 10 files, ~450 LOC core |
| `QueueManager/` | Priority queue for IPFS content fetching. Uses RxJS `BehaviorSubject` as its reactive queue. Three source strategies (`db → node → gateway`, etc.) with per-source concurrency limits and timeouts. Integrates Rune scripting post-processing. | ~500 LOC |
| `backend/workers/background/` | The main SharedWorker (falls back to Worker in dev). Coordinates IPFS, Rune scripting engine, ML embeddings, and the sync service. Exposed via Comlink proxy. | ~80 LOC entry + sub-APIs |
| `backend/workers/db/` | Dedicated DB Worker exposing CozoDb operations via Comlink. Handles migrations on init. | ~100 LOC |
| `backend/services/sync/` | The sync engine: 4 loop classes (`SyncTransactionsLoop`, `SyncParticlesLoop`, `SyncMyFriendsLoop`, `CommunitySync`), all RxJS-driven, polling/subscribing to the indexer GraphQL and the Tendermint node WebSocket. | 15+ files, ~1200 LOC |
| `backend/services/DbApi/` | `DbApiWrapper` — high-level API over `CozoDbWorker`. All sync loops use this. Contains complex CozoLog query for the Sense list. | ~374 LOC |
| `backend/services/indexer/` | GraphQL query functions against the Hasura indexer. Both HTTP (one-shot queries) and WebSocket subscriptions via Apollo. Uses generated types from `src/generated/graphql.ts`. | ~200 LOC + 10 `.graphql` files |
| `backend/channels/` | `BroadcastChannel`-based IPC between workers and the main thread. `BroadcastChannelSender` (worker side) posts typed messages; `RxBroadcastChannelListener` (main thread) feeds them into Redux dispatch. | ~120 LOC |
| `ipfs/node/` | Three IPFS node implementations behind a single `IpfsNode` interface: `KuboNode` (external daemon via RPC), `JsIpfsNode` (deprecated `ipfs-core`), `HeliaNode` (new libp2p-based). Factory selects by `ipfsNodeType`. `withCybFeatures` mixin adds swarm reconnect + `fetchWithDetails`. | ~600 LOC across 4 files |
| `ipfs/utils/` | Content type detection, MIME parsing, CID utilities, cache DB interface, stream helpers. | ~300 LOC |
| `lcd/websocket.ts` | Creates a raw `WebSocket` to the Tendermint RPC, wraps it in an RxJS `Observable`. Used by `SyncTransactionsLoop` for real-time incoming transfers. | ~45 LOC |
| `lcd/utils/mapping.ts` | Maps raw Tendermint WS tx events to `TransactionDto`. | ~50 LOC |
| `community/` | Fetches follow/follower relationships via LCD REST (`getTransactions` with cyberlink event filters). | ~80 LOC |
| `transactions/lcd.tsx` | Axios-based LCD REST wrappers for Cosmos tx queries. Marked `@deprecated` but still widely used. | ~150 LOC |
| `passports/lcd.ts` | CosmWasm contract query for passport NFTs. | ~20 LOC |
| `neuron/neuronApi.ts` | High-level action functions: `sendCyberlink`, `sendCyberlinkArray`, `sendTokensWithMessage`, `investmint`, `updatePassportParticle`. Uses `SigningCyberClient`. | ~150 LOC |
| `scripting/engine.ts` | Rune scripting engine backed by `cyb-rune-wasm`. Compiles and runs Rune scripts with budget limits. Entrypoints: `personalProcessor`, `askCompanion`, `executeFunction`. RxJS observables track init state. | ~339 LOC |
| `scripting/services/` | LLM (OpenAI) integration for scripting + post-processing pipeline. | ~80 LOC |
| `service-worker/` | Workbox-based Service Worker. Handles precaching, CacheFirst for assets, NetworkFirst for API, POST caching, and offline fallback. | ~90 LOC |
| `graphql/index.ts` | A second Apollo client instance (HTTP + WS split link). **Duplicates** the client setup already inside `backend/services/indexer/graphqlClient.ts`. Appears unused in production paths — the indexer uses `graphql-request` for HTTP and its own Apollo WS client. | ~50 LOC |
| `relayer/` | IBC relayer helpers (private). Not integrated into the main UI flow. | ~7 files |
| `soft.js/` | `Soft3MessageFactory` — fee calculation and message building utilities for signing transactions. | ~4 files |

---

## 2. API Layer

**Three distinct mechanisms are in use simultaneously, with no central API client:**

### REST (LCD)
- `src/services/transactions/lcd.tsx` — direct `axios` calls to `LCD_URL`. Marked `@deprecated`, but used everywhere for follow/follower/tweet queries.
- `src/services/community/lcd.ts`, `src/services/passports/lcd.ts` — also direct REST via the shared `getTransactions` wrapper.
- Pattern: bare `axios.get` with hand-built params. No request interceptor, no retry logic in the base layer.

### GraphQL (HTTP + WebSocket)
- **HTTP queries**: `graphql-request` `GraphQLClient` instantiated per-call via `createIndexerClient(abortSignal)`. No client caching, a new client object per request.
- **WebSocket subscriptions**: Two separate setups:
  1. `createIndexerWebsocket()` in `graphqlClient.ts` — uses `graphql-ws` + Apollo `ApolloClient` with `GraphQLWsLink`. Used by `SyncTransactionsLoop` for real-time tx streaming.
  2. `src/services/graphql/index.ts` — a second standalone Apollo client with HTTP+WS split link. Appears to be a legacy artifact; not imported by any active sync path.
- Generated types live in `src/generated/graphql.ts` (15,589 lines), generated by `@graphql-codegen/cli` against `https://index.bostrom.cybernode.ai/v1/graphql` with `typescript`, `typescript-operations`, and `typescript-react-apollo` plugins.

### CosmJS / cyber-js
- `SigningCyberClient` (`@cybercongress/cyber-js`) and `CyberClient` for on-chain reads/writes.
- `@tanstack/react-query` (`useMutation`) used in feature hooks like `useCyberlink.ts` — so there is a partial React Query layer in features, but it is not used for data fetching from the indexer.

**Summary**: No single consistent API layer. REST = axios, indexer reads = graphql-request, indexer subscriptions = Apollo WS, chain interaction = cyber-js/cosmjs. React Query appears only for mutation-side effects in feature hooks. The `src/services/graphql/index.ts` Apollo client is a dead second client.

---

## 3. IPFS Integration

**Highly complex — three separate IPFS implementations behind one interface.**

### Node Implementations
| Class | Library | Status |
|---|---|---|
| `KuboNode` | `kubo-rpc-client` (HTTP RPC to external daemon) | Active — "external" mode |
| `JsIpfsNode` | `ipfs-core` (embedded JS-IPFS) | Effectively deprecated — `ipfs-core` is an archived project; `ipfs-core-types` still imported in 3 non-impl files |
| `HeliaNode` | `helia` + `@helia/unixfs` + `libp2p` | Active — "helia" mode (the intended future path) |

All three implement the same `IpfsNode` interface (cat, add, pin, stat, ls, getPeers, connectPeer, info). The factory `initIpfsNode()` selects by `ipfsNodeType` config.

### Complexity Issues
- **Three live IPFS libraries** in `package.json`: `helia`, `ipfs-core`, `kubo-rpc-client`. All three plus `ipfs-core-types` (which imports from the archived `ipfs-core`) are actively referenced.
- `ipfs-core-types` is imported in `CozoDb/mapping.ts`, `CozoDb/types/entities.ts`, and `SyncIpfsLoop/services.ts` — these are not dead from the compiler's view but `ipfs-core` itself is deprecated upstream.
- `SyncIpfsLoop` is **commented out** in `sync.ts` (`// new SyncIpfsLoop(deps, particlesResolver).start()`). Its service file still imports `ipfs-core-types`. This is dead sync code that was never removed.
- **Mixin pattern**: `withCybFeatures` wraps any `IpfsNode` class to add `reconnectToSwarm()`, `fetchWithDetails()`, and `addContent()`. This is the correct abstraction but the mixin has to handle all three backends via polymorphism.
- The IPFS node lives inside the background SharedWorker (`ipfsApi.ts`). The main thread never holds a direct IPFS reference — it goes through the Comlink proxy.
- `QueueManager` manages IPFS fetch concurrency: up to 30 concurrent node fetches, 11 gateway fetches, unlimited DB fetches. It automatically falls back across sources (`db → node → gateway` for Kubo/Helia).

---

## 4. Backend / Sync Architecture

The backend is a **two-worker architecture** connected via Comlink and a BroadcastChannel bus:

```
Main Thread (React)
  ├── BackendProvider (React Context)
  │     ├── cozoDbWorkerInstance  ←── [Comlink proxy] ──→  DB Worker
  │     │     └── CozoDb (WASM, IndexedDB)
  │     ├── backgroundWorkerInstance ←── [Comlink proxy] ──→  Background Worker
  │     │     ├── ipfsApi (QueueManager + IpfsNode)
  │     │     ├── rune (scripting engine)
  │     │     ├── embeddingApi$ (ML/sentence-transformers)
  │     │     └── SyncService
  │     │           ├── SyncTransactionsLoop   (GraphQL WS + Tendermint WS)
  │     │           ├── SyncParticlesLoop      (GraphQL HTTP polling)
  │     │           ├── SyncMyFriendsLoop      (GraphQL HTTP polling per friend)
  │     │           └── CommunitySync          (follows/followers from LCD)
  │     └── DbApiWrapper (high-level DB interface, proxied into bg worker)
  │
  └── BroadcastChannel ("cyb~broadcast")
        Worker → Main: service_status, sync_entry, sync_ml_entry,
                       load_community, sense_list_update (Redux actions)
        Main → Worker: (none directly; uses Comlink calls instead)
```

### How Sync Works
1. `BackendProvider` initializes both workers on mount.
2. DB worker initializes CozoDb WASM, runs migrations, reports `service_status: started` via BroadcastChannel.
3. Background worker starts IPFS (with retry), then receives a `DbApiWrapper` proxy from the main thread via `injectDb()`.
4. `SyncService` starts 4 loops, all waiting on RxJS `combineLatest` of `[dbInstance$, ipfsInstance$, params$]`.
5. **`SyncTransactionsLoop`**: does an initial bulk fetch from the indexer (paginated via `fetchTransactionsIterable`), then switches to dual WebSocket subscription (GraphQL indexer + Tendermint node WS) for live updates.
6. **`SyncParticlesLoop`**: polls the indexer on a timer for new cyberlinks (tweets) belonging to the current user. For each particle, fetches its links from the indexer and enqueues them in the IPFS QueueManager.
7. **`SyncMyFriendsLoop`**: for each followed address, fetches their cyberlinks from the indexer. Restarts when the followings list changes.
8. Progress is broadcast to Redux via `BroadcastChannelSender.postSyncEntryProgress()` → `backendReducer` case `sync_entry`.
9. Sense list updates flow from workers → `BroadcastChannelSender.postSenseUpdate()` → this dispatches the Redux action `updateSenseList` directly (the sender imports and calls `updateSenseList(senseList)` from the sense redux slice and posts that action object over BroadcastChannel, where `RxBroadcastChannelListener` feeds it straight into `dispatch()`).

---

## 5. Redux Usage

### Slices (13 total)

| Slice | File | Type | Size | Notes |
|---|---|---|---|---|
| `gol` | `reducers/gol.js` | Plain JS switch reducer | ~90 LOC | Game-of-Links prizes. Old-style, no RTK. Likely dead — GoL ended. |
| `bandwidth` | `reducers/bandwidth.js` | Plain JS switch reducer | ~30 LOC | Single `SET_BANDWIDTH` action. Old-style. |
| `pocket` | `features/pocket.ts` | RTK slice | ~200 LOC | Account management, localStorage persistence. Has a hand-written thunk `initPocket`. |
| `currentAccount` | `features/currentAccount.ts` | RTK slice | ~40 LOC | Community following/followers/friends for current account. Thin wrapper — duplicates data from `backend.community`. |
| `passports` | `features/passport/passports.redux.ts` | RTK slice + `createAsyncThunk` | ~150 LOC | Address-keyed passport map. Uses `createAsyncThunk` for LCD calls. |
| `backend` | `reducers/backend.ts` | Old-style switch reducer | ~203 LOC | Service statuses, sync progress, community list. **No RTK** — receives raw BroadcastChannel messages as actions. Uses `ramda.assocPath` for immutability. |
| `commander` | `containers/application/Header/Commander/commander.redux` | RTK slice | unknown | UI command palette. |
| `sense` | `features/sense/redux/sense.redux.ts` | RTK slice + 2 `createAsyncThunk` | ~574 LOC | **Largest slice.** Chat list, per-chat message cache, unread counts, and LLM thread state all in one slice. LLM threads persisted directly to `localStorage` inside reducers. |
| `warp` | `features/warp.ts` | RTK slice | ~50 LOC | Liquidity pools, localStorage persistence. |
| `ibcDenom` | `features/ibcDenom.ts` | RTK slice | ~45 LOC | IBC denom traces, localStorage persistence. |
| `scripting` | `reducers/scripting.ts` | RTK slice | ~120 LOC | Rune script entrypoints + context. Reads/writes localStorage in reducers (side-effect in reducer — an anti-pattern). |
| `hub` | `pages/Hub/redux/hub` | RTK slice | unknown | Hub page state. |
| `timeHistory` | `features/TimeHistory/redux/TimeHistory.redux.ts` | RTK slice | unknown | Navigation history. |

### Notable Issues
- **`sense.redux.ts` is bloated (574 LOC)**: mixes messaging state, LLM thread state, and optimistic update logic. LLM threads are a conceptually separate concern.
- **`backend` reducer is old-style switch**: the only reducer not using RTK. It exists to handle raw BroadcastChannel message objects that arrive as Redux actions — a design that bypasses RTK because the messages come from outside the normal dispatch path.
- **`gol` reducer is very likely dead code**: Game of Links was a testnet incentive program. No actions dispatch `CHANGE_GOL_*` type strings in the current codebase.
- **`currentAccount` duplicates `backend.community`**: `currentAccount.community` and `backend.community` hold essentially the same following/followers/friends data. `currentAccount` appears to be an older slice that was partially superseded.
- **Multiple slices write localStorage in reducers** (`pocket`, `warp`, `ibcDenom`, `scripting`) — side effects in reducers are an RTK anti-pattern and make testing difficult.
- **Two plain JS reducers** (`gol`, `bandwidth`) remain unported to RTK.
- 26 feature files use `useAppSelector`, so Redux is pervasive in features.

---

## 6. Feature Modules (`src/features/`)

### Module Inventory

| Feature | Contents | Follows FSD? |
|---|---|---|
| `sense/` | Full messaging/notification inbox. Redux slice, UI components, action bar, LLM chat, voice interaction. Has its own `frontend-architecture.md`. | Partial — has `ui/`, `redux/`, `types/` but no `api/` or `model/` segments |
| `cyberlinks/` | Cyberlink graph visualization, hooks for creating/counting links, rank display. | Partial — `hooks/`, `graph/`, `CyberlinksGraph/` but mixed naming |
| `cybernet/` | Full sub-application: CosmWasm contract interaction for neural subnet staking. Has `api.ts`, `types.ts`, `ui/`, context, hooks, routes. | Closest to FSD — has api/types/ui layers |
| `passport/` | Passport NFT hooks + Redux slice. | Minimal — just `hooks/` + one redux file |
| `ibc-history/` | IBC transfer tracking. Dexie IndexedDB, polling, WebSocket tracer. Context-based (no Redux). | Non-standard — all in flat directory |
| `ipfs/` | IPFS Drive UI + hooks (`useAddToIPFS`, `useGetIPFSHash`) + settings page. | Mixed — settings deeply nested |
| `particle/` | Particle details hook + utility. | Minimal |
| `staking/` | Staking hooks (delegations, rewards, params). No Redux, uses React Query / direct queries. | Hooks-only, no UI layer |
| `studio/` | Milkdown markdown editor with custom cyberSyntax plugin. Context-based. | Reasonable structure |
| `adviser/` | "Adviser" typing animation help text system. Context + hooks. | Good |
| `TimeHistory/` | Route/navigation history. Redux slice + components. | OK |
| `TimeFooter/` | Simple timestamp footer component. | Trivial |
| `rank/` | Just an `index.md`. No code — placeholder. | Dead |

### FSD Assessment
Feature-Sliced Design is **inconsistently applied**. `cybernet/` comes closest with `api/types/ui` layers. Most features mix concerns: `ibc-history/` has business logic (polling, WebSocket tracing) and UI (context provider) in a flat structure. `sense/` is the most developed but its redux slice doubles as both state management and business logic. There is no `shared/`, `entities/`, or `pages/` FSD layer structure at the feature level — those exist at the top `src/` level as informal folders.

---

## 7. GraphQL Usage

### Code Generation
- Config: `codegen.ts` at root
- Schema source: `https://index.bostrom.cybernode.ai/v1/graphql` (Hasura)
- Documents: all `src/**/*.graphql` files (18 total across `backend/services/indexer/graphql/` and `services/graphql/queries/`)
- Output: `src/generated/graphql.ts` — **15,589 lines** containing TypeScript types, operation documents, and React Apollo hooks (though `withHooks: true` generates hooks that are almost never used — the sync layer uses raw `graphql-request` instead)
- Plugins: `typescript` + `typescript-operations` + `typescript-react-apollo`

### Usage Pattern
- **In sync loops**: `graphql-request` `GraphQLClient` with generated `Document` objects and typed variables. Clean typed usage.
- **WebSocket subscriptions**: Apollo `ApolloClient` + `GraphQLWsLink` + generated `WsDocument` types. The `MessagesByAddressSenseWsDocument` subscription drives real-time tx sync.
- **React components**: The generated Apollo hooks (`useXxxQuery`) are **barely used** — only 4 files in the entire codebase import from `@apollo/client` outside the graphql service files. Features instead call `DbApi` (local CozoDb), not the indexer directly.
- **Dead client**: `src/services/graphql/index.ts` creates a full Apollo client instance that is not imported by anything in the active data path.
- **`src/services/graphql/queries/messagesByAddress.graphql`**: A single query file in the old `services/graphql/` location — but the real queries are all in `backend/services/indexer/graphql/`. This looks like a migration artifact.

### Verdict
GraphQL is used effectively in the sync layer (typed, document-based, both HTTP and WS). The generated React hooks (`typescript-react-apollo` plugin) are largely wasted codegen output since the architecture deliberately keeps indexer calls in workers, not components.

---

## 8. WebSocket Usage

Three distinct WebSocket mechanisms:

| Mechanism | Location | Protocol | Purpose |
|---|---|---|---|
| Tendermint JSON-RPC WS | `src/services/lcd/websocket.ts` | Raw `WebSocket` + JSON-RPC subscribe | Real-time incoming transfer events for current user. Wrapped in RxJS `Observable`. |
| GraphQL WS (Apollo) | `src/services/backend/services/indexer/utils/graphqlClient.ts` | `graphql-ws` protocol via `GraphQLWsLink` | Indexer subscription for `MessagesByAddressSense`. Has retry (10 attempts, exponential backoff). |
| IBC TracerTx WS | `src/features/ibc-history/tx/TracerTx.ts` | Tendermint WS `tx_search` | Traces IBC packet receipt/timeout on destination chain. |
| IBC PollingStatusSubscription | `src/features/ibc-history/polling-status-subscription.ts` | Tendermint REST polling | Block time polling to detect IBC timeout timestamps. |

`SyncTransactionsLoop` runs **both** the GraphQL WS and the Tendermint node WS simultaneously via `merge()`, using both as input streams. The GraphQL WS catches indexed transactions (with full metadata), while the Tendermint WS catches incoming transfers faster (but with less metadata). This dual-source design is intentional to cover both latency and completeness.

The IBC `TracerTx` was recently given a timeout fix (per git log: `fix(ibc-history): add WebSocket timeout to TracerTx`). The `HistoryContextProvider` is 375 LOC of business logic living in a React context — no Redux, no service layer separation.

---

## 9. Worker Threads

### Web Workers (2)

**DB Worker** (`src/services/backend/workers/db/worker.ts`):
- Runs `CozoDb` WASM in isolation.
- Exposed via Comlink as `CozoDbWorker`.
- In production: `SharedWorker`; in dev: regular `Worker`.
- Operations: `init`, `runCommand`, `executePutCommand`, `executeRmCommand`, `executeUpdateCommand`, `executeGetCommand`, `executeBatchPutCommand`, `importRelations`, `exportRelations`.

**Background Worker** (`src/services/backend/workers/background/worker.ts`):
- The main orchestrator.
- Contains: IPFS node + QueueManager, Rune scripting engine, ML embedding API, `SyncService`.
- Exposed via Comlink as `BackgroundWorker`.
- In production: `SharedWorker` (survives tab navigation); in dev: regular `Worker`.
- `factoryMethods.ts` handles the SharedWorker/Worker duality plus Comlink Observable/Subscription transfer handlers for passing RxJS streams across the Comlink boundary.

**Key design**: Observable streams are passed across the Comlink boundary using custom transfer handlers registered in `factoryMethods.ts`. This allows the main thread to subscribe to `embeddingApi$` (an `Observable<EmbeddingApi>`) living inside the worker.

### Service Worker
- `src/services/service-worker/service-worker.ts` — Workbox-based.
- Handles: precache of app shell, CacheFirst for static assets (24h TTL), NetworkFirst for API calls, offline document fallback, POST response caching.
- Importantly, it checks for `X-Cyb-Source: shared-worker` header to **skip caching** responses that come from the SharedWorker (to avoid double-caching IPFS content).

---

## 10. Data Flow Traces

### Trace A: Sending a Cyberlink

```
User clicks "Submit" in Studio/Search
  └── useCyberlinkWithWaitAndAdviser (features/cyberlinks/hooks/useCyberlink.ts)
        ├── useAddToIPFS(from) → backgroundWorkerInstance.ipfsApi.addContent(content)
        │     └── [Comlink] → Background Worker → ipfsInstance$.getValue().addContent()
        │           └── HeliaNode/KuboNode.add() → returns CID string
        ├── useAddToIPFS(to) → same path → returns CID string
        └── useCyberlink({ fromCid, toCid }) [useMutation / react-query]
              └── sendCyberlink(address, from, to, { signingClient, senseApi })
                    ├── signingClient.cyberlink(neuron, from, to, fee)
                    │     └── @cybercongress/cyber-js → broadcasts tx to chain
                    ├── senseApi.putCyberlink(link) → dbApi.putCyberlinks(link)
                    │     └── [Comlink] → DB Worker → CozoDb.put('link', ...)
                    └── senseApi.addCyberlinkLocal(link)
                          ├── dbApi.putTransactions([syntheticTx])
                          ├── dbApi.putSyncStatus(newItem)
                          └── new BroadcastChannelSender().postSenseUpdate([newItem])
                                └── BroadcastChannel → Main Thread
                                      └── RxBroadcastChannelListener → dispatch(updateSenseList(...))
                                            └── sense.redux.ts reducer → state.chats updated
                                                  └── Sense UI re-renders (useAppSelector)
```

### Trace B: Sense Inbox Loading (on login)

```
User connects wallet → myAddress set in pocket slice
  └── BackendProvider useEffect [myAddress]
        └── backgroundWorkerInstance.setParams({ myAddress })
              └── [Comlink] → Background Worker → params$.next({ myAddress })
                    └── SyncTransactionsLoop.isInitialized$ emits true
                          └── initSync() runs:
                                ├── fetchTransactionMessagesCount(address, ...) [GraphQL HTTP]
                                ├── fetchTransactionsIterable(...) [GraphQL HTTP, paginated]
                                │     └── For each batch: dbApi.putTransactions(batch)
                                │                         dbApi.putSyncStatus(...)
                                ├── syncMyChats(dbApi, myAddress, ...)
                                └── channelApi.postSenseUpdate(syncStatusItems)
                                      └── BroadcastChannel → dispatch(updateSenseList)
                                            └── sense.redux reducer populates state.chats

  └── useSenseManager (features/sense/ui/useSenseManager.ts)
        └── dispatch(getSenseList(senseApi)) [createAsyncThunk]
              └── senseApi.getList()
                    └── dbApi.getSenseList(myAddress)
                          └── [Comlink] → DB Worker → CozoDb.run(complexCozoQuery)
                                └── Returns joined sync_status + transaction + particle data
              └── getSenseList.fulfilled reducer → state.list.data = [ids...]
                                                   state.chats[id] = { data: [...] }
                    └── SenseList component re-renders (useAppSelector(store => store.sense))

  Meanwhile, SyncTransactionsLoop switches to live mode:
    ├── createIndexerWebsocket(MessagesByAddressSenseWsDocument, vars)
    │     └── Apollo WS subscription → on new tx:
    │           └── onUpdate() → processBatchTransactions() → dbApi.putTransactions()
    │                            postSenseUpdate() → BroadcastChannel → dispatch(updateSenseList)
    └── createNodeWebsocketObservable(address, incomingTransfersQuery)
          └── Raw Tendermint WS → on new transfer:
                └── same onUpdate() path
```

---

## Summary: Architectural Strengths and Concerns

### Strengths
- **Worker isolation is excellent**: DB and background processing are fully off the main thread. SharedWorker in production means state survives tab switches.
- **Comlink usage is sophisticated**: Observable transfer handlers across worker boundaries are non-trivial and well-implemented.
- **Typed GraphQL pipeline**: codegen → typed document objects → typed request functions is consistently applied in the sync layer.
- **QueueManager design**: priority + multi-source fallback + per-source concurrency is well-architected for the IPFS use case.
- **RxJS used deliberately**: sync loops use `combineLatest` / `BehaviorSubject` / `merge` cleanly to compose initialization conditions and live streams.

### Concerns

1. **Three IPFS libraries in production**: `helia`, `ipfs-core` (archived), `kubo-rpc-client` are all bundled. `JsIpfsNode` should be removed; `SyncIpfsLoop` is commented out but its imports remain.

2. **No central API client**: REST uses bare axios, GraphQL uses `graphql-request` per-call with no caching, and there is a dead second Apollo client instance (`services/graphql/index.ts`). This makes request deduplication, error handling, and auth injection impossible to centralize.

3. **`sense.redux.ts` is overloaded (574 LOC)**: LLM thread management, optimistic updates, unread counts, and the list/chat data cache are all one slice. The LLM thread state writing to `localStorage` inside a reducer is a direct anti-pattern.

4. **`backendReducer` is not RTK**: It consumes raw BroadcastChannel message objects as Redux actions, which works but means no RTK `createSlice` benefits (Immer, action creators, type safety). The `assocPath` from Ramda is a workaround for the lack of Immer.

5. **Side effects in multiple reducers**: `pocket`, `warp`, `ibcDenom`, `scripting` all call `localStorage.setItem` from within reducer bodies.

6. **`gol` reducer is dead code**: Game of Links ended years ago; no dispatchers remain for `CHANGE_GOL_*` actions.

7. **`currentAccount` slice duplicates `backend.community`**: Two Redux slices hold the same following/followers/friends data.

8. **`HistoryContextProvider` (375 LOC)** packs WS tracing, Dexie DB access, polling subscriptions, and React state all into one React context. It has no service layer separation and has already accumulated multiple bug fixes (per git log).

9. **Generated Apollo hooks unused**: `typescript-react-apollo` plugin generates 15k-line output including hooks that are structurally unusable because indexer queries happen in workers, not components. The codegen config should drop this plugin.

10. **`src/services/graphql/index.ts`**: Dead Apollo client. Should be removed.

11. **`features/rank/`**: Contains only an `index.md`. Placeholder with no code — either implement or remove.

12. **`src/services/graphql/queries/messagesByAddress.graphql`**: Stale query file in the wrong location; the actual query used is in `backend/services/indexer/graphql/`.


---

## 5. Security & Performance Audit

# Security & Performance Audit — cyb-ts/src

---

## SECURITY

---

### 1. XSS Vectors

**No `dangerouslySetInnerHTML` found.** This is good — the React codebase avoids this API entirely.

**`innerHTML` is used, but only in a vendored/bundled JS library:**

```
src/features/adviser/Adviser/typeit.js:225  doc.body.innerHTML = content;
src/features/adviser/Adviser/typeit.js:434  character.innerHTML = '';
src/features/adviser/Adviser/typeit.js:1132 this.element.innerHTML = '';
src/features/adviser/Adviser/typeit.js:1134 this.element.innerHTML = existingMarkup;
src/features/adviser/Adviser/typeit.js:1162 cursor.innerHTML = getParsedBody(this.opts.cursorChar).innerHTML;
```

The `typeit.js` file appears to be a bundled/vendored third-party library (TypeIt). Line 225 (`doc.body.innerHTML = content`) and line 1134 (`this.element.innerHTML = existingMarkup`) accept string content. If the `cursorChar` option or initial content ever comes from user input, this is an XSS vector. **Verdict:** Low direct risk currently, but any user-controlled string passed as `opts.cursorChar` or as markup to TypeIt would be injected unsanitized. Review what feeds into this library.

```
src/components/loader/loader.js:251  progressData.innerHTML = `Loading: <span>${Math.round(...)}</span>`;
```

This is interpolating a `Math.round()` result — always a number — so no XSS risk here.

**`iframe` usage (IPFS content rendering):**

```
src/components/Iframe/Iframe.tsx:18  sandbox={['allow-scripts', 'allow-same-origin']}
```

This is the **most significant XSS/security risk** in the codebase. The general-purpose IPFS content viewer uses `allow-scripts` AND `allow-same-origin` together. Combining these two sandbox flags effectively **negates the sandbox** — a malicious IPFS document can escape the sandbox and access `localStorage`, cookies, and the parent page's DOM.

```
src/components/contentIpfs/component/gateway/index.tsx:7  <Iframe url={url} />
```

The `url` here is an IPFS gateway URL built from user-supplied CIDs. Any IPFS content rendered through this path runs with full same-origin access.

The Movie page is safer:
```
src/containers/movie/index.jsx:62  sandbox="allow-scripts"   // no allow-same-origin — correct
```

**Fix:** Remove `allow-same-origin` from `Iframe.tsx`. If same-origin is genuinely needed, use a separate subdomain for IPFS gateway content.

---

### 2. Secret Handling — Private Keys, Mnemonics, Passwords

**No mnemonics or private keys found stored in localStorage.** The codebase exclusively uses Keplr for key management — private keys never touch the app.

**What IS stored in localStorage:**

| Key | File | Concern |
|---|---|---|
| `pocket` / `pocketAccount` | `src/redux/features/pocket.ts:46,53` | Stores bech32 addresses and public key hex (`pk`) — not sensitive but worth noting |
| `llmThreads` | `src/features/sense/redux/sense.redux.ts:377` | LLM conversation history stored in plaintext |
| `ipfsState` | `src/features/ipfs/ipfsSettings/...` | IPFS node config |
| `CHAIN_PARAMS` | `src/contexts/networks.tsx:33` | Network config |
| `marketData`, `lastCap`, `lastPoolCap` | various | Cached market data |

**The `pocketAccount` entry stores the public key:**
```typescript
// src/containers/portal/citizenship/ActionBar.tsx:77-105
const { bech32Address, pubKey, name } = await signer.keplr.getKey(chainId);
const pk = toHex(new Uint8Array(pubKey));
// ...
accounts.cyber = { bech32: bech32Address, keys: 'keplr', pk, path: LEDGER.HDPATH, name };
localStorage.setItem('pocketAccount', JSON.stringify(pocketAccount));
```

Public keys are not secret, but storing derivation paths (`LEDGER.HDPATH`) is unnecessary metadata exposure.

**OpenAI API key exposure risk:**
```typescript
// src/features/sense/ui/VoiceInteraction/VoiceInteraction.tsx:66
const apiKey = process.env.REACT_APP_OPENAI_API_KEY;
// ...
headers: { Authorization: `Bearer ${apiKey}` }
```

`REACT_APP_` prefixed variables are **bundled into the client-side JavaScript** by Create React App / webpack. The OpenAI key will be visible in the built bundle to anyone who inspects the source. This should be proxied through a server-side function instead.

---

### 3. Input Validation — eval, Function(), Dynamic Script Injection

**No `eval()` or `new Function()` calls found** in application code.

The `typeit.js` library is a compiled bundle and doesn't use eval in its surface.

No dynamic `<script>` tag injection was found in application code (the analytics script injection in `index.html` is static and hardcoded).

**Input handling in the Commander (search bar):**
```typescript
// src/containers/application/Header/Commander/Commander.tsx
function onChange(event) {
  const { value } = event.target;
  dispatch(setValue(value.replace(fixedValue, '')));
}
```

Input is passed to Redux state and eventually to router navigation — this is safe since React Router handles URL encoding and React's JSX rendering auto-escapes.

---

### 4. CSP Compliance

**File:** `/Users/mastercyb/git/cyb-ts/src/index.html`

The CSP header is defined, but has serious issues:

```html
script-src 'self' 'unsafe-eval' 'unsafe-inline' https://metrics.cyb.ai;
style-src 'self' 'unsafe-inline';
```

**`unsafe-eval` is present** — this is required by webpack/bundlers but allows any `eval()`-based code execution, negating XSS protections for scripts.

**`unsafe-inline` for both scripts and styles** — completely disables inline script and style protections. This is the primary CSP protection and it is effectively off.

**Inline scripts in index.html violate CSP intent** (though `unsafe-inline` is declared to allow them):

```html
<!-- index.html:45-57 -->
<script>
  if ('<%= NODE_ENV %>' === 'production') {
    const commitSHA = document.querySelector('meta[name="commit-version"]').content;
    // ...
  }
</script>

<script>
  window.cyb = {};
</script>

<!-- body -->
<script>
  const script = document.createElement('script');
  script.src = 'https://metrics.cyb.ai/js/script.js';
  script.defer = true;
  document.body.appendChild(script);
</script>
```

The Plausible analytics script is **dynamically injected** via `createElement`+`appendChild` rather than a static `<script>` tag. This means it wouldn't even need to be in the `connect-src` allowlist — it bypasses the declared `script-src`. The cleaner approach is a static `<script defer src="...">` tag.

**`connect-src` is quite permissive** — it lists many external RPC endpoints by full hostname. This is fine for operational reasons but worth auditing as endpoints change.

---

### 5. External URL Handling — Open Redirect Risks

**`window.open` calls:**

```typescript
// src/containers/portal/citizenship/ActionBar.tsx:145
const newWindow = window.open(url, '_blank', 'noopener,noreferrer');
if (newWindow) { newWindow.opener = null; }
```

This is the **correct** pattern — `noopener,noreferrer` and explicit `opener = null`.

```javascript
// src/containers/forceGraph/forceQuitter.jsx:84,95
window.open(...)  // no noopener/noreferrer seen
```

```typescript
// src/features/cyberlinks/CyberlinksGraph/CyberlinksGraph.tsx:145,149
window.open(`${window.location.origin}/ipfs/${node.id}`, '_blank');
window.open(...)
```

The graph opens URLs built from `node.id` (an IPFS CID from the graph) without `noopener,noreferrer`. While CIDs are hashes and not user-typed strings, this is still missing the security attributes.

**No open redirect via query parameters was found** — the app uses React Router's `<Navigate>` and `navigate()` for internal redirects, which are path-based.

---

### 6. Wallet Security — Keplr Integration

The Keplr integration is **architecturally sound** — private keys never leave Keplr's extension context.

**How it works:**
```typescript
// src/contexts/signerClient.tsx:103-133
const getOfflineSigner = useCallback(async (chainId) => {
  const windowKeplr = await getKeplr();
  await windowKeplr.experimentalSuggestChain(configKeplr(_BECH32_PREFIX));
  await windowKeplr.enable(CHAIN_ID);
  const offlineSigner = await windowKeplr.getOfflineSignerAuto(_CHAIN_ID);
  return offlineSigner;
}, [selectAddress]);
```

The app uses `getOfflineSignerAuto` — this returns a signer that delegates signing to Keplr's UI, never exposing the key material.

**Keystroke change listener is properly cleaned up:**
```typescript
// src/contexts/signerClient.tsx:159-161
window.addEventListener('keplr_keystorechange', handleKeystoreChange);
return () => {
  window.removeEventListener('keplr_keystorechange', handleKeystoreChange);
};
```

**One concern — `getKey()` called in multiple places returns `pubKey` as a `Uint8Array`:**
```typescript
// src/containers/portal/citizenship/ActionBar.tsx:77-78
const { bech32Address, pubKey, name } = await signer.keplr.getKey(chainId);
const pk = toHex(new Uint8Array(pubKey));
```

Public keys are not secret, but this pattern is copy-pasted across 4+ files (`ActionBar.tsx`, `ActionBarPortalGift.tsx`, `ActionBarRelease.tsx`, `signerClient.tsx`). Consolidating into a single utility would reduce the attack surface.

**`window.keplr` is accessed directly in one legacy file:**
```typescript
// src/containers/portal/citizenship/index.tsx:218
const offlineSigner = window.getOfflineSigner(CHAIN_ID);
```

`window.getOfflineSigner` is a deprecated Keplr API. This should use `window.keplr.getOfflineSignerAuto()`.

---

## PERFORMANCE

---

### 7. Bundle Size — No Lazy Loading of Routes

**The entire router is statically imported.** Every page component is a top-level static import in `src/router.tsx`:

```typescript
// src/router.tsx — ALL static imports
import Home from './containers/home/home';
import Story from './containers/story/story';
import TxsDetails from './containers/txs/txsDetails';
import Ipfs from './containers/ipfs/ipfs';
import BlockDetails from './containers/blok/blockDetails';
import ForceQuitter from './containers/forceGraph/forceQuitter';
import Temple from './containers/temple/Temple';
import WarpDashboardPools from './containers/warp/WarpDashboardPools';
import Warp from './containers/warp/Warp';
// ... ~30 more static page imports
```

**Zero `React.lazy()` / dynamic `import()` is used for routes.** The search for lazy imports returned only 2 matches in non-routing files. This means the entire application — all pages, all libraries — is bundled into one JavaScript chunk. The initial load downloads and parses code for pages the user may never visit.

Heavy pages that should be lazy-loaded include:
- `ForceQuitter` (3D force graph — likely pulls in `three.js` or similar)
- `CyberlinksGraph` (also a graph library)
- `Temple` with its canvas animations
- `FreestyleIde` (code editor)
- `StudioWrapper`
- `GovernanceRoutes`

**Large SVG assets bundled as imports:**

```
src/image/new_icons/warp.svg     — 289 KB
src/image/new_icons/sphere.svg   — 141 KB
```

Complex SVGs approaching 300 KB are extremely large. These should be simplified, converted to raster at their display size, or lazy-loaded.

**Large unoptimized PNGs:**

```
src/image/temple/fast-growing-vibrant-economy.png   — 1.62 MB
src/image/temple/permissionless-onchain-learning.png — 500 KB
src/image/temple/autonomous-semantic-programs.png    — 433 KB
src/image/temple/next-gen-censor-free-content-resolver.png — 399 KB
src/image/temple/superintelligent-moon-network-state.png   — 373 KB
src/image/temple/ever-growing-planet-scale-singleton.png   — 373 KB
src/image/temple/collaborative-neural-network.png   — 321 KB
src/image/temple/provable-efficient-and-fair-game.png — 301 KB
```

The Temple page alone carries **4+ MB of uncompressed PNG images**. These should be converted to WebP and lazy-loaded with `loading="lazy"`.

---

### 8. Lazy Loading — Routes and Heavy Components

As noted, **no route-level code splitting exists.** The recommended fix:

```typescript
// Current (bad):
import Temple from './containers/temple/Temple';

// Should be:
const Temple = React.lazy(() => import('./containers/temple/Temple'));
// wrapped in <Suspense fallback={<Loading />}>
```

The `CyberlinksGraph` component uses `setTimeout` + `setInterval` for animation internally but is not lazy-loaded:

```typescript
// src/features/cyberlinks/CyberlinksGraph/CyberlinksGraph.tsx:91-92
const timeout = setTimeout(() => {
  interval = setInterval(() => { ... }, ...);
```

The `EPubView` component and PDF viewer also appear to be heavy candidates for lazy loading.

---

### 9. Re-render Patterns

**React.memo / useMemo / useCallback usage: 461 occurrences across 152 files.** The team does use memoization, but several patterns are concerning.

**`robot.context.tsx` — accumulating refetch functions in state:**
```typescript
// src/pages/robot/robot.context.tsx:45
const [refetchFuncs, setRefetch] = useState<(() => void)[]>([]);

// src/pages/robot/robot.context.tsx:170-173
const addRefetch = useCallback(
  (func: () => void) => {
    setRefetch((items) => [...items, func]);
  }, [setRefetch]
);
```

Every call to `addRefetch` creates a new array and triggers a state update on `RobotContext`, which causes every consumer to re-render. This accumulates indefinitely — there is no mechanism to remove functions from the array. If child tabs call `addRefetch` on mount (confirmed at `feeds.tsx:42`, `heroes.jsx:74`, `useMenuCounts.tsx:69`), navigating between tabs repeatedly will grow the array without bound.

**`BackendProvider` — large context with many state values:**
```typescript
// src/contexts/backend/backend.tsx
// valueMemo deps: isReady, isIpfsInitialized, isDbInitialized, isSyncInitialized,
//                ipfsError, senseApi, dbApi, ipfsApi
```

The backend context is consumed by many components. Any status change (e.g., IPFS initializing) triggers a re-render of all consumers. Consider splitting into smaller contexts (e.g., a dedicated `IpfsStatusContext`).

**Missing `useCallback` on event handlers in class-pattern components:**

```typescript
// src/containers/application/Header/Commander/Commander.tsx
// onChange and submit are defined inline in the function body without useCallback
function onChange(event) { ... }
function submit(event) { ... }
```

These recreate on every render and are passed to child `Input` component.

---

### 10. Memory Leaks

**Event listeners without cleanup — confirmed leaks:**

```javascript
// src/containers/movie/index.jsx:38-40 — NO CLEANUP
useEffect(() => {
  const iframeTag = document.querySelector('iframe');
  if (iframeTag) {
    iframeTag.addEventListener('load', () => { setLoading(false); });
    // NO removeEventListener in return function
  }
}, []);
```

The `load` listener on the iframe is never removed.

```javascript
// src/containers/temple/pages/play/PlayBanerContent.jsx:47
animID = setInterval(typeLetter, delay);
// Is clearInterval called? Need to verify cleanup.
```

**`setInterval` cleanup count vs usage:**
- `setInterval` calls found: ~12 locations
- `clearInterval` calls found: 18 (across 18 files — looks sufficient, but several of the `setInterval` in `PlayBanerContent.jsx` use a module-scope `animID` variable that may not clean up across re-renders)

**RxJS subscriptions — most are cleaned up, but `BaseSyncLoop` is not:**

```typescript
// src/services/backend/services/sync/services/BaseSyncLoop/BaseSyncLoop.ts:61
public start() {
  this.loop$.subscribe(() => this.statusApi.sendStatus('active'));
  return this;
}
```

`start()` subscribes to `loop$` with no stored reference and no `.unsubscribe()`. This is a service-layer singleton so it probably lives for the app's lifetime, but it's still not disposable.

**`PollingStatusSubscription` — polling loop with no AbortController:**

```typescript
// src/features/ibc-history/polling-status-subscription.ts:32-43
while (this._subscriptionCount > 0) {
  await new Promise(resolve => setTimeout(resolve, currentInterval));
  const response = await this.rpcInstance.get('/status');
  // ...
}
```

The `while` loop with `await setTimeout` cannot be aborted mid-sleep. If `decreaseSubscriptionCount()` is called while sleeping, the loop exits after the current sleep completes, but the in-flight `axios.get('/status')` has no cancellation token. For a 60-second sleep interval, this means up to 60 seconds of orphaned polling after the user navigates away.

**`blockSubscriberMap` in `historyContext.tsx` — module-level Map:**

```typescript
// src/features/ibc-history/historyContext.tsx:78
const blockSubscriberMap: Map<string, PollingStatusSubscription> = new Map();
```

This `Map` is declared at module scope outside the React component. Entries are added when IBC transfers are initiated, but it's unclear if they are ever cleaned up when the `HistoryContextProvider` unmounts. Module-scope state survives HMR and component lifecycle.

---

### 11. Network — Polling, Unnecessary API Calls, Caching

**Active polling loops found:**

| Location | Interval | Purpose |
|---|---|---|
| `src/components/NewVersionChecker/NewVersionChecker.tsx:62` | 3 min | GitHub API for new commits — calls `api.github.com` on every app load |
| `src/features/ibc-history/polling-status-subscription.ts` | 7.5s base | IBC tx status — has exponential backoff and circuit breaker (good) |
| `src/services/relayer/relay.ts:14,100` | 4s | Relayer poll loop |
| `src/pages/oracle/landing/Stats/Stats.tsx:17` | 7s | Graph stats and negentropy |
| `src/containers/temple/hooks/useGetPortalStats.js:53` | 3 min | Portal stats |
| `src/containers/temple/hooks/useGetValidatorsBonded.js:39` | 3 min | Validator data |
| `src/hooks/useGetMarketData.ts:87` | 3 min | Market data |
| `src/containers/warp/Warp.tsx:51` | 50s | Pool list |
| `src/pages/teleport/swap/swap.tsx:48` | 5 min | Pool list for swap |
| `src/features/TimeFooter/TimeFooter.tsx:35` | via `setInterval` | Block time footer |
| `src/features/cybernet/ui/cybernet.context.tsx:55` | 20s (main page only) | Cybernet data |

The `NewVersionChecker` polls GitHub's API every 3 minutes for every user. This is an unauthenticated API call that counts against GitHub's rate limit (60 req/hour per IP). On a corporate network where many users share an IP, this will hit rate limits. Consider using a webhook or ETag-based conditional request.

**`useGetMarketData.ts` — localStorage as network cache:**
```typescript
// src/hooks/useGetMarketData.ts:93
const marketDataLS = localStorage.getItem('marketData');
// ...
localStorage.setItem('marketData', JSON.stringify(obj));
```

This is used as a stale-while-revalidate cache, which is a reasonable pattern. However, there is no cache invalidation strategy other than the `refetchInterval`. If the stored data is corrupt JSON, `JSON.parse` may throw without a try/catch.

**Multiple data sources for the same data:**
`usePoolsAssetAmount.ts` and `warp.ts` both cache pool data to localStorage with different keys (`lastPoolCap`, `lastPoolData`, `pools-warp`). There are redundant fetches of pool data across the Warp and Teleport sections.

---

### 12. Images / Assets

**Total oversized assets:**

```
fast-growing-vibrant-economy.png    1.62 MB  — should be WebP, max ~100KB
permissionless-onchain-learning.png  500 KB
autonomous-semantic-programs.png     433 KB
next-gen-censor-free-content-resolver.png 399 KB
superintelligent-moon-network-state.png   373 KB
ever-growing-planet-scale-singleton.png   373 KB
collaborative-neural-network.png     321 KB
provable-efficient-and-fair-game.png 301 KB
warp.svg                             289 KB  — SVG should not be 289KB
stars.png                            269 KB
cyber.png                            177 KB
sphere.svg                           141 KB  — SVG should not be 141KB
```

The Temple page images alone total **~4.3 MB** uncompressed. At typical compression ratios for PNG-to-WebP conversion, this could be reduced to under 800 KB total.

The `warp.svg` at 289 KB is abnormally large for an SVG icon — it likely contains embedded raster data or excessive path complexity. SVG icons should be under 10 KB.

No `loading="lazy"` attributes were found on `<img>` tags (the codebase uses React components, so this would need to be checked per image component, but no `loading` prop was seen in image component definitions).

---

## Summary of Critical Issues

**Security — fix immediately:**

1. **`src/components/Iframe/Iframe.tsx:18`** — `allow-scripts` + `allow-same-origin` together negate the iframe sandbox. Any malicious IPFS content has full same-origin DOM/storage access.
2. **`src/features/sense/ui/VoiceInteraction/VoiceInteraction.tsx:66`** — `REACT_APP_OPENAI_API_KEY` is bundled into client JavaScript and visible to all users.
3. **`src/index.html`** — CSP uses `unsafe-eval` + `unsafe-inline` for scripts, making the entire CSP ineffective against XSS.

**Security — fix soon:**

4. **`src/containers/portal/citizenship/index.tsx:218`** — Deprecated `window.getOfflineSigner` API.
5. **`src/containers/forceGraph/forceQuitter.jsx`** and **`CyberlinksGraph.tsx`** — `window.open()` missing `noopener,noreferrer`.

**Performance — fix immediately:**

6. **`src/router.tsx`** — Zero route-level code splitting. All ~30 pages load upfront. Convert to `React.lazy()` + `<Suspense>`.
7. **`src/image/temple/*.png`** — 4.3 MB of PNGs. Convert to WebP and add `loading="lazy"`.
8. **`src/image/new_icons/warp.svg`** (289 KB) and `sphere.svg` (141 KB) — Abnormally large SVGs.

**Performance — fix soon:**

9. **`src/pages/robot/robot.context.tsx:45`** — `refetchFuncs` array grows without bound, each entry triggering context re-renders.
10. **`src/containers/movie/index.jsx:38`** — iframe `load` event listener never cleaned up.
11. **`src/features/ibc-history/historyContext.tsx:78`** — Module-scope `blockSubscriberMap` not cleaned up on unmount.
12. **`src/components/NewVersionChecker`** — Polling GitHub API every 3 minutes per user, unauthenticated.
