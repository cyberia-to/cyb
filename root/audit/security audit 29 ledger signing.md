---
tags: cyb, operation, article
crystal-type: process
crystal-domain: cyber
date: 2026-03-21
scope: ledger signing path, transport security, mnemonic regression, error exposure, input validation, state management
audit-number: 29
auditor: Claude Opus 4.6
---
# Security Audit 29: Ledger Signing Path and Wallet Security

## Scope

Thorough audit of the cyb wallet codebase at `src/` covering six focus areas:
1. Ledger signing path (version detection, adaptive HRP, fallback, size guard, signing mutex, batch claim rewards)
2. Transport security (health check CLA, stale references)
3. Mnemonic handling (encryption, plaintext leaks, auto-lock, password-protected routes)
4. Error message exposure (information leakage, friendlyErrorMessage consistency)
5. Input validation (amounts, addresses, memos, injection risks)
6. State management (Redux store, migration idempotency, XSS vectors)

---

## Findings

### F29-01: Sign doc JSON logged to console in production

Severity: MEDIUM
Status: OPEN
File: `src/utils/ledgerSigner.ts` line 155
New finding: yes

```ts
console.log('[Ledger] sign doc JSON:', jsonStr);
```

The full serialized sign document (containing sender address, recipient address, amounts, memo, and all message data) is logged to the browser console on every Ledger signing attempt. Line 204 also logs the sign doc on error:

```ts
console.error('[Ledger] sign doc was:', jsonStr);
```

While no mnemonic is exposed, the sign doc reveals all transaction details. In a shared workstation or developer tools scenario, this leaks financial activity. The debug logging at lines 135, 156, 157, and 171 also exposes firmware version, message size, HRP prefix, and signing strategy.

Recommendation: gate all `[Ledger]` console output behind a `DEBUG` flag or remove entirely for production builds.

---

### F29-02: Transaction response object logged to console

Severity: LOW
Status: OPEN
File: `src/pages/Sphere/pages/components/ActionBarContainer/ActionBarContainer.tsx` line 90
New finding: yes

```ts
console.log('response', response);
```

The `checkTxs` helper logs the full transaction response object, which may contain `rawLog` with internal chain state. Line 113 in `useCheckStatusTx` also logs the full getTx response.

Recommendation: remove or gate behind debug flag.

---

### F29-03: Regex patterns use global flag, causing intermittent validation failures

Severity: MEDIUM
Status: OPEN
File: `src/constants/patterns.ts` lines 3-27
New finding: yes

All address validation patterns use the `g` (global) flag:

```ts
export const PATTERN_CYBER = new RegExp(`^${BECH32_PREFIX}[a-zA-Z0-9]{39}$`, 'g');
```

When a `RegExp` with the `g` flag is stored as a module-level constant and used with `.match()` or `.test()`, the regex `lastIndex` advances on each successful match. On the next call with the same regex object, matching starts from `lastIndex` instead of position 0, causing the pattern to fail every other invocation.

This affects `actionBarSend.tsx` (line 104), `actionBarConnect.tsx` (line 102), and dozens of other call sites across the codebase. The result is a validation gate that alternates between working and failing on consecutive renders. In the send flow, this means a valid bech32 address could be rejected on every other keystroke, and in theory a user could trigger a send when the check spuriously passes on an invalid string (though the chain would reject it).

Recommendation: remove the `g` flag from all patterns in `src/constants/patterns.ts`. None of these patterns need global search behavior since they use `^...$` anchors and are meant for single-match validation.

---

### F29-04: Raw error `.toString()` exposed to users in multiple action bars

Severity: LOW
Status: OPEN
Files:
- `src/containers/mint/actionBar.tsx` line 67
- `src/containers/governance/actionBarDatail.tsx` line 130
- `src/pages/teleport/send/actionBar.send.tsx` line 71
- `src/pages/teleport/bridge/actionBar.bridge.tsx` lines 146, 216
- `src/containers/Search/ActionBarContainer.tsx` line 194
- `src/features/studio/ActionBar.tsx` line 167
New finding: yes

These locations use `e.toString()` directly as the user-facing error message instead of passing through `friendlyErrorMessage()`. This can expose raw JavaScript stack traces, internal error codes, and contract addresses to the user. Example:

```ts
setErrorMessage(e.toString());
```

The codebase has `friendlyErrorMessage` available and it is used correctly in the Sphere, warp, energy, bridge (rawLog path), and swap action bars. The inconsistency creates information leakage in the remaining action bars.

Recommendation: wrap all error-to-user paths through `friendlyErrorMessage()` consistently.

---

### F29-05: Health check ping uses CLA 0xe0 (dashboard) instead of 0x55 (Cosmos)

Severity: INFO
Status: OPEN (by design)
File: `src/utils/ledgerSigner.ts` lines 43, 307
New finding: yes

```ts
const response = await _transport.send(0xe0, 0x01, 0x00, 0x00);
```

Both `getTransport()` and `checkTransportHealth()` send APDU with CLA `0xe0` (Ledger dashboard/BOLOS level) with INS `0x01` (GET_VERSION). The Cosmos Ledger app CLA is `0x55`.

This is actually a reasonable design choice: the dashboard-level GET_VERSION command works regardless of which app is open, so the ping succeeds even if the Cosmos app is open (BOLOS forwards unrecognized CLAs to the active app, which responds or the OS catches it). If the ping used `0x55`, it would fail when no app is open.

However, the status word check `sw !== 0x9000` at line 48 means that if a non-Cosmos app is open and responds with a different status, the transport is incorrectly invalidated and recreated. This is harmless since the next signing attempt would fail anyway.

No action required, but documenting for completeness.

---

### F29-06: Mnemonic encryption is sound (regression check PASSED)

Severity: INFO
Status: VERIFIED SECURE
File: `src/utils/mnemonicCrypto.ts`
Previously known: continuation of audit 17-19 findings

Encryption uses AES-256-GCM via Web Crypto API with PBKDF2 key derivation:
- v2: 1,000,000 iterations (current)
- v1: 600,000 iterations (legacy, backward-compatible decryption)
- 16-byte random salt, 12-byte random IV per encryption
- Versioned binary format (1 byte version + salt + IV + ciphertext)
- Fallback logic handles ~0.8% chance of legacy salt starting with 0x01/0x02

No plaintext mnemonic in `console.log`, `console.error`, localStorage, Redux state, or error messages. Grep for `console.*mnemonic` and `console.*password` returned zero results.

---

### F29-07: Auto-lock behavior is correct

Severity: INFO
Status: VERIFIED SECURE
File: `src/contexts/signerClient.tsx`

- Mnemonic is held in `useRef` (not React state, not Redux), so it never appears in React DevTools or Redux DevTools
- 15-minute idle timer clears mnemonic and signer via `setMnemonicWithAutoClear`
- Visibility change handler clears mnemonic when tab is hidden (line 155)
- Correctly skips auto-lock for Ledger accounts (line 155: `!isLedgerAccount`)
- Timer is cleared on unmount (line 147)
- `__cyb_wallet_locked` CustomEvent dispatched to notify UI

---

### F29-08: Ledger signing mutex and batch claim rewards are well-implemented

Severity: INFO
Status: VERIFIED SECURE
Files:
- `src/utils/ledgerSigner.ts` (signing mutex)
- `src/pages/Sphere/pages/components/ActionBarContainer/ActionBarContainer.tsx` (batch claims)

The `_signingInProgress` flag prevents health check pings from colliding with active signing APDU sequences (line 251-258 in `ReconnectingLedgerSigner.signAmino`). The flag is wrapped in a `try/finally` block to guarantee cleanup.

Batch claim rewards (line 274-308):
- Batches validators into groups of 5 (`LEDGER_CLAIM_BATCH`)
- Waits for each batch to confirm on-chain before sending the next (prevents sequence mismatch)
- Polling with 30-attempt timeout and 2-second intervals
- Errors in any batch abort the entire flow
- Size guard at line 160 prevents sign docs exceeding 10KB Ledger buffer limit

One minor concern: the polling promise at line 293 uses `setTimeout(poll, 2000)` recursively without a `cancelled` flag. If the component unmounts during polling, the promise chain continues in the background. This is not a security issue but could cause a stale state update warning.

---

### F29-09: Transport mutex has subtle race window

Severity: LOW
Status: OPEN
File: `src/utils/ledgerSigner.ts` lines 60-74
New finding: yes

The transport creation mutex at line 60 correctly prevents concurrent `TransportWebUSB.create()` calls by storing the promise in `_transportPromise`. However, between line 49 (setting `_transport = null` after failed ping) and line 60 (checking `_transportPromise`), two concurrent callers could both find `_transport === null` and `_transportPromise === null`. The first caller sets `_transportPromise` at line 64, so the second would correctly wait. But the window where both test `_transport` simultaneously (lines 35-57) could cause a double ping on a stale transport.

In practice, this is unlikely since JavaScript is single-threaded for synchronous code, and the async boundaries are well-placed. The risk is theoretical.

---

### F29-10: No negative amount validation before delegation

Severity: LOW
Status: OPEN
File: `src/pages/Sphere/pages/components/ActionBarContainer/ActionBarContainer.tsx`
New finding: yes

The delegation flow passes `amount` (a string from `InputNumber`) directly to `coin(amount, BASE_DENOM)` at line 200 without checking for negative values, zero, or non-integer amounts. While `InputNumber` uses `react-number-format` which prevents non-numeric input, it does not enforce a minimum value. The `isAllowed` callback only checks `maxValue`, not minimum.

The chain would reject negative or zero amounts, so this is not exploitable, but it creates a poor UX where the user sees a chain rejection error instead of a client-side guard.

The send flow at `actionBarSend.tsx` line 104 correctly checks `parseFloat(amountSend) > 0`.

---

### F29-11: Pocket migration (keplr to read-only) is idempotent

Severity: INFO
Status: VERIFIED SECURE
File: `src/redux/features/pocket.ts` lines 131-175

`migrateKeplrAccounts()` is called on every `initPocket()` dispatch. The migration:
- Only modifies entries where `keys === 'keplr'`
- Sets them to `keys = 'read-only'`
- Patches both `POCKET_ACCOUNT` and `POCKET` localStorage entries
- Uses a `changed` flag to avoid unnecessary writes
- Sets a one-time `cyb:keplr-migrated` localStorage flag
- Wrapped in try/catch to handle corrupt localStorage gracefully
- Running multiple times is a no-op (idempotent)

Account deletion at line 79 correctly removes the encrypted mnemonic from localStorage when a wallet account is deleted.

---

### F29-12: Redux store does not contain sensitive data

Severity: INFO
Status: VERIFIED SECURE
File: `src/redux/features/pocket.ts`

The Redux pocket slice stores only:
- Account names (string labels)
- Account objects (bech32 addresses, key type: 'wallet', 'ledger', 'read-only')
- Action bar tweet stage

No mnemonics, passwords, private keys, or encrypted blobs are stored in Redux state. Encrypted mnemonics are stored only in localStorage under `cyb:mnemonic:{bech32}`.

---

### F29-13: XSS mitigation through DOMPurify in Rune post-processing

Severity: INFO
Status: VERIFIED SECURE
File: `src/services/scripting/services/postProcessing.ts` line 52

Content returned from Rune VM scripts is sanitized with DOMPurify before being set as page content:

```ts
const sanitized = typeof mutation.content === 'string'
  ? DOMPurify.sanitize(mutation.content)
  : mutation.content;
```

The codebase does not use `dangerouslySetInnerHTML` anywhere. The `innerHTML` usage in `typeit.js` (the adviser typing animation) operates on its own controlled content, not user input.

---

### F29-14: Rune VM secrets are stripped from compiler params

Severity: INFO
Status: VERIFIED SECURE (previously CRITICAL in audit 17-18, now FIXED)
File: `src/services/scripting/engine.ts` line 130

```ts
const { secrets: _secrets, ...safeContext } = context;
```

Secrets are destructured out of the context before passing to the Rune compiler. The `scriptParams.app` object contains only `params`, `user`, and `refId`. The `getDebug()` method at line 301 also strips secrets.

The Rune default scripts (`particle.rn`, `ai.rn`) reference `cyb::context.secrets`, which is bound through the `cyb-rune-wasm` module's native context injection. This is a separate channel from the `params` JSON passed to `compile()`. The secrets are available to the sandboxed Rune VM for API key access (OpenAI), but not leaked through the debug interface or compiler params.

---

### F29-15: Address validation uses regex, not bech32 checksum

Severity: LOW
Status: OPEN
File: `src/constants/patterns.ts`
New finding: yes

Address validation patterns only check length and character set:

```ts
export const PATTERN_CYBER = new RegExp(`^${BECH32_PREFIX}[a-zA-Z0-9]{39}$`, 'g');
```

This does not verify the bech32 checksum. An address with the correct prefix and length but an invalid checksum would pass client-side validation. The chain would reject the transaction, but the user gets a confusing chain error instead of an immediate "invalid address" message.

The `fromBech32` utility in `src/utils/utils.ts` (line 104) does use the `bech32` library for proper decode/encode, but this is not called in the validation paths.

---

## Summary

| ID | Severity | Status | Finding |
|---|---|---|---|
| F29-01 | MEDIUM | OPEN | Sign doc JSON logged to console |
| F29-02 | LOW | OPEN | Transaction response logged to console |
| F29-03 | MEDIUM | OPEN | Global regex flag causes intermittent validation |
| F29-04 | LOW | OPEN | Raw error toString() in 6 action bars |
| F29-05 | INFO | BY DESIGN | Health check uses dashboard CLA 0xe0 |
| F29-06 | INFO | VERIFIED | Mnemonic encryption is sound |
| F29-07 | INFO | VERIFIED | Auto-lock behavior is correct |
| F29-08 | INFO | VERIFIED | Ledger mutex and batch claims are correct |
| F29-09 | LOW | OPEN | Transport mutex has theoretical race window |
| F29-10 | LOW | OPEN | No negative amount validation in delegation |
| F29-11 | INFO | VERIFIED | Pocket migration is idempotent |
| F29-12 | INFO | VERIFIED | Redux store has no sensitive data |
| F29-13 | INFO | VERIFIED | DOMPurify sanitizes Rune output |
| F29-14 | INFO | VERIFIED | Rune VM secrets stripped from params |
| F29-15 | LOW | OPEN | Address validation lacks bech32 checksum |

Findings by severity:
- CRITICAL: 0
- HIGH: 0
- MEDIUM: 2 (F29-01, F29-03)
- LOW: 5 (F29-02, F29-04, F29-09, F29-10, F29-15)
- INFO: 8 (verified secure or by-design)

No critical or high-severity issues found. The core signing path, mnemonic encryption, auto-lock behavior, and batch transaction handling are all implemented correctly. The main actionable items are removing production console logging of sign docs (F29-01) and fixing the global regex flag (F29-03).