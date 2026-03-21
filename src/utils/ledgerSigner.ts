import TransportWebUSB from '@ledgerhq/hw-transport-webusb';
import CosmosApp from '@zondax/ledger-cosmos-js';
import { encodeSecp256k1Signature, serializeSignDoc } from '@cosmjs/amino';
import { Secp256k1Signature } from '@cosmjs/crypto';
import type { AccountData, AminoSignResponse, OfflineAminoSigner, StdSignDoc } from '@cosmjs/amino';

const HD_PATH = "m/44'/118'/0'/0/0";
const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes — signing on device can take time
const HEALTH_CHECK_TIMEOUT_MS = 3_000; // 3 seconds — ping timeout

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
      // Ping: getVersion APDU — Cosmos app responds with 0x9000
      const response = await _transport.send(0xe0, 0x01, 0x00, 0x00);
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
 * Uses @zondax/ledger-cosmos-js which sends HRP in the INIT chunk —
 * required by Cosmos Ledger app v2.35+.
 */
async function signWithLedger(
  signDoc: StdSignDoc,
  prefix: string
): Promise<AminoSignResponse> {
  const transport = await getTransport();
  const app = new CosmosApp(transport);

  // Get account for the signature envelope
  const response = await app.getAddressAndPubKey(HD_PATH, prefix);
  const pubkey = Uint8Array.from(response.compressed_pk);

  // Serialize sign doc to canonical JSON bytes
  const message = Buffer.from(serializeSignDoc(signDoc));

  // Sign with HRP — the key fix for Cosmos Ledger app v2.35+
  const signResponse = await app.sign(HD_PATH, message, prefix);

  // Convert DER signature to fixed-length 64-byte format
  const signature = Secp256k1Signature.fromDer(
    Uint8Array.from(signResponse.signature)
  ).toFixedLength();

  return {
    signed: signDoc,
    signature: encodeSecp256k1Signature(pubkey, signature),
  };
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
      return await signWithLedger(signDoc, this.prefix);
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
      _transport.send(0xe0, 0x01, 0x00, 0x00),
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
