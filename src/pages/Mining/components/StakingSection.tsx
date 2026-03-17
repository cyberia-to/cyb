import { useCallback, useState } from 'react';
import { Link } from 'react-router-dom';
import { LITIUM_STAKE_CONTRACT, LITIUM_CORE_CONTRACT, LITIUM_MINE_CONTRACT } from 'src/constants/mining';
import { routes } from 'src/routes';
import { trimString } from 'src/utils/utils';
import useQueryContract from 'src/hooks/contract/useQueryContract';
import type { TotalStakedResponse } from 'src/generated/lithium/LitiumStake.types';
import type { TotalMintedResponse, BurnStatsResponse } from 'src/generated/lithium/LitiumCore.types';
import type { WindowStatusResponse } from 'src/generated/lithium/LitiumMine.types';
import useAutoSigner from '../hooks/useAutoSigner';
import useStakeInfo from '../hooks/useStakeInfo';
import { compactLi } from '../utils/formatLi';
import styles from '../Mining.module.scss';

const SECONDS_PER_YEAR = 365.25 * 24 * 3600;

type Props = {
  logAction: (action: string, detail?: string, result?: 'ok' | 'error', error?: string) => void;
  sendContractTx: (contract: string, msg: Record<string, unknown>) => Promise<{
    transactionHash: string;
    code: number;
    events: { type: string; attributes?: { key: string; value: string }[] }[];
    rawLog: string;
  }>;
};

function StakingSection({ logAction, sendContractTx }: Props) {
  const { address } = useAutoSigner();
  const { stakeInfo, refetch } = useStakeInfo(address);

  const { data: totalStakedData, refetch: refetchTotalStaked } = useQueryContract(LITIUM_STAKE_CONTRACT, {
    total_staked: {},
  });
  const { data: totalMintedData, refetch: refetchTotalMinted } = useQueryContract(LITIUM_CORE_CONTRACT, {
    total_minted: {},
  });
  const { data: stakingStatsData, refetch: refetchStakingStats } = useQueryContract(LITIUM_STAKE_CONTRACT, {
    staking_stats: {},
  });
  const { data: windowData } = useQueryContract(LITIUM_MINE_CONTRACT, {
    window_status: {},
  });
  const { data: burnStatsData, refetch: refetchBurnStats } = useQueryContract(LITIUM_CORE_CONTRACT, {
    burn_stats: {},
  });
  const { data: pendingRewardsData, refetch: refetchPendingRewards } = useQueryContract(LITIUM_STAKE_CONTRACT, {
    total_pending_rewards: {},
  });

  const totalStaked = totalStakedData as TotalStakedResponse | undefined;
  const totalMinted = totalMintedData as TotalMintedResponse | undefined;
  const windowStatus = windowData as WindowStatusResponse | undefined;
  const burnStats = burnStatsData as BurnStatsResponse | undefined;
  const pendingRewards = pendingRewardsData as { total_pending_rewards: string } | undefined;

  const totalStakedLi = totalStaked ? Number(totalStaked.total_staked) / 1_000_000 : 0;
  // Effective supply = minted - burned + pending staking rewards (unminted but accrued)
  const circulatingLi = totalMinted
    ? (Number(totalMinted.total_minted) - Number(burnStats?.total_burned ?? 0)
      + Number(pendingRewards?.total_pending_rewards ?? 0)) / 1_000_000
    : 0;

  // Staked % of circulating supply
  const stakedPercent = circulatingLi > 0 ? (totalStakedLi / circulatingLi) * 100 : 0;

  // Forward-looking APR (spec §5):
  // Staking gets S^alpha of 90% post-referral gross emission.
  // APR = base_rate * D_rate * 0.9 * S^alpha * SECONDS_PER_YEAR / total_staked * 100
  const baseRate = windowStatus ? Number(windowStatus.base_rate) : 0;
  const dRate = windowStatus ? Number(windowStatus.window_d_rate) : 0;
  const alpha = windowStatus ? Number(windowStatus.alpha) : 0;
  const stakedFraction = circulatingLi > 0 ? totalStakedLi / circulatingLi : 0;
  const stakingShare = stakedFraction > 0 && alpha > 0 ? Math.pow(stakedFraction, alpha) : 0;
  const postReferralShare = 0.9; // 10% of gross goes to referral program

  const stakingApr = totalStakedLi > 0 && baseRate > 0 && dRate > 0
    ? (baseRate * dRate * postReferralShare * stakingShare * SECONDS_PER_YEAR) / (totalStakedLi * 1_000_000) * 100
    : 0;

  // CW20 balance (what miners actually receive and can stake)
  const { data: cw20BalanceData, refetch: refetchBalance } = useQueryContract(
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
  const nowSec = Math.floor(Date.now() / 1000);
  const unbondingReady = pendingUnbonding > 0 && unbondingUntil <= nowSec;
  const unbondingRemainingSec = pendingUnbonding > 0 && unbondingUntil > nowSec
    ? unbondingUntil - nowSec
    : 0;

  const executeOnContract = useCallback(
    async (actionName: string, contract: string, msg: Record<string, unknown>, detail?: string) => {
      if (!address) return;
      setBusy(true);
      setStatus(null);
      logAction(actionName, detail);
      try {
        const result = await sendContractTx(contract, msg);
        const txHash = result.transactionHash;
        setStatus({ ok: true, txHash });
        logAction(actionName, `txHash=${txHash}`, 'ok');
        setTimeout(() => {
          refetch();
          refetchBalance();
          refetchTotalStaked();
          refetchTotalMinted();
          refetchStakingStats();
          refetchBurnStats();
          refetchPendingRewards();
        }, 2000);
      } catch (err: any) {
        const errMsg = err?.message?.slice(0, 120) || 'Failed';
        setStatus({ ok: false, error: errMsg });
        logAction(actionName, undefined, 'error', errMsg);
      } finally {
        setBusy(false);
      }
    },
    [address, refetch, refetchBalance, refetchTotalStaked, refetchTotalMinted, refetchStakingStats, refetchBurnStats, refetchPendingRewards, logAction, sendContractTx]
  );

  // Stake: send CW20 tokens to stake contract via litium-core's `send`
  const handleStake = useCallback(() => {
    const amountMicro = Math.floor(Number(stakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeOnContract('stake_li', LITIUM_CORE_CONTRACT, {
      send: {
        contract: LITIUM_STAKE_CONTRACT,
        amount: String(amountMicro),
        msg: btoa(JSON.stringify({ stake: {} })),
      },
    }, `amount=${stakeAmount} LI`);
    setStakeAmount('');
  }, [stakeAmount, executeOnContract]);

  const handleUnstake = useCallback(() => {
    const amountMicro = Math.floor(Number(unstakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeOnContract('unstake_li', LITIUM_STAKE_CONTRACT, { unstake: { amount: String(amountMicro) } }, `amount=${unstakeAmount} LI`);
    setUnstakeAmount('');
  }, [unstakeAmount, executeOnContract]);

  const handleClaimRewards = useCallback(() => {
    executeOnContract('claim_staking_rewards', LITIUM_STAKE_CONTRACT, { claim_staking_rewards: {} });
  }, [executeOnContract]);

  const handleClaimUnbonding = useCallback(() => {
    executeOnContract('claim_unbonding', LITIUM_STAKE_CONTRACT, { claim_unbonding: {} });
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
          {unbondingReady && pendingUnbonding > 0 && (
            <span className={styles.statCardSuffix} style={{ color: '#36d6ae' }}>Ready</span>
          )}
          {unbondingRemainingSec > 0 && (
            <span className={styles.statCardSuffix}>
              {unbondingRemainingSec >= 86400
                ? `${Math.floor(unbondingRemainingSec / 86400)}d ${Math.floor((unbondingRemainingSec % 86400) / 3600)}h`
                : unbondingRemainingSec >= 3600
                  ? `${Math.floor(unbondingRemainingSec / 3600)}h ${Math.floor((unbondingRemainingSec % 3600) / 60)}m`
                  : `${Math.floor(unbondingRemainingSec / 60)}m`}
            </span>
          )}
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
            <>
              {busy && <span style={{ color: '#888' }}>Confirming... </span>}
              <Link to={routes.txExplorer.getLink(status.txHash)} style={{ color: '#36d6ae' }}>
                TX: {trimString(status.txHash, 10, 6)}
              </Link>
            </>
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
