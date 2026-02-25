/**
 * Format a LI amount (already in human units, not micro) into compact notation.
 * Examples: 2.17T, 1.50B, 3.20M, 12.5K, 42.00, 0.0012
 */
export function compactLi(val: number): string {
  if (val === 0) return '0';
  const abs = Math.abs(val);
  if (abs >= 1e12) return `${(val / 1e12).toFixed(2)}T`;
  if (abs >= 1e9) return `${(val / 1e9).toFixed(2)}B`;
  if (abs >= 1e6) return `${(val / 1e6).toFixed(2)}M`;
  if (abs >= 1e3) return `${(val / 1e3).toFixed(1)}K`;
  if (abs >= 1) return val.toFixed(2);
  if (abs >= 0.01) return val.toFixed(4);
  return val.toFixed(6);
}

/**
 * Format a micro-LI string (from contract response) into compact human-readable form.
 */
export function formatLi(amount: string | undefined): string {
  if (!amount) return '0';
  const val = Number(amount) / 1_000_000;
  return compactLi(val);
}
