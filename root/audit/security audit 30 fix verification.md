---
tags: cyb, operation, article
crystal-type: process
crystal-domain: cyber
date: 2026-03-21
scope: fix verification, raw error strings, IBC amino compatibility, transport CLA, portal Ledger paths, relay module
audit-number: 30
auditor: Claude Opus 4.6
diffusion: 0.00011233815923477823
springs: 0.00008246788381555257
heat: 0.000037649645370807764
focus: 0.00008843937383621902
gravity: 0
density: 0
---
# Security Audit 30: Fix Verification and Remaining Error Exposure

## Scope

Follow-up to [[security audit 29 ledger signing]]. Six focus areas:
1. Verify fixes applied after audit 29 (patterns, ledgerSigner, bridge, IBC balances, batch claims)
2. Remaining raw error string exposure (F29-04 continuation)
3. IBC MsgTransfer amino compatibility
4. Transport health check CLA correctness
5. Portal signArbitrary / Ledger account handling
6. Relay module import hygiene and Ledger compatibility

## 1. Verification of Recent Fixes

### patterns.ts

All 12 regex patterns confirmed free of the `g` flag. No stateful regex issues.

### ledgerSigner.ts

Six sub-checks passed:
- Sign doc JSON content is never logged (only byte count, chunk count, HRP prefix)
- Version detection falls back to modern strategy when getVersion() fails
- Adaptive HRP fallback retries with opposite mode on DataIsInvalid (0x6984)
- Message size guard throws user-friendly error at 10 KB limit
- Signing mutex (try/finally) prevents health-check APDU collision
- No sensitive data in any console output

### actionBar.bridge.tsx

Both deposit and withdraw use `MsgTransfer.fromPartial()`. Both catch blocks use `friendlyErrorMessage(e?.message || e)`. No `e.toString()` remaining.

### useGetBalancesIbc.ts

Null guard on `networkList` access: `const network = networkList[responseChainId]; if (!network?.destChannelId) return null;` -- prevents crash on unknown chain IDs.

### ActionBarContainer.tsx (Sphere batch claims)

Ledger batch claiming logic correct:
- Detects Ledger via `isLedgerSigner(signer)`
- Batches validators in groups of 5
- Polls for tx confirmation (up to 60s) before sending next batch
- Rejects on non-zero response code or timeout
- Non-Ledger path unchanged (single call for all validators)
- All error paths use `friendlyErrorMessage`

## 2. Remaining Raw Error Strings

Five action bars still expose raw `e.toString()` to users instead of `friendlyErrorMessage`:

### F30-01 governance/actionBarDatail.tsx:130 (LOW)

`setErrorMessage(e.toString())` in generateTx catch. `friendlyErrorMessage` is imported but not used here.

### F30-02 teleport/send/actionBar.send.tsx:71 (LOW)

`setErrorMessage(e.toString())` in sendOnClick catch. `friendlyErrorMessage` is not imported.

### F30-03 mint/actionBar.tsx:67 (LOW)

`setErrorMessage(e.toString())` in investmintFunc catch. `friendlyErrorMessage` is not imported.

### F30-04 features/studio/ActionBar.tsx:167 (LOW)

`setError(e.toString())` in createCyberlinkTx catch. `friendlyErrorMessage` is not imported.

### F30-05 Search/ActionBarContainer.tsx:194 (LOW)

`errorMessage: e.toString()` in generateTx outer catch. `friendlyErrorMessage` IS imported and used in confirmTx but not in the catch block.

### Informational

- `services/backend/channels/BackendQueueChannel.ts:67` -- debug log, not user-facing
- `services/scripting/runeDeps.ts:106,115` -- Rune VM error responses, may surface to script output

## 3. IBC MsgTransfer Amino Compatibility

Only one file constructs MsgTransfer: `pages/teleport/bridge/actionBar.bridge.tsx`. Both usages (deposit line 98, withdraw line 183) use `MsgTransfer.fromPartial()` which produces proper protobuf-encoded message objects compatible with amino signing.

All other transaction types use high-level `signingClient` methods (delegateTokens, sendTokens, voteProposal, etc.) which handle message encoding internally. No plain-object protobuf messages found.

## 4. Transport Health Check CLA

### F30-08 ledgerSigner.ts:43,305 (MEDIUM)

The health check sends `CLA 0xe0` (Ledger Dashboard / BOLOS) instead of `CLA 0x55` (Cosmos app):

```typescript
await _transport.send(0xe0, 0x01, 0x00, 0x00);
```

When the Cosmos app is open, it responds to `0xe0` with an error status (not `0x9000`). The health check interprets this as "transport dead" and sets `_transport = null`, forcing unnecessary transport recreation on every `getTransport()` call.

The signing mutex (`_signingInProgress`) prevents this during active signing, so the practical impact is limited to a slight delay on the first transaction after idle. The transport mutex prevents concurrent recreation.

Recommended fix: use Cosmos app CLA with getVersion:

```typescript
await _transport.send(0x55, 0x00, 0x00, 0x00);
```

## 5. Portal signArbitrary and Ledger Accounts

Both portal flows correctly guard signArbitrary behind `hasSignArbitrary()`:

- `containers/portal/citizenship/index.tsx:330` -- checks `hasSignArbitrary(signer)` before calling `signArbitrary`, shows "Ledger cannot sign messages" warning otherwise
- `containers/portal/gift/ActionBarPortalGift.tsx:190` -- same pattern with `hasSignArbitrary(chainSigner)`

The gift claim and release flows use standard `signingClient.execute()` which works with amino signing. Ledger detection via `getSignerKeyInfo()` correctly uses `isLedgerSigner()` and limits batch size to 1 message for Ledger accounts.

No code path calls `signArbitrary` on Ledger accounts.

## 6. Relay Module

`services/relayer/relay.ts` imports are clean:
- `OfflineDirectSigner` from `@cosmjs/proto-signing`
- `OfflineAminoSigner` from `@cosmjs/amino`
- `GasPrice` from `@cosmjs/stargate`
- No `@keplr-wallet` references anywhere in `services/relayer/`

Function signature accepts both signer types. The relay loop is functionally correct with Ledger signers but impractical (each transaction requires physical device confirmation in a continuous loop).

## Summary

| ID | Severity | File | Description |
|---|---|---|---|
| F30-01 | LOW | governance/actionBarDatail.tsx:130 | raw e.toString() |
| F30-02 | LOW | teleport/send/actionBar.send.tsx:71 | raw e.toString() |
| F30-03 | LOW | mint/actionBar.tsx:67 | raw e.toString() |
| F30-04 | LOW | studio/ActionBar.tsx:167 | raw e.toString() |
| F30-05 | LOW | Search/ActionBarContainer.tsx:194 | raw e.toString() (friendlyErrorMessage imported but unused in catch) |
| F30-08 | MEDIUM | ledgerSigner.ts:43,305 | health check CLA 0xe0 causes unnecessary transport recreation |

42 verification checks passed. 5 LOW findings (error string consistency). 1 MEDIUM finding (transport CLA). No CRITICAL or HIGH findings.