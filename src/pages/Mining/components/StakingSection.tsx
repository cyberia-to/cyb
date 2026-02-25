import { useCallback, useState } from 'react';
import { LITIUM_STAKE_CONTRACT, LI_DENOM } from 'src/constants/mining';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import useAutoSigner from '../hooks/useAutoSigner';
import useStakeInfo from '../hooks/useStakeInfo';
import { compactLi } from '../utils/formatLi';
import styles from '../Mining.module.scss';

function StakingSection() {
  const { signer, signingClient, address } = useAutoSigner();
  const { stakeInfo, refetch } = useStakeInfo(address);

  const [stakeAmount, setStakeAmount] = useState('');
  const [unstakeAmount, setUnstakeAmount] = useState('');
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');

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

  const executeMsg = useCallback(
    async (msg: Record<string, unknown>, funds?: { amount: string; denom: string }[]) => {
      if (!signer || !signingClient || !address) return;
      setBusy(true);
      setStatus('');
      try {
        const [account] = await signer.getAccounts();
        const result = await signingClient.execute(
          account.address,
          LITIUM_STAKE_CONTRACT,
          msg,
          Soft3MessageFactory.fee(8),
          '',
          funds
        );
        setStatus(`OK: ${result.transactionHash.slice(0, 12)}...`);
        // Wait for next block then refetch staking data
        setTimeout(() => refetch(), 7000);
      } catch (err: any) {
        setStatus(`Error: ${err?.message?.slice(0, 80) || 'Failed'}`);
      } finally {
        setBusy(false);
      }
    },
    [signer, signingClient, address, refetch]
  );

  const handleStake = useCallback(() => {
    const amountMicro = Math.floor(Number(stakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeMsg({ stake: {} }, [{ amount: String(amountMicro), denom: LI_DENOM }]);
    setStakeAmount('');
  }, [stakeAmount, executeMsg]);

  const handleUnstake = useCallback(() => {
    const amountMicro = Math.floor(Number(unstakeAmount) * 1_000_000);
    if (amountMicro <= 0) return;
    executeMsg({ unstake: { amount: String(amountMicro) } });
    setUnstakeAmount('');
  }, [unstakeAmount, executeMsg]);

  const handleClaimRewards = useCallback(() => {
    executeMsg({ claim_staking_rewards: {} });
  }, [executeMsg]);

  const handleClaimUnbonding = useCallback(() => {
    executeMsg({ claim_unbonding: {} });
  }, [executeMsg]);

  return (
    <div className={styles.sectionBox}>
      <span className={styles.sectionTitle}>Staking</span>

      <div className={styles.statsGrid}>
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

      {status && <div className={styles.stakingStatus}>{status}</div>}
    </div>
  );
}

export default StakingSection;
