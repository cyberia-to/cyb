import { useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { CreatedAt } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { CHAIN_ID } from 'src/constants/config';
import networkList from 'src/utils/networkListIbc';
import { HistoriesItem } from '../../../../../features/ibc-history/HistoriesItem';
import useGetStatus from '../../../../../features/ibc-history/useGetStatus';
import { AmountSend, RouteAddress, Status, TypeTsx } from './DataHistoryItems';
import styles from './DataHistoryRow.module.scss';

function getExplorerUrl(sourceChainId: string, txHash: string): string | null {
  const network = networkList[sourceChainId];
  if (!network?.explorerUrlToTx || !txHash) return null;
  return network.explorerUrlToTx.replace('{txHash}', txHash);
}

function DataHistoryRow({ item }: { item: HistoriesItem }) {
  const statusTrace = useGetStatus(item);
  const navigate = useNavigate();

  const handleClick = useCallback(() => {
    const url = getExplorerUrl(item.sourceChainId, item.txHash);
    if (!url) return;

    if (item.sourceChainId === CHAIN_ID) {
      navigate(url);
    } else {
      window.open(url, '_blank', 'noopener');
    }
  }, [item.sourceChainId, item.txHash, navigate]);

  return (
    <Display>
      <div
        className={styles.containerDataIbc}
        onClick={handleClick}
        style={{ cursor: 'pointer' }}
      >
        <div className={styles.containerDataIbcFlexBetween}>
          <TypeTsx sourceChainId={item.sourceChainId} />

          <RouteAddress item={item} />
        </div>

        <div className={styles.containerDataIbcFlexBetween}>
          <AmountSend coin={item.amount} sourceChainId={item.sourceChainId} />

          <div className={styles.containerDataIbcTimeStatus}>
            <Status status={statusTrace} />

            <CreatedAt timeAt={item.createdAt} />
          </div>
        </div>
      </div>
    </Display>
  );
}

export default DataHistoryRow;
