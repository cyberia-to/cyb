import dateFormat from 'dateformat';
import Account from 'src/components/account/account';
import Tooltip from 'src/components/tooltip/tooltip';
import useGetCreator from 'src/containers/ipfs/hooks/useGetCreator';
import { timeSince } from 'src/utils/utils';
import styles from './Creator.module.scss';

function Creator({ cid, onlyTime }: { cid: string; onlyTime?: boolean }) {
  const { creator } = useGetCreator(cid);

  if (!creator) {
    return null;
  }

  return (
    <>
      <Tooltip
        placement="bottom"
        tooltip={dateFormat(new Date(creator.timestamp), 'dd/mm/yyyy, HH:MM')}
      >
        <span className={styles.date}>
          {timeSince(Date.now() - Date.parse(creator.timestamp))} ago
        </span>
      </Tooltip>

      {!onlyTime && <Account address={creator.address} avatar />}
    </>
  );
}

export default Creator;
