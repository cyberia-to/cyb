import { useCallback, useState } from 'react';
import { useSigningClient } from 'src/contexts/signerClient';
import { useAppSelector } from 'src/redux/hooks';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { UHASH_CONTRACT } from 'src/constants/mining';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import useReferralInfo from '../hooks/useReferralInfo';
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
  const address = useAppSelector(selectCurrentAddress);
  const { signer, signingClient } = useSigningClient();
  const { referralInfo } = useReferralInfo(address);

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
    saveReferrer(trimmed);
    onReferrerChange(trimmed);
    setStatus('Referrer saved. Will be included in your next proof submission.');
  }, [inputValue, onReferrerChange]);

  const handleClaimReferralRewards = useCallback(async () => {
    if (!signer || !signingClient || !address) return;
    setBusy(true);
    setStatus('');
    try {
      const [account] = await signer.getAccounts();
      const result = await signingClient.execute(
        account.address,
        UHASH_CONTRACT,
        { claim_referral_rewards: {} },
        Soft3MessageFactory.fee(8),
        ''
      );
      setStatus(`OK: ${result.transactionHash.slice(0, 12)}...`);
    } catch (err: any) {
      setStatus(`Error: ${err?.message?.slice(0, 80) || 'Failed'}`);
    } finally {
      setBusy(false);
    }
  }, [signer, signingClient, address]);

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
            {referralRewards.toFixed(4)}
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

export { loadReferrer };
export default ReferralSection;
