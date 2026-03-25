// Format versions: v1 = 600k iterations (legacy), v2 = 1M iterations
const PBKDF2_ITERATIONS_V1 = 600_000;
const PBKDF2_ITERATIONS_V2 = 1_000_000;
const CURRENT_VERSION = 2;
const SALT_BYTES = 16;
const IV_BYTES = 12;

async function deriveKey(
  password: string,
  salt: Uint8Array,
  iterations: number
): Promise<CryptoKey> {
  const encoder = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(password),
    'PBKDF2',
    false,
    ['deriveKey']
  );

  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt, iterations, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

export async function encryptMnemonic(mnemonic: string, password: string): Promise<string> {
  const encoder = new TextEncoder();
  const salt = crypto.getRandomValues(new Uint8Array(SALT_BYTES));
  const iv = crypto.getRandomValues(new Uint8Array(IV_BYTES));
  const key = await deriveKey(password, salt, PBKDF2_ITERATIONS_V2);

  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    encoder.encode(mnemonic)
  );

  // Pack: version(1) + salt(16) + iv(12) + ciphertext → base64
  const ct = new Uint8Array(ciphertext);
  const packed = new Uint8Array(1 + SALT_BYTES + IV_BYTES + ct.length);
  packed[0] = CURRENT_VERSION;
  packed.set(salt, 1);
  packed.set(iv, 1 + SALT_BYTES);
  packed.set(ct, 1 + SALT_BYTES + IV_BYTES);

  return btoa(Array.from(packed, (b) => String.fromCharCode(b)).join(''));
}

async function tryDecrypt(
  packed: Uint8Array,
  password: string,
  versioned: boolean
): Promise<string> {
  const offset = versioned ? 1 : 0;
  const minLength = offset + SALT_BYTES + IV_BYTES + 1;
  if (packed.length < minLength) {
    throw new RangeError(`Encrypted blob too short: ${packed.length} < ${minLength}`);
  }

  const iterations = versioned && packed[0] === 2
    ? PBKDF2_ITERATIONS_V2
    : PBKDF2_ITERATIONS_V1;

  const salt = packed.slice(offset, offset + SALT_BYTES);
  const iv = packed.slice(offset + SALT_BYTES, offset + SALT_BYTES + IV_BYTES);
  const ciphertext = packed.slice(offset + SALT_BYTES + IV_BYTES);

  const key = await deriveKey(password, salt, iterations);
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv },
    key,
    ciphertext
  );
  return new TextDecoder().decode(plaintext);
}

export async function decryptMnemonic(encrypted: string, password: string): Promise<string> {
  let packed: Uint8Array;
  try {
    packed = Uint8Array.from(atob(encrypted), (c) => c.charCodeAt(0));
  } catch {
    throw new Error('Invalid encrypted mnemonic data');
  }

  // Try versioned format first (v2 = 1M iterations), fall back to legacy v1 (600k)
  const isVersioned = packed[0] === 1 || packed[0] === 2;

  if (isVersioned) {
    try {
      return await tryDecrypt(packed, password, true);
    } catch (err) {
      // ~0.8% chance legacy salt starts with 0x01/0x02 — retry as unversioned.
      // AES-GCM decryption failure throws DOMException (OperationError).
      if (err instanceof DOMException) {
        return tryDecrypt(packed, password, false);
      }
      throw err;
    }
  }

  return tryDecrypt(packed, password, false);
}
