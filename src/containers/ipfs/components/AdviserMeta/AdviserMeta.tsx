import { Link } from 'react-router-dom';
import { Account } from 'src/components';
import { LLMAvatar, useIsLLMPageParam } from 'src/containers/Search/LLMSpark/LLMSpark';
import useRank from 'src/features/cyberlinks/rank/useRank';
import { routes } from 'src/routes';
import { formatCurrency, timeSince } from 'src/utils/utils';
import useGetCreator from '../../hooks/useGetCreator';
import { PREFIXES } from '../metaInfo';
import styles from './AdviserMeta.module.scss';

type Props = {
  cid: string;
  type: string | undefined;
  size: number | bigint | undefined;
};

function AdviserMeta({ cid, type, size }: Props) {
  const { creator } = useGetCreator(cid);
  const rank = useRank(cid);

  const isLLM = useIsLLMPageParam();

  return (
    <div className={styles.meta}>
      <div className={styles.left}>
        <span>{type}</span>

        {!!rank && (
          <div className={styles.rank}>
            <span className={styles.number}>{rank.toLocaleString().replaceAll(',', ' ')}</span>
            <Link to="https://docs.cyb.ai/#/page/cyberank" replace target="_blank">
              🦠
            </Link>
            {/* <Rank hash={cid} rank={rank} /> */}
          </div>
        )}
      </div>

      {isLLM ? (
        <LLMAvatar />
      ) : (
        // eslint-disable-next-line react/jsx-no-useless-fragment
        creator && (
          <div className={styles.center}>
            <span className={styles.date}>
              {timeSince(Date.now() - Date.parse(creator.timestamp))} ago
            </span>
            <Account sizeAvatar="20px" address={creator.address} avatar />
          </div>
        )
      )}

      <div className={styles.right}>
        <span>🟥 {size ? formatCurrency(size, 'B', 0, PREFIXES) : 'unknown'}</span>
        <Link to={routes.robot.routes.soul.path}>🌓</Link>
      </div>
    </div>
  );
}

export default AdviserMeta;
