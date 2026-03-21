import { useCallback, useRef } from 'react';
import { toUtf8 } from '@cosmjs/encoding';
import { TxRaw } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { CHAIN_ID } from 'src/constants/config';
import type { OfflineSigner } from '@cosmjs/proto-signing';
import type { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';

export type TxResult = {
  transactionHash: string;
  code: number;
  height: number;
  events: { type: string; attributes?: { key: string; value: string }[] }[];
  rawLog: string;
  gasUsed: bigint;
  gasWanted: bigint;
};

/**
 * Shared TX sender that serializes all transactions from the same account.
 * Uses local sequence tracking to avoid re-querying the chain between TXs.
 * Both proof submission and staking actions must go through this to avoid
 * sequence conflicts.
 */
export default function useSharedTxSender(
  signer: OfflineSigner | undefined,
  signingClient: SigningCosmWasmClient | undefined,
  address: string | undefined,
) {
  const seqRef = useRef<{ accountNumber: number; sequence: number } | null>(null);
  const busyRef = useRef(false);

  /**
   * Sign, broadcast, and verify a contract execute message.
   * Throws on failure. Caller is responsible for error handling.
   *
   * Waits for mutex (busyRef) — if another TX is in flight, this call
   * waits up to 60s for it to finish.
   */
  const sendContractTx = useCallback(
    async (contract: string, msg: Record<string, unknown>): Promise<TxResult> => {
      if (!signer || !signingClient || !address) {
        throw new Error('No signer/client available');
      }

      // Wait for any in-flight TX to finish (up to 60s)
      const waitStart = Date.now();
      while (busyRef.current) {
        if (Date.now() - waitStart > 60_000) {
          throw new Error('TX queue timeout — another transaction is stuck');
        }
        await new Promise<void>((r) => setTimeout(r, 500));
      }

      busyRef.current = true;
      try {
        const [account] = await signer.getAccounts();

        const encodeMsg = {
          typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract' as const,
          value: {
            sender: account.address,
            contract,
            msg: toUtf8(JSON.stringify(msg)),
            funds: [],
          },
        };
        const fee = Soft3MessageFactory.fee(10);

        // Fetch sequence on first call (or after reset)
        if (!seqRef.current) {
          const { accountNumber, sequence } = await signingClient.getSequence(account.address);
          seqRef.current = { accountNumber, sequence };
          console.log('[TxSender] Fetched sequence from chain:', sequence);
        }

        const signerData = {
          accountNumber: seqRef.current.accountNumber,
          sequence: seqRef.current.sequence,
          chainId: CHAIN_ID,
        };
        console.log('[TxSender] Signing with sequence:', signerData.sequence);

        const txRaw = await signingClient.sign(account.address, [encodeMsg], fee, '', signerData);
        const txBytes = TxRaw.encode(txRaw).finish();
        const broadcastResult = await signingClient.broadcastTx(txBytes);

        // Poll for DeliverTx inclusion (up to ~30s)
        const txHash = broadcastResult.transactionHash;
        let deliverResult = await signingClient.getTx(txHash);
        for (let attempt = 0; attempt < 10 && !deliverResult; attempt++) {
          await new Promise<void>((r) => setTimeout(r, 3000));
          deliverResult = await signingClient.getTx(txHash);
        }

        // Increment sequence — tx was broadcast (consumes sequence even if DeliverTx fails)
        seqRef.current.sequence += 1;

        if (!deliverResult) {
          throw new Error(`TX ${txHash} not confirmed within 30s — may have been dropped`);
        }
        if (deliverResult.code !== 0) {
          throw new Error(`Contract error: ${deliverResult.rawLog || `code ${deliverResult.code}`}`);
        }

        return {
          transactionHash: txHash,
          code: deliverResult.code,
          height: deliverResult.height,
          events: deliverResult.events,
          rawLog: deliverResult.rawLog,
          gasUsed: deliverResult.gasUsed,
          gasWanted: deliverResult.gasWanted,
        };
      } catch (err: any) {
        const errText = (err?.message || '').toLowerCase();

        // Sequence mismatch — re-fetch and retry once
        if (/account sequence mismatch/.test(errText)) {
          console.log('[TxSender] Sequence mismatch, re-fetching...');
          seqRef.current = null;
          // Don't retry here — let caller handle it
        } else {
          // Unknown error — reset sequence for safety
          seqRef.current = null;
        }
        throw err;
      } finally {
        busyRef.current = false;
      }
    },
    [signer, signingClient, address]
  );

  /**
   * Sign + broadcast to mempool, increment sequence, return txHash.
   * Fire-and-forget: does NOT wait for block inclusion.
   * Caller should use the verifier to track confirmation and extract events.
   * Uses the same mutex + sequence counter as sendContractTx.
   */
  const broadcastContractTx = useCallback(
    async (contract: string, msg: Record<string, unknown>): Promise<{ txHash: string }> => {
      if (!signer || !signingClient || !address) {
        throw new Error('No signer/client available');
      }

      // Wait for any in-flight TX to finish (up to 60s)
      const waitStart = Date.now();
      while (busyRef.current) {
        if (Date.now() - waitStart > 60_000) {
          throw new Error('TX queue timeout — another transaction is stuck');
        }
        await new Promise<void>((r) => setTimeout(r, 500));
      }

      busyRef.current = true;
      try {
        const [account] = await signer.getAccounts();

        const encodeMsg = {
          typeUrl: '/cosmwasm.wasm.v1.MsgExecuteContract' as const,
          value: {
            sender: account.address,
            contract,
            msg: toUtf8(JSON.stringify(msg)),
            funds: [],
          },
        };
        const fee = Soft3MessageFactory.fee(10);

        // Fetch sequence on first call (or after reset)
        if (!seqRef.current) {
          const { accountNumber, sequence } = await signingClient.getSequence(account.address);
          seqRef.current = { accountNumber, sequence };
          console.log('[TxSender] Fetched sequence from chain:', sequence);
        }

        const signerData = {
          accountNumber: seqRef.current.accountNumber,
          sequence: seqRef.current.sequence,
          chainId: CHAIN_ID,
        };
        console.log('[TxSender] Broadcasting with sequence:', signerData.sequence);

        const txRaw = await signingClient.sign(account.address, [encodeMsg], fee, '', signerData);
        const txBytes = TxRaw.encode(txRaw).finish();

        // broadcastTxSync: submits to mempool (CheckTx) and returns immediately
        const cometClient = (signingClient as any).getCometClient?.() || (signingClient as any).forceGetCometClient?.();
        if (!cometClient) {
          throw new Error('Cannot access CometClient for sync broadcast');
        }
        const syncResult = await cometClient.broadcastTxSync({ tx: txBytes });

        // CheckTx failure = TX rejected by mempool (bad sequence, out of gas, etc.)
        if (syncResult.code !== 0) {
          throw new Error(`CheckTx failed (code ${syncResult.code}): ${syncResult.log || 'unknown'}`);
        }

        // Increment sequence immediately — TX accepted into mempool
        seqRef.current.sequence += 1;

        const txHash = syncResult.hash ? Buffer.from(syncResult.hash).toString('hex').toUpperCase() : '';
        console.log('[TxSender] Mempool accepted, txHash:', txHash);
        return { txHash };
      } catch (err: any) {
        const errText = (err?.message || '').toLowerCase();

        if (/account sequence mismatch/.test(errText)) {
          console.log('[TxSender] Sequence mismatch, re-fetching...');
          seqRef.current = null;
        } else {
          seqRef.current = null;
        }
        throw err;
      } finally {
        busyRef.current = false;
      }
    },
    [signer, signingClient, address]
  );

  const resetSequence = useCallback(() => {
    seqRef.current = null;
  }, []);

  const isBusy = useCallback(() => busyRef.current, []);

  return { sendContractTx, broadcastContractTx, resetSequence, isBusy, seqRef };
}
