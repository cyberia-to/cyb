import ActionBar from 'src/components/actionBar';
import { Dots } from 'src/components';

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
}: Props) {
  const canMine = blockReady && difficulty && address;
  const isMining = miningStatus?.mining;

  if (submitting) {
    return (
      <ActionBar
        button={{
          text: 'Stop Mining',
          onClick: onStopMining,
        }}
        text={<>Submitting proof<Dots /></>}
      />
    );
  }

  if (isMining || autoMining) {
    return (
      <ActionBar
        button={{
          text: 'Stop Mining',
          onClick: onStopMining,
        }}
        text={`${miningStatus?.hashrate.toFixed(0) || 0} H/s | auto-submit on`}
      />
    );
  }

  const missingParts = [
    !address && 'wallet',
    !blockReady && 'block data (loading...)',
    !difficulty && 'difficulty (loading...)',
  ].filter(Boolean);

  return (
    <ActionBar
      button={{
        text: 'Start Mining',
        onClick: onStartMining,
        disabled: !canMine,
      }}
      text={!canMine ? `Waiting for: ${missingParts.join(', ')}` : 'Auto-submit enabled'}
    />
  );
}

export default MiningActionBar;
