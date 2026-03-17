import { useCallback, useRef } from 'react';
import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';

export type PendingTx = {
  txHash: string;
  proofHash: string;
  proofNonce: number;
  submittedAt: number;
};

type TxEvent = { type: string; attributes?: { key: string; value: string }[] };

const TX_TIMEOUT_MS = 60_000; // 60s — ~10 blocks

/**
 * Tracks pending proof TXs and verifies them asynchronously on each new block.
 * When a TX confirms, extracts miner_reward from wasm events and calls onConfirmed.
 * When a TX fails or times out, calls onFailed.
 */
export default function usePendingTxVerifier(
  signingClient: SigningCosmWasmClient | undefined,
  onConfirmed: (txHash: string, proofHash: string, reward: number) => void,
  onFailed: (txHash: string, proofHash: string, error: string) => void,
) {
  const pendingRef = useRef<Map<string, PendingTx>>(new Map());

  const addPending = useCallback((tx: PendingTx) => {
    pendingRef.current.set(tx.txHash, tx);
    console.log('[Verifier] Added pending TX:', tx.txHash.slice(0, 16), 'Pending:', pendingRef.current.size);
  }, []);

  const checkAll = useCallback(async () => {
    if (!signingClient || pendingRef.current.size === 0) return;

    const now = Date.now();
    const entries = Array.from(pendingRef.current.entries());

    console.log('[Verifier] Checking', entries.length, 'pending TX(s)');

    // Check all pending TXs concurrently
    const results = await Promise.allSettled(
      entries.map(async ([txHash, pending]) => {
        try {
          const tx = await signingClient.getTx(txHash);

          if (tx) {
            // TX found on chain
            pendingRef.current.delete(txHash);

            if (tx.code !== 0) {
              onFailed(txHash, pending.proofHash, tx.rawLog || `code ${tx.code}`);
              return;
            }

            // Extract miner_reward from events (prefer miner_reward over gross reward)
            const allEvents: TxEvent[] = [...(tx.events || [])];
            let reward = 0;
            const wasmEvent = allEvents.find(
              (e) => e.type === 'wasm' && e.attributes?.some(
                (a) => a.key === 'miner_reward'
              )
            );
            if (wasmEvent) {
              const rewardAttr = wasmEvent.attributes?.find(
                (a) => a.key === 'miner_reward'
              );
              if (rewardAttr?.value) {
                reward = Number(rewardAttr.value) / 1_000_000;
              }
            }

            console.log('[Verifier] Confirmed TX:', txHash.slice(0, 16), 'reward:', reward.toFixed(6));
            onConfirmed(txHash, pending.proofHash, reward);
          } else if (now - pending.submittedAt > TX_TIMEOUT_MS) {
            // Not found and timed out
            pendingRef.current.delete(txHash);
            console.log('[Verifier] TX timed out:', txHash.slice(0, 16));
            onFailed(txHash, pending.proofHash, `TX not confirmed within ${TX_TIMEOUT_MS / 1000}s — may have been dropped`);
          }
          // else: not found yet but within timeout — keep waiting
        } catch (err) {
          // getTx network error — keep in map, will retry next block
          console.warn('[Verifier] getTx error for', txHash.slice(0, 16), err);
        }
      })
    );

    // Suppress unused var lint — results checked via Promise.allSettled pattern
    void results;

    if (pendingRef.current.size > 0) {
      console.log('[Verifier] Still pending:', pendingRef.current.size);
    }
  }, [signingClient, onConfirmed, onFailed]);

  const pendingCount = useCallback(() => pendingRef.current.size, []);

  return { addPending, checkAll, pendingCount };
}
