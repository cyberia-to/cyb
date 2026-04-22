# IPFS stream-drain timeout + error propagation

**Status:** draft
**Date:** 2026-04-22

## Problem

Particles sometimes get stuck forever in `loading…` state on pages that
consume `useParticle(cid)` (oracle/ask, particle view, search results).

Repro seen on mainnet against `Qmf6oHoADe5GRo1SENvKY3BJhFaSG6v6ocZbinA2jtQKoF`:
content is in our IPFS and served by the gateway in ~2.6s, but the page
stays on `loading…` indefinitely.

## Root cause

Structural hole in the content fetch pipeline — **stream drain has no
timeout, and errors are swallowed**. Trace:

1. `useParticle` → `fetchParticle` → `QueueManager.enqueue`
2. `QueueManager` runs `fetchIpfsContent` with a per-source timeout
   (5s db / 60s node / 21s gateway)
3. `fetchIpfsContent` returns **as soon as it has a stream handle**
   (fast — milliseconds). Queue reports `status='completed'`.
4. `useParticle` sees `queueItemStatus='completed'` and calls
   `parseArrayLikeToDetails(content, cid)` to convert the stream into
   a `Uint8Array` for MIME sniffing and rendering.
5. Inside `parseArrayLikeToDetails`, `getResponseResult` drains a
   `ReadableStream` or `AsyncIterable` with no timeout:
   - Line 127 `stream.ts`: `return reader.read().then(readStream);` —
     recursive read until `done`.
   - Line 142 `stream.ts`: `for await (const chunk of reader)` — loops
     until the iterator signals completion.
6. If a libp2p peer delivers some chunks then stalls (common on
   partial providers), the drain hangs forever. `setStatus('completed')`
   in `useParticle` (line 38) never fires. UI stays `loading…`.

Secondary bugs that aggravate it:

- `parseArrayLikeToDetails` has its try/catch commented out
  (`content.ts:80, 188-190`) — errors from downstream disappear silently.
- `useParticle` awaits the parse without `.catch()` — a reject would
  leave status stuck too, though currently the inner function returns
  `undefined` instead of throwing, so this is belt-and-suspenders.

Once a drain hangs, `enqueueParticleSave(content)` is called by the
queue manager (line 163 of `QueueManager.ts`) before the drain has
finished — but the saved object references a never-completed stream.
On next page load the cached item is also unusable, so the user sees
"never cached and failed to load again".

## Fix

Three small, independently-valuable changes:

### 1. Timeout + error-propagation in `getResponseResult`

File: `react/src/services/ipfs/utils/stream.ts`

Wrap the drain in a `Promise.race` against a timeout. On timeout, throw
a `StreamDrainTimeoutError`. Default 30s — generous for large files
over in-browser libp2p, but finite.

Also: propagate read errors instead of returning `undefined` silently.
Log with `console.error` including cid for debuggability.

### 2. Restore error handling in `parseArrayLikeToDetails`

File: `react/src/services/ipfs/utils/content.ts`

Uncomment the try/catch around the whole function. On any throw,
return the "can't parse" fallback so `useParticle` can still transition
to `completed` with gateway fallback render — never "loading forever".

### 3. Defensive `.catch()` in `useParticle`

File: `react/src/hooks/useParticle.ts`

Add `.catch(() => setStatus('error'))` on the `parseArrayLikeToDetails`
promise chain. Cheap insurance — if anything slips past #1 and #2,
the UI at least shows an error instead of spinning.

## Verification

Before shipping:

1. `deno task build` — no type errors, no new lint failures.
2. Local dev server: load
   `/oracle/ask/Qmf6oHoADe5GRo1SENvKY3BJhFaSG6v6ocZbinA2jtQKoF`.
   Expected: either renders the PNG (if drain succeeds via any source)
   or shows error after ≤30s. Never spins forever.
3. Network tab: confirm IndexedDB `content` cache does **not** get
   polluted with a broken entry when drain times out.

## Out of scope

- Actual root-cause fix (getting Helia libp2p peers to not stall) —
  that's a libp2p/Helia concern.
- Retry logic — current queue already falls through `db → node → gateway`;
  if drain timeout propagates as error, queue can advance to the next
  source on its own. Audit that separately.
- Caching improvements — `enqueueParticleSave` being called with a
  still-draining stream is a separate structural issue deserving its
  own plan.
