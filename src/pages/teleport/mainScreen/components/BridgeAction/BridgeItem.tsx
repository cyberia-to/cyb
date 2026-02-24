import fromToIbc from 'images/fromToIbc.svg';
import { createSearchParams, Link } from 'react-router-dom';
import { DenomArr } from 'src/components';
import styles from './BridgeItem.module.scss';

type PropsBridgeItem = {
  item: {
    networkFrom: string;
    networkTo: string;
    token: string;
  };
};

function BridgeItem({ item }: PropsBridgeItem) {
  const { networkFrom, networkTo, token } = item;

  return (
    <Link
      to={{
        pathname: 'bridge',
        search: createSearchParams({
          networkFrom,
          networkTo,
          token,
        }).toString(),
      }}
      className={styles.containerBridgeItem}
    >
      <DenomArr
        denomValue={item.token}
        onlyImg
        size={45}
        // tooltipStatusImg={false}
      />
      <div className={styles.networks}>
        <DenomArr type="network" onlyImg denomValue={item.networkFrom} />
        <img src={fromToIbc} alt="fromToIbc" className={styles.networksArrow} />

        <DenomArr type="network" onlyImg denomValue={item.networkTo} />
      </div>
    </Link>
  );
}

export default BridgeItem;
