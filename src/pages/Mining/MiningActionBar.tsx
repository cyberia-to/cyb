import ActionBar from 'src/components/actionBar';
import { Dots } from 'src/components';
import BackendSelector from './components/BackendSelector';
import ThreadSelector from './components/ThreadSelector';

type MiningStatus = {
  mining: boolean;
  hashrate: number;
  total_hashes: number;
  elapsed_secs: number;
  pending_proofs: number;
};

type Props = {
  difficulty: number;
  minDifficulty: number;
  address: string | undefined;
  blockReady: boolean;
  autoMining: boolean;
  submitting: boolean;
  miningStatus: MiningStatus | null;
  onStartMining: () => void;
  onStopMining: () => void;
  onDifficultyChange: (v: number) => void;
  // Backend & thread selectors
  backend: string;
  onBackendChange: (v: string) => void;
  availableBackends: string[];
  activeBackend?: string;
  threadCount: number;
  onThreadCountChange: (v: number) => void;
  maxThreads: number;
  isNative: boolean;
};

function MiningActionBar({
  difficulty,
  minDifficulty,
  address,
  blockReady,
  autoMining,
  submitting,
  miningStatus,
  onStartMining,
  onStopMining,
  onDifficultyChange,
  backend,
  onBackendChange,
  availableBackends,
  activeBackend,
  threadCount,
  onThreadCountChange,
  maxThreads,
  isNative,
}: Props) {
  const canMine = blockReady && difficulty >= minDifficulty && address;
  const isMining = miningStatus?.mining || autoMining;
  const disabled = isMining;

  const selectors = (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 12 }}>
      {/* Difficulty selector */}
      <span style={{ display: 'inline-flex', alignItems: 'center', gap: 2, fontSize: 12 }}>
        <span style={{ color: '#888' }}>d:</span>
        <button
          type="button"
          onClick={() => { if (difficulty > minDifficulty) onDifficultyChange(difficulty - 1); }}
          disabled={!!disabled || difficulty <= minDifficulty}
          style={{
            background: 'transparent',
            border: '1px solid rgba(54, 214, 174, 0.3)',
            borderRadius: 3,
            color: '#36d6ae',
            fontSize: 12,
            cursor: 'pointer',
            padding: '0 4px',
            lineHeight: '18px',
          }}
        >
          −
        </button>
        <span style={{
          minWidth: 24,
          textAlign: 'center',
          color: '#36d6ae',
          fontFamily: 'monospace',
          fontSize: 12,
        }}>
          {difficulty}
        </span>
        <button
          type="button"
          onClick={() => { if (difficulty < 64) onDifficultyChange(difficulty + 1); }}
          disabled={!!disabled || difficulty >= 64}
          style={{
            background: 'transparent',
            border: '1px solid rgba(54, 214, 174, 0.3)',
            borderRadius: 3,
            color: '#36d6ae',
            fontSize: 12,
            cursor: 'pointer',
            padding: '0 4px',
            lineHeight: '18px',
          }}
        >
          +
        </button>
      </span>
      {isNative && availableBackends.length > 1 && (
        <BackendSelector
          value={backend}
          onChange={onBackendChange}
          availableBackends={availableBackends}
          activeBackend={activeBackend}
          disabled={disabled}
        />
      )}
      {(!isNative || backend === 'cpu') && (
        <ThreadSelector
          value={threadCount}
          onChange={onThreadCountChange}
          max={maxThreads}
          total={navigator.hardwareConcurrency || 4}
          disabled={disabled}
        />
      )}
    </span>
  );

  // Status text: always a single line, fixed structure
  let statusText: React.ReactNode = null;
  if (isMining) {
    statusText = submitting
      ? <> Submitting proof<Dots /></>
      : <> {miningStatus?.hashrate.toFixed(0) || 0} H/s</>;
  } else if (!canMine) {
    const missingParts = [
      !address && 'wallet',
      !blockReady && 'block data',
      difficulty < minDifficulty && `difficulty < ${minDifficulty}`,
    ].filter(Boolean);
    statusText = <> Waiting: {missingParts.join(', ')}...</>;
  }

  return (
    <ActionBar
      button={{
        text: isMining ? 'Stop Mining' : 'Start Mining',
        onClick: isMining ? onStopMining : onStartMining,
        disabled: !isMining && !canMine,
      }}
      text={
        <>
          {selectors}
          {statusText}
        </>
      }
    />
  );
}

export default MiningActionBar;
