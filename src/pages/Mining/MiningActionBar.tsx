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
  difficulty: number | undefined;
  address: string | undefined;
  blockReady: boolean;
  autoMining: boolean;
  submitting: boolean;
  miningStatus: MiningStatus | null;
  onStartMining: () => void;
  onStopMining: () => void;
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
  address,
  blockReady,
  autoMining,
  submitting,
  miningStatus,
  onStartMining,
  onStopMining,
  backend,
  onBackendChange,
  availableBackends,
  activeBackend,
  threadCount,
  onThreadCountChange,
  maxThreads,
  isNative,
}: Props) {
  const canMine = blockReady && difficulty && address;
  const isMining = miningStatus?.mining || autoMining;
  const disabled = isMining;

  const selectors = (
    <span style={{ display: 'inline-flex', alignItems: 'center', gap: 12 }}>
      {isNative && availableBackends.length > 1 && (
        <BackendSelector
          value={backend}
          onChange={onBackendChange}
          availableBackends={availableBackends}
          activeBackend={activeBackend}
          disabled={disabled}
        />
      )}
      <ThreadSelector
        value={threadCount}
        onChange={onThreadCountChange}
        max={maxThreads}
        disabled={disabled}
      />
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
      !difficulty && 'difficulty',
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
