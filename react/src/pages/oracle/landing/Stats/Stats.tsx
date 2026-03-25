import cx from 'classnames';
import { Link } from 'react-router-dom';
import { useGetGraphStats, useGetNegentropy } from 'src/containers/temple/hooks';
import { TypingText } from 'src/containers/temple/pages/play/PlayBanerContent';
import { routes } from 'src/routes';
import { formatNumber, timeSince } from 'src/utils/utils';
import { TitleType } from '../type';
import styles from './Stats.module.scss';

type Props = {
  type: TitleType;
};

const REFETCH_INTERVAL = 1000 * 7;

function Stats({ type }: Props) {
  const dataGetGraphStats = useGetGraphStats(REFETCH_INTERVAL);
  const negentropy = useGetNegentropy(REFETCH_INTERVAL);
  let value: number | undefined;
  let text: string | JSX.Element;
  let change: number | undefined;
  let time = timeSince(dataGetGraphStats.changeTimeAmount.time);

  switch (type) {
    case TitleType.search:
      value = dataGetGraphStats.data?.particles;
      text = <Link to={routes.oracle.ask.getLink('particle')}>particles</Link>;
      if (dataGetGraphStats.changeTimeAmount.particles) {
        change = dataGetGraphStats.changeTimeAmount.particles;
      }
      break;

    case TitleType.learning:
      value = dataGetGraphStats.data?.cyberlinks;
      text = <Link to={routes.oracle.ask.getLink('cyberlink')}>cyberlinks</Link>;
      if (dataGetGraphStats.changeTimeAmount.cyberlinks) {
        change = dataGetGraphStats.changeTimeAmount.cyberlinks;
      }
      break;

    case TitleType.ai:
      value = negentropy.data?.negentropy;
      text = <Link to={routes.oracle.ask.getLink('negentropy')}>syntropy bits</Link>;
      if (negentropy.changeTimeAmount.amount) {
        change = negentropy.changeTimeAmount.amount;
        time = timeSince(negentropy.changeTimeAmount.time);
      }
      break;

    default:
  }

  return (
    <div className={cx(styles.wrapper)}>
      {value && (
        <>
          <TypingText
            content={`${Number(value).toLocaleString().replaceAll(',', ' ')}`}
            delay={40}
          />{' '}
          <strong>{text}</strong>{' '}
          {change ? (
            <p className={styles.change}>
              | <strong>+{formatNumber(change)}</strong> in {time}
            </p>
          ) : (
            <>
              and <strong>growing</strong>
            </>
          )}
        </>
      )}
    </div>
  );
}

export default Stats;
