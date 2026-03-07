const SLOT_SIZE = 512;
const KEY_SIZE = 32;
const IV_SIZE = 12;
const LEN_SIZE = 2;
const HEADER_SIZE = KEY_SIZE + IV_SIZE + LEN_SIZE; // 46
const PAYLOAD_SIZE = SLOT_SIZE;

/**
 * Encrypts {mnemonic, referrer} with AES-256-GCM and returns
 * a 512-byte payload: key[32] || iv[12] || ct_len[2 big-endian] || ciphertext || zeroes
 */
export async function encryptBootstrap(payload: {
  mnemonic: string;
  referrer: string;
  name?: string;
}): Promise<Uint8Array> {
  const plaintext = new TextEncoder().encode(JSON.stringify(payload));

  const key = crypto.getRandomValues(new Uint8Array(KEY_SIZE));
  const iv = crypto.getRandomValues(new Uint8Array(IV_SIZE));

  const cryptoKey = await crypto.subtle.importKey(
    'raw',
    key,
    { name: 'AES-GCM' },
    false,
    ['encrypt']
  );

  const ciphertext = new Uint8Array(
    await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, cryptoKey, plaintext)
  );

  const maxCipherSize = PAYLOAD_SIZE - HEADER_SIZE;
  if (ciphertext.length > maxCipherSize) {
    throw new Error(
      `Ciphertext too large: ${ciphertext.length} > ${maxCipherSize}`
    );
  }

  // Assemble: key[32] || iv[12] || ct_len[2] || ciphertext || zeroes
  const result = new Uint8Array(PAYLOAD_SIZE);
  result.set(key, 0);
  result.set(iv, KEY_SIZE);
  // 2-byte big-endian ciphertext length
  result[KEY_SIZE + IV_SIZE] = (ciphertext.length >> 8) & 0xff;
  result[KEY_SIZE + IV_SIZE + 1] = ciphertext.length & 0xff;
  result.set(ciphertext, HEADER_SIZE);

  return result;
}
