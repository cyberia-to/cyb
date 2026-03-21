import TransportWebUSB from '@ledgerhq/hw-transport-webusb';
import { LedgerSigner } from '@cosmjs/ledger-amino';
import { makeCosmoshubPath } from '@cosmjs/amino';
import type { AccountData, AminoSignResponse, OfflineAminoSigner, StdSignDoc } from '@cosmjs/amino';

const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes — signing on device can take time
const HEALTH_CHECK_TIMEOUT_MS = 3_000; // 3 seconds — ping timeout

let _transport: TransportWebUSB | null = null;
let _idleTimer: ReturnType<typeof setTimeout> | null = null;
let _transportPromise: Promise<TransportWebUSB> | null = null;

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
    throw new Error('WebUSB is not supported in this browser. Use Chrome or Edge.');
  }

  if (_transport) {
    try {
      // Ping the transport to check if it's still alive
      await _transport.send(0xe0, 0x01, 0x00, 0x00);
      resetIdleTimer();
      return _transport;
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
 * Create a LedgerSigner for Cosmos-based chains.
 * @param prefix - bech32 prefix (default: 'bostrom')
 */
export async function createLedgerSigner(prefix = 'bostrom'): Promise<LedgerSigner> {
  const transport = await getTransport();
  const hdPaths = [makeCosmoshubPath(0)];
  return new LedgerSigner(transport, { hdPaths, prefix });
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
    const inner = await createLedgerSigner(this.prefix);
    this._cachedAccounts = await inner.getAccounts();
    return this._cachedAccounts;
  }

  async signAmino(signerAddress: string, signDoc: StdSignDoc): Promise<AminoSignResponse> {
    // Fresh signer with fresh transport — survives device sleep
    const inner = await createLedgerSigner(this.prefix);
    return inner.signAmino(signerAddress, signDoc);
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
  // Validate device connection with a real signer first
  const inner = await createLedgerSigner(prefix);
  const [account] = await inner.getAccounts();

  // Return a reconnecting signer for long-lived use
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
  return signer instanceof LedgerSigner || signer instanceof ReconnectingLedgerSigner;
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
