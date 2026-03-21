import TransportWebUSB from '@ledgerhq/hw-transport-webusb';
import { LedgerSigner } from '@cosmjs/ledger-amino';
import { makeCosmoshubPath } from '@cosmjs/amino';

const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes — signing on device can take time

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
 * Connect to a Ledger device, create a signer, and return account info.
 * @param prefix - bech32 prefix (default: 'bostrom')
 */
export async function connectLedger(prefix = 'bostrom'): Promise<{
  signer: LedgerSigner;
  address: string;
  pubkey: Uint8Array;
}> {
  const signer = await createLedgerSigner(prefix);
  const [account] = await signer.getAccounts();
  return {
    signer,
    address: account.address,
    pubkey: account.pubkey,
  };
}

/**
 * Type guard: check if a signer is a LedgerSigner instance.
 */
export function isLedgerSigner(signer: unknown): signer is LedgerSigner {
  return signer instanceof LedgerSigner;
}

/**
 * Check if WebUSB is available in the current browser.
 */
export function isWebUSBSupported(): boolean {
  return typeof navigator !== 'undefined' && !!navigator.usb;
}
