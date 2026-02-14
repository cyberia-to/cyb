import { FormatNumber } from 'src/components';
import { formatNumber } from 'src/utils/utils';
import styles from './VotingPower.module.scss';

function VotingPower({
  data,
}: {
  data: { powerPercent: string; tokens: string };
}) {
  const { powerPercent, tokens } = data;
  return (
    <div className={styles.container}>
      <FormatNumber currency="%" number={powerPercent} />
      <span className={styles.tokens}>{formatNumber(tokens)}</span>
    </div>
  );
}

export default VotingPower;
