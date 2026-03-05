import { CyberClient } from '@cybercongress/cyber-js';
import {
  LITIUM_MINE_CONTRACT,
  LITIUM_CORE_CONTRACT,
  LITIUM_STAKE_CONTRACT,
  LITIUM_REFER_CONTRACT,
} from 'src/constants/mining';
import { RPC_URL } from 'src/constants/config';
import type {
  WindowStatusResponse,
  EmissionInfoResponse,
  ConfigResponse,
  StatsResponse,
} from 'src/generated/lithium/LitiumMine.types';
import type {
  BurnStatsResponse,
  TotalMintedResponse,
} from 'src/generated/lithium/LitiumCore.types';
import type { TotalStakedResponse } from 'src/generated/lithium/LitiumStake.types';
import {
  assert,
  assertDefined,
  assertType,
  assertPositive,
  runTests,
  type TestResult,
} from './testRunner';

// ---------------------------------------------------------------------------
// Read-only contract tests (no signer required)
// ---------------------------------------------------------------------------

function buildReadTests(client: CyberClient) {
  return [
    // --- MINE contract ---
    {
      name: 'mine: window_status returns valid structure',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_MINE_CONTRACT,
          { window_status: {} }
        )) as WindowStatusResponse;
        assertDefined(data, 'window_status');
        assertType(data.proof_count, 'number', 'proof_count');
        assertType(data.window_size, 'number', 'window_size');
        assertType(data.window_entries, 'number', 'window_entries');
        assertDefined(data.base_rate, 'base_rate');
        assertDefined(data.alpha, 'alpha');
        assertDefined(data.beta, 'beta');
      },
    },
    {
      name: 'mine: config returns valid structure',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_MINE_CONTRACT,
          { config: {} }
        )) as ConfigResponse;
        assertDefined(data, 'config');
        assertType(data.min_difficulty, 'number', 'min_difficulty');
        assertType(data.alpha, 'number', 'alpha');
        assertType(data.beta, 'number', 'beta');
        assert(
          data.alpha >= 0 && data.alpha <= 1_000_000,
          `alpha ${data.alpha} out of [0, 1_000_000]`
        );
        assert(
          data.min_difficulty >= 1 && data.min_difficulty <= 64,
          `min_difficulty ${data.min_difficulty} out of range`
        );
      },
    },
    {
      name: 'mine: stats returns valid structure',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_MINE_CONTRACT,
          { stats: {} }
        )) as StatsResponse;
        assertDefined(data, 'stats');
        assertType(data.total_proofs, 'number', 'total_proofs');
        assertDefined(data.total_rewards, 'total_rewards');
        assertType(data.unique_miners, 'number', 'unique_miners');
      },
    },
    {
      name: 'mine: calculate_reward returns valid response',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_MINE_CONTRACT,
          { calculate_reward: { difficulty_bits: 15 } }
        )) as { gross_reward: string; earns_reward: boolean };
        assertDefined(data, 'calculate_reward');
        assertDefined(data.gross_reward, 'gross_reward');
        const reward = Number(data.gross_reward);
        assert(!isNaN(reward), 'gross_reward should be numeric');
      },
    },
    {
      name: 'mine: emission_info returns valid split',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_MINE_CONTRACT,
          { emission_info: {} }
        )) as EmissionInfoResponse;
        assertDefined(data, 'emission_info');
        assertDefined(data.emission_rate, 'emission_rate');
        assertDefined(data.gross_rate, 'gross_rate');
        assertDefined(data.mining_rate, 'mining_rate');
        assertDefined(data.staking_rate, 'staking_rate');
        assertType(data.alpha, 'number', 'alpha');
        assertType(data.beta, 'number', 'beta');
      },
    },

    // --- CORE contract ---
    {
      name: 'core: total_minted returns valid values',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_CORE_CONTRACT,
          { total_minted: {} }
        )) as TotalMintedResponse;
        assertDefined(data, 'total_minted');
        const minted = Number(data.total_minted);
        const cap = Number(data.supply_cap);
        assert(!isNaN(minted) && !isNaN(cap), 'minted/cap should be numeric');
        assert(minted <= cap, `minted (${minted}) exceeds cap (${cap})`);
      },
    },
    {
      name: 'core: burn_stats returns total_burned',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_CORE_CONTRACT,
          { burn_stats: {} }
        )) as BurnStatsResponse;
        assertDefined(data, 'burn_stats');
        const burned = Number(data.total_burned);
        assert(!isNaN(burned), 'total_burned should be numeric');
        assert(burned >= 0, 'total_burned should be non-negative');
      },
    },

    // --- STAKE contract ---
    {
      name: 'stake: total_staked returns valid value',
      fn: async () => {
        const data = (await client.queryContractSmart(
          LITIUM_STAKE_CONTRACT,
          { total_staked: {} }
        )) as TotalStakedResponse;
        assertDefined(data, 'total_staked');
        const staked = Number(data.total_staked);
        assert(!isNaN(staked), 'total_staked should be numeric');
        assert(staked >= 0, 'total_staked should be non-negative');
      },
    },
    {
      name: 'stake: total_staked <= total_minted',
      fn: async () => {
        const [stakeData, mintData] = await Promise.all([
          client.queryContractSmart(LITIUM_STAKE_CONTRACT, { total_staked: {} }) as Promise<TotalStakedResponse>,
          client.queryContractSmart(LITIUM_CORE_CONTRACT, { total_minted: {} }) as Promise<TotalMintedResponse>,
        ]);
        const staked = Number(stakeData.total_staked);
        const minted = Number(mintData.total_minted);
        assert(staked <= minted, `staked (${staked}) > minted (${minted})`);
      },
    },

    // --- REFER contract ---
    {
      name: 'refer: config returns valid structure',
      fn: async () => {
        const data = await client.queryContractSmart(
          LITIUM_REFER_CONTRACT,
          { config: {} }
        );
        assertDefined(data, 'refer config');
      },
    },
  ];
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function runContractTests(): Promise<TestResult[]> {
  const client = await CyberClient.connect(RPC_URL);
  const tests = buildReadTests(client);
  const results = await runTests(tests);

  // Log structured results
  console.log('[ContractTests] Results:', JSON.stringify(results, null, 2));
  const passed = results.filter((r) => r.passed).length;
  console.log(
    `[ContractTests] ${passed}/${results.length} passed`
  );

  return results;
}
