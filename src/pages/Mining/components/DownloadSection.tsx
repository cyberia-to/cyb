import { useCallback, useState } from 'react';
import { BOOT_SERVER_URL } from 'src/constants/mining';
import { getMnemonic } from 'src/utils/utils';
import { encryptBootstrap } from '../utils/bootstrapCrypto';
import { loadReferrer } from './ReferralSection';
import styles from '../Mining.module.scss';

type Platform = {
  key: string;
  label: string;
  icon: string;
};

// Detect Apple Silicon via WebGL — works in all browsers including Safari.
// Layer 1: GPU renderer string ("Apple M1/M2/M3" in Chrome/Firefox)
// Layer 2: ETC texture compression (supported on Apple Silicon, not Intel)
function isAppleSilicon(): boolean {
  try {
    const canvas = document.createElement('canvas');
    const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
    if (!gl) return false;

    const dbg = gl.getExtension('WEBGL_debug_renderer_info');
    if (dbg) {
      const renderer = gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) as string;
      if (/apple m\d/i.test(renderer)) return true;
    }

    // Safari returns generic "Apple GPU" — fall back to ETC extension check
    const exts = gl.getSupportedExtensions() || [];
    return exts.includes('WEBGL_compressed_texture_etc');
  } catch {
    return false;
  }
}

function detectPlatform(): Platform {
  const ua = navigator.userAgent.toLowerCase();
  const platform = (navigator as any).userAgentData?.platform?.toLowerCase()
    || navigator.platform?.toLowerCase() || '';

  if (platform.includes('mac') || ua.includes('macintosh')) {
    return isAppleSilicon()
      ? { key: 'aarch64-apple-darwin', label: 'macOS (Apple Silicon)', icon: '' }
      : { key: 'x86_64-apple-darwin', label: 'macOS (Intel)', icon: '' };
  }
  if (platform.includes('win') || ua.includes('windows')) {
    return { key: 'x86_64-pc-windows-msvc', label: 'Windows', icon: '' };
  }
  return { key: 'x86_64-unknown-linux-musl', label: 'Linux', icon: '' };
}

type Props = {
  address?: string;
  accountName?: string;
};

function DownloadSection({ address, accountName }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const detected = detectPlatform();

  const handleDownload = useCallback(async () => {
    setError(null);
    setLoading(true);

    try {
      const mnemonic = getMnemonic(address);
      if (!mnemonic) {
        setError('No wallet found. Mine at least once first.');
        return;
      }

      const referrer = loadReferrer();
      const payload = await encryptBootstrap({ mnemonic, referrer, name: accountName });
      const data = btoa(String.fromCharCode(...payload));

      const response = await fetch(BOOT_SERVER_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ platform: detected.key, data }),
      });

      if (!response.ok) {
        const text = await response.text();
        throw new Error(text || `Server error: ${response.status}`);
      }

      const disposition = response.headers.get('content-disposition') || '';
      const match = disposition.match(/filename="?([^"]+)"?/);
      const filename = match?.[1] || 'cyb-boot.zip';

      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = filename;
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
  }, [address, detected.key, accountName]);

  return (
    <div className={styles.downloadHero}>
      <div className={styles.downloadHeroContent}>
        <span className={styles.downloadBadge}>100x faster</span>
        <div className={styles.downloadHeadline}>
          Mine with GPU on Desktop
        </div>
        <div className={styles.downloadSub}>
          Your wallet and referrer transfer automatically
        </div>
        <button
          type="button"
          className={`${styles.downloadCtaBtn} ${loading ? styles.downloadCtaLoading : ''}`}
          onClick={handleDownload}
          disabled={loading}
        >
          <span className={styles.downloadCtaGlow} />
          {loading ? 'Preparing...' : `Download for ${detected.label}`}
        </button>
        {error && <div className={styles.downloadError}>{error}</div>}
      </div>
    </div>
  );
}

export default DownloadSection;
