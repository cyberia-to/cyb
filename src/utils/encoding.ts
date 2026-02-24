const hexChars = '0123456789abcdef';

export function toHex(bytes: Uint8Array): string {
  let hex = '';
  for (let i = 0; i < bytes.length; i++) {
    hex += hexChars[bytes[i] >> 4] + hexChars[bytes[i] & 0x0f];
  }
  return hex;
}

export function toBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export function toBytes(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

export function fromBytes(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes);
}
