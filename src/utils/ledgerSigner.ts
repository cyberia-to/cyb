import TransportWebUSB from '@ledgerhq/hw-transport-webusb';
import CosmosApp from '@zondax/ledger-cosmos-js';
import { encodeSecp256k1Signature, serializeSignDoc } from '@cosmjs/amino';
import { Secp256k1Signature } from '@cosmjs/crypto';
import type { AccountData, AminoSignResponse, OfflineAminoSigner, StdSignDoc } from '@cosmjs/amino';

const HD_PATH = "m/44'/118'/0'/0/0";
const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes — signing on device can take time
const HEALTH_CHECK_TIMEOUT_MS = 3_000; // 3 seconds — ping timeout
const MAX_LEDGER_MSG_BYTES = 10_240; // 10 KB — Cosmos Ledger app buffer limit
const CHUNK_SIZE = 250; // bytes per APDU data chunk

let _transport: TransportWebUSB | null = null;
let _idleTimer: ReturnType<typeof setTimeout> | null = null;
let _transportPromise: Promise<TransportWebUSB> | null = null;
let _signingInProgress = false;

function resetIdleTimer() {
  if (_idleTimer) clearTimeout(_idleTimer);
  _idleTimer = setTimeout(() => {
    closeTransport();
  }, IDLE_TIMEOUT_MS);
}

/**
 * Get or create a WebUSB transport to the Ledger device.
 * Reuses existing transport if still alive. Requires a user gesture.
 * Uses a mutex to prevent concurrent TransportWebUSB.create() calls.
 */
export async function getTransport(): Promise<TransportWebUSB> {
  if (!navigator.usb) {
    throw new Error('Ledger requires Chrome, Edge, or the cyb.ai desktop app');
  }

  if (_transport) {
    // Skip ping if another signing operation owns the transport
    if (_signingInProgress) {
      resetIdleTimer();
      return _transport;
    }
    try {
      // Ping: Cosmos app getVersion (CLA=0x55, INS=0x00) — responds 0x9000
      const response = await _transport.send(0x55, 0x00, 0x00, 0x00);
      // Last 2 bytes = status word; 0x9000 = OK, anything else = wrong app
      const sw = response.length >= 2
        ? (response[response.length - 2] << 8) | response[response.length - 1]
        : 0;
      if (sw !== 0x9000) {
        _transport = null;
      } else {
        resetIdleTimer();
        return _transport;
      }
    } catch {
      _transport = null;
    }
  }

  // Mutex: if another call is already creating a transport, wait for it
  if (_transportPromise) {
    return _transportPromise;
  }

  _transportPromise = TransportWebUSB.create().then((t) => {
    _transport = t;
    _transportPromise = null;
    resetIdleTimer();
    return t;
  }).catch((err) => {
    _transportPromise = null;
    throw err;
  });

  return _transportPromise;
}

/**
 * Close the current transport and clear the idle timer.
 */
export async function closeTransport(): Promise<void> {
  // Never close transport while Ledger is showing "Review Transaction"
  if (_signingInProgress) return;
  if (_idleTimer) {
    clearTimeout(_idleTimer);
    _idleTimer = null;
  }
  if (_transport) {
    try {
      await _transport.close();
    } catch {
      // ignore close errors
    }
    _transport = null;
  }
}

/**
 * Get Cosmos account from the Ledger device.
 * @param prefix - bech32 prefix (default: 'bostrom')
 */
async function getCosmosAccount(prefix: string): Promise<AccountData> {
  const transport = await getTransport();
  const app = new CosmosApp(transport);
  const response = await app.getAddressAndPubKey(HD_PATH, prefix);
  return {
    algo: 'secp256k1' as const,
    address: response.bech32_address,
    pubkey: Uint8Array.from(response.compressed_pk),
  };
}

/**
 * Sign an amino sign doc using the Ledger device.
 *
 * Detects the Cosmos Ledger app version and uses the appropriate signing
 * strategy:
 *   - v2.34+  → sends HRP in the INIT chunk (required by modern firmware)
 *   - < v2.34 → path-only INIT chunk (legacy firmware)
 *
 * If the first attempt returns DataIsInvalid (0x6984), retries with the
 * opposite HRP strategy as a fallback.
 */
async function signWithLedger(
  signDoc: StdSignDoc,
  prefix: string,
  cachedPubkey?: Uint8Array
): Promise<AminoSignResponse> {
  const transport = await getTransport();
  const app = new CosmosApp(transport);

  // ── 1. Detect firmware version ────────────────────────────────────
  let ver: { major: number; minor: number; patch: number } | null = null;
  try {
    ver = await app.getVersion();
    console.log('[Ledger] Cosmos app version:', `${ver.major}.${ver.minor}.${ver.patch}`);
  } catch (e) {
    console.warn('[Ledger] Could not read app version:', e);
  }

  // ── 2. Resolve pubkey ─────────────────────────────────────────────
  let pubkey: Uint8Array;
  if (cachedPubkey) {
    pubkey = cachedPubkey;
  } else {
    const response = await app.getAddressAndPubKey(HD_PATH, prefix);
    pubkey = Uint8Array.from(response.compressed_pk);
  }

  // ── 3. Serialize sign doc ─────────────────────────────────────────
  const serialized = serializeSignDoc(signDoc);
  const message = Buffer.from(serialized);
  const jsonStr = new TextDecoder().decode(serialized);
  const numChunks = Math.ceil(message.length / CHUNK_SIZE) + 1; // +1 for INIT

  console.log('[Ledger] message length:', message.length, 'bytes,', numChunks, 'chunks');
  console.log('[Ledger] HRP (prefix):', prefix);

  // ── 4. Guard: message size ────────────────────────────────────────
  if (message.length > MAX_LEDGER_MSG_BYTES) {
    throw new Error(
      `Transaction too large for Ledger device ` +
      `(${message.length} bytes, limit ~${MAX_LEDGER_MSG_BYTES}). ` +
      `Try claiming rewards from fewer validators.`
    );
  }

  // ── 5. Sign with version-appropriate strategy ─────────────────────
  // v2.34+ expects HRP; older firmware rejects it
  const useHRP = !ver || ver.major > 2 || (ver.major === 2 && ver.minor >= 34);
  console.log('[Ledger] signing strategy:', useHRP ? 'with HRP (modern)' : 'path-only (legacy)');

  const attemptSign = async (withHRP: boolean) => {
    const signResponse = withHRP
      ? await app.sign(HD_PATH, message, prefix)
      : await app.sign(HD_PATH, message);

    const signature = Secp256k1Signature.fromDer(
      Uint8Array.from(signResponse.signature)
    ).toFixedLength();

    return {
      signed: signDoc,
      signature: encodeSecp256k1Signature(pubkey, signature),
    };
  };

  try {
    return await attemptSign(useHRP);
  } catch (err: any) {
    const isDataInvalid = err?.returnCode === 27012;

    // Fallback: if DataIsInvalid, retry with the opposite HRP mode
    if (isDataInvalid) {
      console.warn('[Ledger] DataIsInvalid — retrying', useHRP ? 'WITHOUT' : 'WITH', 'HRP');
      try {
        return await attemptSign(!useHRP);
      } catch (retryErr: any) {
        console.error('[Ledger] fallback also failed:', retryErr?.returnCode, retryErr?.errorMessage || retryErr?.message);
      }
    }

    console.error('[Ledger] sign failed:', err?.returnCode, err?.errorMessage || err?.message);
    throw err;
  }
}

/**
 * Create a one-shot signer for the Ledger (used for validation / getAccounts).
 * For signing, use ReconnectingLedgerSigner which handles transport lifecycle.
 * @param prefix - bech32 prefix (default: 'bostrom')
 */
export async function createLedgerSigner(prefix = 'bostrom'): Promise<OfflineAminoSigner> {
  // Validate transport is alive
  await getTransport();

  return {
    async getAccounts(): Promise<readonly AccountData[]> {
      return [await getCosmosAccount(prefix)];
    },
    async signAmino(_signerAddress: string, signDoc: StdSignDoc): Promise<AminoSignResponse> {
      return signWithLedger(signDoc, prefix);
    },
  };
}

/**
 * A signer that acquires a fresh transport for each signing operation.
 * Survives device sleep / disconnect — reconnects automatically when
 * the device is available again.
 */
export class ReconnectingLedgerSigner implements OfflineAminoSigner {
  private prefix: string;
  private _cachedAccounts: readonly AccountData[] | null;

  constructor(prefix = 'bostrom', cachedAccounts?: readonly AccountData[]) {
    this.prefix = prefix;
    this._cachedAccounts = cachedAccounts ?? null;
  }

  async getAccounts(): Promise<readonly AccountData[]> {
    if (this._cachedAccounts) return this._cachedAccounts;
    const account = await getCosmosAccount(this.prefix);
    this._cachedAccounts = [account];
    return this._cachedAccounts;
  }

  async signAmino(_signerAddress: string, signDoc: StdSignDoc): Promise<AminoSignResponse> {
    // Block health-check pings while Ledger shows "Review Transaction"
    _signingInProgress = true;
    try {
      // Pass cached pubkey to avoid extra APDU round-trip before signing
      const cachedPubkey = this._cachedAccounts?.[0]?.pubkey;
      return await signWithLedger(signDoc, this.prefix, cachedPubkey);
    } finally {
      _signingInProgress = false;
    }
  }
}

/**
 * Connect to a Ledger device, validate it, and return a reconnecting signer.
 * @param prefix - bech32 prefix (default: 'bostrom')
 */
export async function connectLedger(prefix = 'bostrom'): Promise<{
  signer: ReconnectingLedgerSigner;
  address: string;
  pubkey: Uint8Array;
}> {
  const account = await getCosmosAccount(prefix);

  return {
    signer: new ReconnectingLedgerSigner(prefix, [account]),
    address: account.address,
    pubkey: account.pubkey,
  };
}

/**
 * Type guard: check if a signer is a Ledger-backed signer.
 */
export function isLedgerSigner(signer: unknown): boolean {
  return signer instanceof ReconnectingLedgerSigner;
}

/**
 * Check if WebUSB is available in the current browser.
 */
export function isWebUSBSupported(): boolean {
  return typeof navigator !== 'undefined' && !!navigator.usb;
}

/**
 * Check if the Ledger transport is alive and the Cosmos app is responsive.
 * Returns true if reachable, false if device is asleep/disconnected.
 */
export async function checkTransportHealth(): Promise<boolean> {
  if (!_transport) return false;
  // Never ping during signing — APDU collision aborts the Ledger prompt
  if (_signingInProgress) return true;
  try {
    const timeout = new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('timeout')), HEALTH_CHECK_TIMEOUT_MS)
    );
    await Promise.race([
      _transport.send(0x55, 0x00, 0x00, 0x00),
      timeout,
    ]);
    resetIdleTimer(); // successful ping counts as activity
    return true;
  } catch (err: any) {
    // TransportStatusError (has statusCode) means USB works but app returned error
    // — device is alive, just maybe wrong app open
    if (err?.statusCode !== undefined) {
      resetIdleTimer();
      return true;
    }
    return false;
  }
}
