/**
 * Parse raw blockchain error logs and JS errors into human-readable messages.
 * Pattern: "what happened. what to do" — no jargon, no raw logs.
 */
export function friendlyErrorMessage(raw: string | undefined | null): string {
  if (!raw) return 'Transaction failed';

  const text = typeof raw === 'string' ? raw : String(raw);
  const lower = text.toLowerCase();

  // --- User action ---

  if (lower.includes('rejected by the user') || lower.includes('transaction declined')) {
    return 'You rejected the transaction on your device';
  }

  if (lower.includes('request rejected')) {
    return 'Transaction was cancelled';
  }

  // --- Funds & fees ---

  if (lower.includes('insufficient funds') || lower.includes('insufficient balance')) {
    const match = text.match(/(\d+\w+) is smaller than (\d+\w+)/);
    if (match) {
      return `Not enough tokens. You have ${match[1]}, but ${match[2]} is needed`;
    }
    return 'Not enough tokens to complete this transaction';
  }

  if (lower.includes('insufficient fee') || (lower.includes('codespace sdk') && lower.includes('code 13'))) {
    return 'Transaction fee is too low. Try again with a higher fee';
  }

  if (lower.includes('out of gas')) {
    return 'Transaction ran out of gas. Try again with a higher gas limit';
  }

  // --- Sequencing ---

  if (lower.includes('account sequence mismatch')) {
    return 'Previous transaction is still processing. Wait a moment and try again';
  }

  if (lower.includes('tx already in mempool') || lower.includes('already exists')) {
    return 'This transaction was already submitted. Wait for it to confirm';
  }

  // --- Address & format ---

  if (lower.includes('decoding bech32 failed') || lower.includes('invalid address')) {
    return 'Invalid recipient address. Check the address and try again';
  }

  if (lower.includes('invalid coins') || lower.includes('invalid amount')) {
    return 'Invalid amount. Check the value and try again';
  }

  // --- Auth & permissions ---

  if (lower.includes('unauthorized') || lower.includes('not allowed')) {
    return 'This account does not have permission for this action';
  }

  if (lower.includes('signature verification failed')) {
    return 'Signature check failed. Make sure you are using the correct account';
  }

  // --- Network & connectivity ---

  if (
    lower.includes('failed to fetch') ||
    lower.includes('networkerror') ||
    lower.includes('network error') ||
    lower.includes('load failed')
  ) {
    return 'Network error. Check your internet connection and try again';
  }

  if (lower.includes('econnrefused') || lower.includes('connection refused')) {
    return 'Could not reach the node. The RPC server may be down — try again later';
  }

  if (
    lower.includes('502') ||
    lower.includes('503') ||
    lower.includes('bad gateway') ||
    lower.includes('service unavailable')
  ) {
    return 'Server is temporarily unavailable. Try again in a few minutes';
  }

  if (lower.includes('timed out') || lower.includes('timeout') || lower.includes('etimedout')) {
    return 'Connection timed out. Check your transaction history — it may have gone through';
  }

  // --- Smart contracts ---

  if (lower.includes('execute wasm contract failed')) {
    const reason = extractReason(text);
    return reason
      ? `Smart contract error: ${reason}`
      : 'Smart contract execution failed. Check your input and try again';
  }

  if (lower.includes('contract') && lower.includes('not found')) {
    return 'Smart contract not found. It may have been removed or the address is wrong';
  }

  // --- Passport / gift ---

  if (lower.includes('too many addresses')) {
    return 'You can prove only 8 addresses for one passport';
  }

  // --- Generic chain rejection with reason ---

  if (lower.includes('failed to execute message')) {
    const reason = extractReason(text);
    return reason
      ? `Transaction failed: ${reason}`
      : 'Transaction was rejected by the chain';
  }

  // --- Fallback: trim to readable length ---

  if (text.length > 200) {
    return text.slice(0, 200) + '…';
  }

  return text;
}

/**
 * Extract the meaningful part from verbose chain error messages.
 * Strips "failed to execute message; message index: N:" and "execute wasm contract failed:" prefixes.
 */
function extractReason(text: string): string | null {
  const cleaned = text
    .replace(/failed to execute message;\s*message index:\s*\d+:\s*/gi, '')
    .replace(/execute wasm contract failed:\s*/gi, '')
    .trim();

  if (!cleaned || cleaned === text.trim()) return null;
  return cleaned.length > 150 ? cleaned.slice(0, 150) + '…' : cleaned;
}
