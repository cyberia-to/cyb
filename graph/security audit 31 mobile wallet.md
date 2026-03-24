---
tags: cyb, security, audit
crystal-type: log
crystal-domain: cyb
---

## scope

Mobile wallet support: mnemonic import, Ledger signer, encrypted storage, auto-lock, scripting secrets. Audit covers all code paths relevant to mobile browsers and Tauri desktop.

## findings and status

| # | severity | finding | status |
|---|----------|---------|--------|
| 1 | critical | `setMnemonic()` stored raw mnemonic as plaintext in localStorage (Tauri path) | fixed — `ba67423d` |
| 2 | high | `getMnemonic()` returned both plaintext and encrypted data from the same key | fixed — `ba67423d` |
| 3 | high | signer object retains mnemonic in memory via `DirectSecp256k1HdWallet.mnemonic` getter | not fixable (cosmjs limitation), mitigated by auto-lock |
| 4 | high | `dispatch`, `addAddressPocket`, `setDefaultAccount` not imported in Tauri bootstrap — runtime crash | fixed — `ba67423d` |
| 5 | medium | `generateMnemonic` not exported from `offlineSigner.ts` — TypeError in Tauri | fixed — `ba67423d` |
| 6 | medium | no logout/wipe function to clear all `cyb:mnemonic:*` keys | fixed — `ba67423d`, `79783db3` |
| 7 | medium | scripting secrets (API keys) stored as plaintext JSON in localStorage | fixed — `055f343d` |

## fixes applied

### encrypt mnemonic in Tauri path (`ba67423d`)

- removed `setMnemonic()` and `getMnemonic()` entirely from `src/utils/utils.ts`
- added `getTauriDeviceKey()` in `src/utils/mnemonicCrypto.ts` — random 32-byte key for password-less encryption
- Tauri bootstrap now encrypts mnemonic with device key via AES-256-GCM + PBKDF2 before storing
- auto-switch uses `mnemonicRef` on web (requires unlock), device-key decrypt on Tauri
- added `generateMnemonic()` export to `src/utils/offlineSigner.ts`
- added `removeAllMnemonics()` to `src/utils/utils.ts` for full wallet wipe
- fixed all missing imports in `signerClient.tsx`: `useAppDispatch`, `addAddressPocket`, `setDefaultAccount`

### fire wallet lock on deletion (`79783db3`)

- `deleteAddress` reducer dispatches `__cyb_wallet_locked` event so signerClient drops the active signer immediately

### migrate legacy plaintext + obfuscate secrets (`055f343d`)

- Tauri bootstrap detects old plaintext `cyb:mnemonic` key (word count >= 12), migrates to encrypted, removes old key
- removed dead `localStorageKeys.signer.mnemonic` constant
- scripting secrets obfuscated in localStorage using XOR with device key
- auto-migrates old plaintext secrets on first load (JSON.parse fallback)

## what already worked well

- AES-256-GCM + PBKDF2 (1M iterations) encryption in `mnemonicCrypto.ts`
- auto-lock: 15-min timer + visibility change handler
- mnemonic stored in `useRef` not `useState` (invisible to React DevTools)
- clipboard cleared after mnemonic paste
- error messages sanitized — no mnemonic/key/password leaks
- Ledger handles WebUSB absence gracefully on mobile

## verification pass (re-audit)

13 files audited. 0 critical, 0 high, 0 medium findings. 3 low (inherent JS limitations):

| # | file | finding | severity |
|---|------|---------|----------|
| 1 | `signerClient.tsx:135,159` | console logs mention "mnemonic" as label, not the value | low |
| 2 | `signerClient.tsx:117-230` | plaintext mnemonic variable in JS heap until GC (same as MetaMask/Keplr) | low |
| 3 | `DownloadSection.tsx:85-96` | decrypted mnemonic in closure scope until GC (same inherent limitation) | low |

Verified:
- `getObfuscationKey()` and `getTauriDeviceKey()` share the same `cyb:device-key` — compatible
- no console.log or error message leaks mnemonics, passwords, or keys
- legacy plaintext migration guard (`split(' ').length >= 12`) rejects encrypted base64 — safe
- obfuscation fallback in `loadJsonFromLocalStorage` is read-only — cannot corrupt data
- BroadcastChannel never transmits secrets — only account metadata
- Ledger transport mutex, health ping, and idle timeout all correct
- password input fields have autocomplete/autocorrect/spellcheck disabled
- mnemonic paste clears clipboard
- `deleteAddress` removes encrypted mnemonic and fires lock event

## known limitations

- finding 3: signer object holds mnemonic in memory until garbage collected — inherent to `@cosmjs/proto-signing` `DirectSecp256k1HdWallet`. mitigated by auto-lock (15 min) and visibility-change lock
- device key stored alongside encrypted data in localStorage — prevents casual exposure but not a determined attacker with filesystem access. the OS sandbox is the primary protection layer
- XOR obfuscation for scripting secrets is not AES-level — sufficient for preventing plaintext exposure in DevTools/backups, consistent with device key threat model
