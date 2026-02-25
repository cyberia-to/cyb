import { useCallback, useState } from 'react';
import { LITIUM_REFER_CONTRACT } from 'src/constants/mining';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import useAutoSigner from '../hooks/useAutoSigner';
import useReferralInfo from '../hooks/useReferralInfo';
import { compactLi } from '../utils/formatLi';
import styles from '../Mining.module.scss';

const REFERRER_KEY = 'mining_referrer';

function loadReferrer(): string {
  try {
    return localStorage.getItem(REFERRER_KEY) || '';
  } catch {
    return '';
  }
}

function saveReferrer(value: string) {
  try {
    localStorage.setItem(REFERRER_KEY, value);
  } catch {
    // ignore
  }
}

type Props = {
  referrer: string;
  onReferrerChange: (value: string) => void;
};

function ReferralSection({ referrer, onReferrerChange }: Props) {
  const { signer, signingClient, address } = useAutoSigner();
  const { referralInfo, refetch } = useReferralInfo(address);

  const [inputValue, setInputValue] = useState(() => loadReferrer());
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('');

  const boundReferrer = referralInfo?.referrer ?? null;
  const referralRewards = referralInfo
    ? Number(referralInfo.referral_rewards) / 1_000_000
    : 0;
  const referralsCount = referralInfo?.referrals_count ?? 0;

  const handleSetReferrer = useCallback(() => {
    const trimmed = inputValue.trim();
    if (!trimmed) return;
    if (address && trimmed === address) {
      setStatus('Cannot refer yourself.');
      return;
    }
    saveReferrer(trimmed);
    onReferrerChange(trimmed);
    setStatus('Referrer saved. Will be included in your next proof submission.');
  }, [inputValue, onReferrerChange, address]);

  const handleClaimReferralRewards = useCallback(async () => {
    if (!signer || !signingClient || !address) return;
    setBusy(true);
    setStatus('');
    try {
      const [account] = await signer.getAccounts();
      const result = await signingClient.execute(
        account.address,
        LITIUM_REFER_CONTRACT,
        { claim_rewards: {} },
        Soft3MessageFactory.fee(8),
        ''
      );
      setStatus(`OK: ${result.transactionHash.slice(0, 12)}...`);
      // Wait for next block then refetch referral data
      setTimeout(() => refetch(), 7000);
    } catch (err: any) {
      setStatus(`Error: ${err?.message?.slice(0, 80) || 'Failed'}`);
    } finally {
      setBusy(false);
    }
  }, [signer, signingClient, address, refetch]);

  const handleCopyLink = useCallback(() => {
    if (address) {
      navigator.clipboard.writeText(`${window.location.origin}/mining?ref=${address}`);
      setStatus('Referral link copied!');
    }
  }, [address]);

  return (
    <div className={styles.sectionBox}>
      <span className={styles.sectionTitle}>Referral</span>

      {/* Referrer display / set */}
      <div className={styles.referralRow}>
        <span className={styles.referralLabel}>Your Referrer:</span>
        {boundReferrer ? (
          <span className={styles.referralValue}>
            {boundReferrer.slice(0, 16)}...{boundReferrer.slice(-6)}
          </span>
        ) : (
          <span className={styles.referralValue}>None (set before first proof)</span>
        )}
      </div>

      {!boundReferrer && (
        <div className={styles.stakingRow}>
          <input
            type="text"
            placeholder="bostrom1..."
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            className={styles.stakingInput}
          />
          <button
            type="button"
            className={styles.stakingBtn}
            onClick={handleSetReferrer}
            disabled={!inputValue.trim()}
          >
            Set
          </button>
        </div>
      )}

      {/* Stats as referrer */}
      <div className={styles.statsGrid}>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Referrals</span>
          <span className={styles.statCardValue}>{referralsCount}</span>
        </div>
        <div className={styles.statCard}>
          <span className={styles.statCardLabel}>Rewards</span>
          <span className={styles.statCardValue}>
            {compactLi(referralRewards)}
            <span className={styles.statCardSuffix}> LI</span>
          </span>
        </div>
      </div>

      <div className={styles.stakingRow}>
        <button
          type="button"
          className={styles.stakingBtn}
          onClick={handleClaimReferralRewards}
          disabled={busy || referralRewards <= 0}
        >
          Claim Referral Rewards
        </button>
        <button
          type="button"
          className={styles.stakingBtn}
          onClick={handleCopyLink}
          disabled={!address}
        >
          Copy Referral Link
        </button>
      </div>

      {status && <div className={styles.stakingStatus}>{status}</div>}
    </div>
  );
}

export { loadReferrer, saveReferrer };
export default ReferralSection;
