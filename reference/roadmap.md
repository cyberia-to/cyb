# Roadmap: project cleanup and improvements

## 1. Commit and clean up current changes
- [ ] Commit pending uncommitted changes (error handling, analytics API update, CSS fix)
- [ ] Find real code duplications and extract into shared utilities
- [ ] Fix pre-existing TS error (`reflect-metadata` in tsconfig)

## 2. Code structure
- [ ] Audit `components/`, `containers/`, `pages/`, `features/` — establish a single convention
- [ ] Define boundaries: what is a page, feature, container
- [ ] Gradually migrate to a unified structure

## 3. Dependency audit
- [ ] Find unused packages (depcheck or similar)
- [ ] Evaluate duplicates (lodash vs ramda, ethers vs web3, etc.)
- [ ] Remove unused, update critical

## 4. State management simplification
- [ ] Audit: what lives in Redux, React Query, contexts
- [ ] Define strategy: server state -> React Query, client state -> Redux/Context
- [ ] Gradually remove Redux Observable where unnecessary

## 5. TypeScript strictness
- [ ] Check tsconfig for strict mode
- [ ] Find `any` types and unused types
- [ ] Enable strict gradually, file by file if needed

## 6. Tests
- [ ] Evaluate current coverage
- [ ] Identify critical paths for testing
- [ ] Add tests for key utilities and services

## 7. Warp (DEX / Liquidity Pools)
- [x] Add APR (fees) display per pool and in dashboard summary
- [x] Show Vol 24h on every pool card (always visible, even when zero)
- [x] Add pool sorting (TVL / APR / Vol 24h)
- [x] On-chain fallback for volume data when warp-dex API is down
- [ ] PnL calculator for LP positions (entry price vs current value)
- [ ] Impermanent loss calculator
- [ ] Historical fee chart per pool
- [ ] Farming rewards contract (CosmWasm) to incentivize LPs
- [ ] Refactor: deduplicate pool caching in localStorage (warp.ts vs usePoolsAssetAmount)

## 8. Wallet / Keys
- [x] Private key import — raw secp256k1 hex key with AES-256-GCM encryption
- [x] Unlock flow detects private-key vs wallet accounts
- [x] ActionBar shows password unlock for private-key accounts
- [x] Security audit passed (see docs/security_audit_private_key_import.md)

## 9. Linting and code style
- [ ] Run ESLint, evaluate warnings/errors count
- [ ] Configure stricter rules where needed
- [ ] Run stylelint for CSS/SCSS
