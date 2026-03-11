import { useCallback, useState } from 'react';
import { Link } from 'react-router-dom';
import { LITIUM_STAKE_CONTRACT, LITIUM_CORE_CONTRACT, LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import type { TotalStakedResponse, StakingStatsResponse } from 'src/generated/lithium/LitiumStake.types';
import type { TotalMintedResponse } from 'src/generated/lithium/LitiumCore.types';
import type { ConfigResponse } from 'src/generated/lithium/LitiumMine.types';
import useAutoSigner from '../hooks/useAutoSigner';
import useStakeInfo from '../hooks/useStakeInfo';
import { compactLi } from '../utils/formatLi';
import styles from '../Mining.module.scss';

const SECONDS_PER_YEAR = 365.25 * 24 * 3600;
const STAKING_INDEX_SCALE = 1_000_000_000_000;

function StakingSection() {
  const { signer, signingClient, address } = useAutoSigner();
  const { stakeInfo, refetch } = useStakeInfo(address);

  const { data: totalStakedData } = useQueryContract(LITIUM_STAKE_CONTRACT, {
    total_staked: {},
  });
  const { data: totalMintedData } = useQueryContract(LITIUM_CORE_CONTRACT, {
    total_minted: {},
  });
  const { data: stakingStatsData } = useQueryContract(LITIUM_STAKE_CONTRACT, {
    staking_stats: {},
  });
  const { data: mineConfigData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    config: {},
  });

  const totalStaked = totalStakedData as TotalStakedResponse | undefined;
  const totalMinted = totalMintedData as TotalMintedResponse | undefined;
  const stakingStats = stakingStatsData as StakingStatsResponse | undefined;
  const mineConfig = mineConfigData as ConfigResponse | undefined;

  const totalStakedLi = totalStaked ? Number(totalStaked.total_staked) / 1_000_000 : 0;
  const totalMintedLi = totalMinted ? Number(totalMinted.total_minted) / 1_000_000 : 0;

  // Staked % of total minted supply
  const stakedPercent = totalMintedLi > 0 ? (totalStakedLi / totalMintedLi) * 100 : 0;

  // APR from real on-chain data: reward_index tracks cumulative rewards per staked token.
  // APR = (reward_index / elapsed_seconds) * SECONDS_PER_YEAR / STAKING_INDEX_SCALE * 100
  const rewardIndex = stakingStats ? Number(stakingStats.reward_index) : 0;
  const genesisTime = mineConfig?.genesis_time ?? 0;
  const elapsedSec = genesisTime > 0 ? Math.floor(Date.now() / 1000) - genesisTime : 0;
  const stakingApr = rewardIndex > 0 && elapsedSec > 0
    ? (rewardIndex / elapsedSec) * SECONDS_PER_YEAR / STAKING_INDEX_SCALE * 100
    : 0;

  // CW20 balance (what miners actually receive and can stake)
  const { data: cw20BalanceData } = useQueryContract(
    LITIUM_CORE_CONTRACT,
    address ? { balance: { address } } : { token_info: {} }
  );
  const cw20Balance = address && cw20BalanceData && 'balance' in (cw20BalanceData as object)
    ? Number((cw20BalanceData as { balance: string }).balance) / 1_000_000
    : 0;

  const [stakeAmount, setStakeAmount] = useState('');
  const [unstakeAmount, setUnstakeAmount] = useState('');
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<{ ok: boolean; txHash?: string; error?: string } | null>(null);

  const stakedLi = stakeInfo
    ? Number(stakeInfo.staked_amount) / 1_000_000
    : 0;
  const claimableRewards = stakeInfo
    ? Number(stakeInfo.claimable_rewards) / 1_000_000
    : 0;
  const pendingUnbonding = stakeInfo
    ? Number(stakeInfo.pending_unbonding) / 1_000_000
    : 0;
  const unbondingUntil = stakeInfo?.pending_unbonding_until ?? 0;
  const unbondingReady =
    pendingUnbonding > 0 && unbondingUntil <= Math.floor(Date.now() / 1000);

  const executeOnContract = useCallback(
    async (contract: string, msg: Record<string, unknown>) => {
      if (!signer || !signingClient || !address) return;
      setBusy(true);
      setStatus(null);
      try {
        const [account] = await signer.getAccounts();
        const result = await signingClient.execute(
          account.address,
          contract,
          msg,
          Soft3MessageFactory.fee(8)
        );
        setStatus({ ok: true, txHash: result.transactionHash });
        setTimeout(() => refetch(), 7000);
      } catch (err: any) {
        setStatus({ ok: false, error: err?.message?.slice(0, 120) || 'Failed' });
      } finally {
        setBusy(false);
      }
    },
    [signer, signingClient, address, refetch]
  );

  // Stake: send CW20 tokens to stake contract via litium-core's `send`
  const handleStake = useCallback(() => {
    const amountMicro = Math.floor(Number(stakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeOnContract(LITIUM_CORE_CONTRACT, {
      send: {
        contract: LITIUM_STAKE_CONTRACT,
        amount: String(amountMicro),
        msg: btoa(JSON.stringify({ stake: {} })),
      },
    });
    setStakeAmount('');
  }, [stakeAmount, executeOnContract]);

  const handleUnstake = useCallback(() => {
    const amountMicro = Math.floor(Number(unstakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeOnContract(LITIUM_STAKE_CONTRACT, { unstake: { amount: String(amountMicro) } });
    setUnstakeAmount('');
  }, [unstakeAmount, executeOnContract]);

  const handleClaimRewards = useCallback(() => {
    executeOnContract(LITIUM_STAKE_CONTRACT, { claim_staking_rewards: {} });
  }, [executeOnContract]);

  const handleClaimUnbonding = useCallback(() => {
    executeOnContract(LITIUM_STAKE_CONTRACT, { claim_unbonding: {} });
  }, [executeOnContract]);

  return (
    <div className={styles.sectionBox}>
      <span className={styles.sectionTitle}>Staking</span>

      <div className={styles.statsGrid}>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Available</span>
          <span className={styles.statCardValue}>
            {compactLi(cw20Balance)}
            <span className={styles.statCardSuffix}> LI</span>
          </span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Staked</span>
          <span className={styles.statCardValue}>
            {compactLi(stakedLi)}
            <span className={styles.statCardSuffix}> LI</span>
          </span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Claimable</span>
          <span className={styles.statCardValue}>
            {compactLi(claimableRewards)}
            <span className={styles.statCardSuffix}> LI</span>
          </span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Unbonding</span>
          <span className={styles.statCardValue}>
            {compactLi(pendingUnbonding)}
            <span className={styles.statCardSuffix}> LI</span>
          </span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Net. Staked</span>
          <span className={styles.statCardValue}>
            {stakedPercent > 0 ? `${stakedPercent.toFixed(1)}` : '\u2014'}
            <span className={styles.statCardSuffix}> %</span>
          </span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Staking APR</span>
          <span className={styles.statCardValue}>
            {stakingApr > 0
              ? stakingApr > 1e12
                ? `${(stakingApr / 1e12).toFixed(1)}T`
                : stakingApr > 1e9
                  ? `${(stakingApr / 1e9).toFixed(1)}B`
                  : stakingApr > 1_000_000
                    ? `${(stakingApr / 1_000_000).toFixed(1)}M`
                    : stakingApr > 10_000
                      ? `${(stakingApr / 1_000).toFixed(0)}K`
                      : `${stakingApr.toFixed(0)}`
              : '\u2014'}
            <span className={styles.statCardSuffix}> %</span>
          </span>
        </div>
      </div>

      <div className={styles.stakingActions}>
        <div className={styles.stakingRow}>
          <input
            type="text"
            inputMode="decimal"
            placeholder="Amount LI"
            value={stakeAmount}
            onChange={(e) => setStakeAmount(e.target.value)}
            className={styles.stakingInput}
            disabled={busy}
          />
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={handleStake}
            disabled={busy || !stakeAmount}
          >
            Stake
          </button>
        </div>
        <div className={styles.stakingRow}>
          <input
            type="text"
            inputMode="decimal"
            placeholder="Amount LI"
            value={unstakeAmount}
            onChange={(e) => setUnstakeAmount(e.target.value)}
            className={styles.stakingInput}
            disabled={busy}
          />
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={handleUnstake}
            disabled={busy || !unstakeAmount}
          >
            Unstake
          </button>
        </div>
        <div className={styles.stakingRow}>
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={handleClaimRewards}
            disabled={busy || claimableRewards <= 0}
          >
            Claim Rewards
          </button>
          {unbondingReady && (
            <button
              type="button"
              className={styles.stakingBtn}
              onClick={handleClaimUnbonding}
              disabled={busy}
            >
              Claim Unbonding
            </button>
          )}
        </div>
      </div>

      {status && (
        <div className={styles.stakingStatus}>
          {status.ok && status.txHash ? (
            <Link to={routes.txExplorer.getLink(status.txHash)} style={{ color: '#36d6ae' }}>
              TX: {trimString(status.txHash, 10, 6)}
            </Link>
          ) : status.error ? (
            <span style={{ color: '#ef4444' }} title={status.error}>
              Error: {status.error.slice(0, 80)}
            </span>
          ) : null}
        </div>
      )}
    </div>
  );
}

export default StakingSection;
