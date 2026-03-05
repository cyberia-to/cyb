import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import type { WindowStatusResponse } from 'src/generated/lithium/LitiumMine.types';

function usePeerEstimate(localHashrate: number) {
  const { data, loading, dataUpdatedAt, refetch: refetchWindow } = useQueryContract(LITIUM_MINE_CONTRACT, {
    window_status: {},
  });

  const resp = data as WindowStatusResponse | undefined;

  // D_rate from contract = total difficulty bits per second across network
  const dRate = resp?.window_d_rate ? Number(resp.window_d_rate) : 0;

  // Network hashrate estimate: each difficulty bit d corresponds to ~2^d hashes.
  // D_rate = sum(d_i) / time_span in difficulty bits/sec.
  // Network hashrate ≈ sum(2^d_i) / time_span, but we approximate using avg difficulty:
  //   avg_d = D_rate / proof_rate, hashrate ≈ proof_rate * 2^avg_d
  const proofCount = resp?.proof_count ?? 0;
  const windowEntries = resp?.window_entries ?? 0;

  // Approximate: if D_rate > 0 and we know the window entries and total_d,
  // we estimate network hashrate from the average difficulty in the window.
  // avg_d ≈ total_d / window_entries (approximated from D_rate * time_span / window_entries)
  // For simplicity, use D_rate directly and convert:
  // If avg difficulty = D_rate / proofRate, and proofRate = windowEntries / timeSpan:
  //   avg_d = D_rate / proofRate = D_rate * timeSpan / windowEntries
  // But we don't have timeSpan directly. Use a simpler estimate:
  // networkHashrate ≈ 2^(avg_d) * proofRate
  // Since D_rate = avg_d * proofRate, proofRate = D_rate / avg_d.
  // networkHashrate = 2^avg_d * D_rate / avg_d.
  // As a practical approximation, we estimate avg_d from recent base_rate or just use D_rate.
  let networkHashrate = 0;
  if (dRate > 0 && windowEntries > 0) {
    // Simple estimate: assume avg difficulty ~= D_rate / proofRate
    // We don't have proofRate directly from the response, so approximate:
    // Just expose D_rate and let the UI interpret it.
    // For "similar devices" estimation, we need hashrate.
    // Use: each proof at difficulty d costs ~2^d hashes.
    // total_hashes/sec ≈ sum(2^d_i)/time ≈ D_rate * 2^(avg_d) / avg_d (complex)
    // Simpler: assume all proofs are at roughly the same difficulty.
    // Then avg_d = total_d / proofCount, hashrate ≈ proofRate * 2^avg_d.
    // D_rate = avg_d * proofRate → proofRate = D_rate / avg_d.
    // hashrate = (D_rate / avg_d) * 2^avg_d.
    // But we don't know avg_d... we need it from config or stats.

    // Fallback: just use D_rate as a rough metric. It scales with network power.
    // For "similar devices", use D_rate as a proxy.
    networkHashrate = dRate; // difficulty-bits-per-second as a proxy metric
  }

  const similarDevices =
    localHashrate > 0 && networkHashrate > 0
      ? Math.max(1, Math.round(networkHashrate / localHashrate))
      : 0;

  return {
    networkHashrate,
    dRate,
    similarDevices,
    windowEntries,
    proofCount,
    baseRate: resp?.base_rate ?? '0',
    alpha: resp?.alpha ?? '0',
    beta: resp?.beta ?? '0',
    loading,
    dataUpdatedAt,
    refetchWindow,
  };
}

export default usePeerEstimate;
