/**
 * Parse raw blockchain error logs into human-readable messages.
 * Falls back to trimmed raw text if no pattern matches.
 */
export function friendlyErrorMessage(raw: string | undefined | null): string {
  if (!raw) return 'Transaction failed';

  const text = typeof raw === 'string' ? raw : String(raw);

  // Insufficient funds
  if (text.includes('insufficient funds')) {
    const match = text.match(/(\d+\w+) is smaller than (\d+\w+)/);
    if (match) {
      return `Not enough tokens. You have ${match[1]}, but ${match[2]} is needed`;
    }
    return 'Not enough tokens to complete this transaction';
  }

  // Out of gas
  if (text.includes('out of gas')) {
    return 'Transaction ran out of gas. Try again with a higher gas limit';
  }

  // User declined on device / in wallet UI
  if (text.includes('rejected by the user') || text.includes('Transaction declined')) {
    return 'You rejected the transaction on your device';
  }

  // Chain-level or validator rejection — keep the reason
  if (text.includes('rejected')) {
    // Strip the verbose "failed to execute message; message index: N:" prefix
    const cleaned = text.replace(/failed to execute message;\s*message index:\s*\d+:\s*/gi, '');
    return `Transaction rejected: ${cleaned.length > 150 ? cleaned.slice(0, 150) + '…' : cleaned}`;
  }

  // Account sequence mismatch (concurrent txs)
  if (text.includes('account sequence mismatch')) {
    return 'Previous transaction is still processing. Wait a moment and try again';
  }

  // Too many addresses for passport
  if (text.includes('Too many addresses')) {
    return 'You can prove only 8 addresses for one passport';
  }

  // Signature verification failed
  if (text.includes('signature verification failed')) {
    return 'Signature verification failed. Make sure you are using the correct account';
  }

  // Timeout
  if (text.includes('timed out') || text.includes('timeout')) {
    return 'Transaction timed out. Check your transaction history to see if it went through';
  }

  // Unknown — trim to reasonable length
  if (text.length > 200) {
    return text.slice(0, 200) + '…';
  }

  return text;
}
