import { useCallback, useState } from 'react';
import { BOOT_SERVER_URL } from 'src/constants/mining';
import { getMnemonic } from 'src/utils/utils';
import { encryptBootstrap } from '../utils/bootstrapCrypto';
import { loadReferrer } from './ReferralSection';
import styles from '../Mining.module.scss';

type Platform = {
  key: string;
  label: string;
};

function detectPlatform(): Platform {
  const ua = navigator.userAgent.toLowerCase();
  const platform = (navigator as any).userAgentData?.platform?.toLowerCase() || navigator.platform?.toLowerCase() || '';

  if (platform.includes('mac') || ua.includes('macintosh')) {
    // Check for Apple Silicon vs Intel
    // navigator.userAgentData.architecture is 'arm' on Apple Silicon
    const arch = (navigator as any).userAgentData?.architecture?.toLowerCase() || '';
    if (arch === 'arm' || ua.includes('arm64')) {
      return { key: 'aarch64-apple-darwin', label: 'macOS (Apple Silicon)' };
    }
    return { key: 'x86_64-apple-darwin', label: 'macOS (Intel)' };
  }
  if (platform.includes('win') || ua.includes('windows')) {
    return { key: 'x86_64-pc-windows-msvc', label: 'Windows' };
  }
  return { key: 'x86_64-unknown-linux-musl', label: 'Linux' };
}

type Props = {
  address?: string;
};

function DownloadSection({ address }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const detected = detectPlatform();

  const handleDownload = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      // Get mnemonic
      const mnemonic = getMnemonic(address);
      if (!mnemonic) {
        setError('No wallet found. Mine at least once first.');
        return;
      }

      const referrer = loadReferrer();

      // Encrypt payload
      const payload = await encryptBootstrap({ mnemonic, referrer });
      const data = btoa(String.fromCharCode(...payload));

      // Request patched binary from distribution server
      const response = await fetch(`${BOOT_SERVER_URL}/boot`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          platform: detected.key,
          data,
        }),
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `Server error: ${response.status}`);
      }

      // Trigger zip download (contains signed binary + boot.dat)
      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'cyb-boot.zip';
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    } catch (err: any) {
      console.error('[DownloadSection] Error:', err);
      setError(err?.message || 'Download failed');
    } finally {
      setLoading(false);
    }
  }, [address, detected.key]);

  return (
    <div className={styles.sectionBox}>
      <span className={styles.sectionTitle}>Desktop App</span>
      <div className={styles.downloadInfo}>
        Switch to desktop for GPU mining — up to 100x faster hashrate.
        Your wallet and referrer transfer automatically.
      </div>
      <div className={styles.downloadRow}>
        <button
          type="button"
          className={styles.downloadBtn}
          onClick={handleDownload}
          disabled={loading}
        >
          {loading ? 'Preparing...' : `Download for ${detected.label}`}
        </button>
      </div>
      {error && (
        <div className={styles.downloadError}>{error}</div>
      )}
    </div>
  );
}

export default DownloadSection;
