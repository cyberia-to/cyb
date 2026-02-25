import useQueryContract from 'src/hooks/contract/useQueryContract';
import { LITIUM_MINE_CONTRACT } from 'src/constants/mining';

const WINDOW_DURATION_S = 600; // 10-minute rolling window

function usePeerEstimate(localHashrate: number) {
  const { data, loading } = useQueryContract(LITIUM_MINE_CONTRACT, {
    difficulty: {},
  });

  const resp = data as
    | {
        current: number;
        min_profitable: number;
        window_proof_count: number;
        window_total_work: string; // Uint128 comes as string
      }
    | undefined;

  const windowWork = resp ? Number(resp.window_total_work) : 0;
  const networkHashrate = windowWork / WINDOW_DURATION_S;
  const similarDevices =
    localHashrate > 0
      ? Math.max(1, Math.round(networkHashrate / localHashrate))
      : 0;

  return {
    networkHashrate,
    similarDevices,
    windowProofCount: resp?.window_proof_count ?? 0,
    loading,
  };
}

export default usePeerEstimate;
