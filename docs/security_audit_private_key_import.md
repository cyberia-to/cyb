# Security Audit: Private Key Import Feature

Date: 2026-05-12
Status: PASSED — 0 critical, 0 high, 0 medium, 1 low (optional)
Scope: Addition of raw secp256k1 private key import to wallet

## Changes Audited

| File | Change |
|---|---|
| `src/types/defaultAccount.d.ts` | Added `'private-key'` to keys union type |
| `src/utils/offlineSigner.ts` | `CybPrivateKeySigner` class + `getOfflineSignerFromPrivateKey()` |
| `src/pages/Keys/ActionBar/ConnectWalletModal/ConnectWalletModal.tsx` | UI tabs, private key input field |
| `src/pages/Keys/ActionBar/actionBarConnect.tsx` | Import routing, encryption, account registration |
| `src/contexts/signerClient.tsx` | Unlock and auto-switch for private-key accounts |
| `src/redux/features/pocket.ts` | Deletion cleanup for private-key accounts |

## Threat Model

| Threat | Mitigation | Status |
|---|---|---|
| Private key in React state (DevTools exposure) | Stored in `useRef`, not `useState` | FIXED |
| Key leak on unmount | Ref cleared in cleanup effect | FIXED |
| Key leak on app background (iOS screenshot) | Ref + display state cleared on `visibilitychange` | FIXED |
| Clipboard leak on paste | `navigator.clipboard.writeText('')` after paste | FIXED |
| Pending ref not cleared after success | `clearState()` zeroes `pendingMnemonicRef` and `pendingImportModeRef` | FIXED |
| Invalid key accepted | 3-layer validation: regex (64 hex), `fromHex()`, `DirectSecp256k1Wallet.fromKey()` | SECURE |
| Error messages leak key material | Generic errors only ("Invalid private key") | SECURE |
| Encryption at rest | Same AES-256-GCM + PBKDF2 (1M iterations) as mnemonic | SECURE |
| Password brute force | 8+ chars, 3/4 character classes if < 12 chars | ADEQUATE |
| Key type disclosure in Redux | `keys: 'private-key'` visible in Redux — accepted (no key material) | ACCEPTED |
| Tauri device key in localStorage | Pre-existing design trade-off, not changed | ACCEPTED |
| Auto-lock timer disabled | Pre-existing, not changed (`MNEMONIC_AUTO_CLEAR_MS` unused) | ACCEPTED |

## Encryption Format

Private key hex string is encrypted with identical format as mnemonic:
```
version(1 byte) + salt(16 bytes) + iv(12 bytes) + AES-GCM-256(plaintext)
→ base64 encoded → localStorage['cyb:mnemonic:{address}']
```

The `decryptMnemonic()` function is format-agnostic — it returns any stored plaintext. The account type (`keys` field in Redux) determines whether to call `getOfflineSignerFromMnemonic()` or `getOfflineSignerFromPrivateKey()`.

## CosmJS Validation Chain

1. `fromHex(privkeyHex)` — validates hex format, throws on non-hex or odd-length
2. `DirectSecp256k1Wallet.fromKey(privkey)` — validates 32-byte length, validates against secp256k1 curve order
3. `Secp256k1Wallet.fromKey(privkey)` — same validation for Amino signer

All three layers throw before any storage occurs.

## signArbitrary (ADR-036)

`CybPrivateKeySigner.signArbitrary()` composes `Secp256k1Wallet` (Amino) for ADR-036 signing — same MsgSignData format as `CybOfflineSigner`. The `hasSignArbitrary()` type guard works via duck-typing (`typeof signer.signArbitrary === 'function'`).

## Findings Fixed Before Commit

1. CRITICAL: Moved `privateKeyHex` from `useState` to `useRef` — prevents React DevTools exposure
2. CRITICAL: Added ref cleanup on unmount
3. HIGH: Added ref + display cleanup on `visibilitychange` (background)
4. HIGH: Added clipboard clearing on private key paste
5. HIGH: Added `pendingImportModeRef` reset in `clearState()`

## Accepted Risks

- Device key for Tauri auto-unlock stored in localStorage (pre-existing, not introduced by this change)
- No auto-lock timer (pre-existing design decision)
- Account type visible in Redux state (information disclosure only, no key material)
