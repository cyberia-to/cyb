import { useMemo } from 'react';
import Display from 'src/components/containerGradient/Display/Display';
import { useChannels } from 'src/hooks/useHub';
import TitleAction from '../TitleAction/TitleAction';
import TotalCount from '../TotalCount/TotalCount';
import styles from './BridgeAction.module.scss';
import BridgeItem from './BridgeItem';

const defaultData = [
  {
    token: 'ibc/15E9C5CF5969080539DB395FA7D9C0868265217EFC528433671AAF9B1912D159',
    networkFrom: 'cosmoshub-4',
    networkTo: 'bostrom',
  },
  {
    token: 'ibc/13B2C536BB057AC79D5616B8EA1B9540EC1F2170718CAFF6F0083C966FFFED0B',
    networkFrom: 'osmosis-1',
    networkTo: 'bostrom',
  },
];

function BridgeAction() {
  const { channels } = useChannels();

  const totalCount = useMemo(() => {
    if (channels) {
      return Object.keys(channels).length - defaultData.length;
    }
    return 0;
  }, [channels]);

  const renderItem = defaultData.map((item, index) => {
    return <BridgeItem key={index} item={item} />;
  });

  return (
    <Display
      title={
        <TitleAction
          to="bridge"
          title="bridge"
          subTitle="reliable transfer of tokens from net to net"
        />
      }
    >
      <div className={styles.container}>
        {renderItem}
        {totalCount > 0 && <TotalCount text="networks" value={totalCount} to="bridge" />}
      </div>
    </Display>
  );
}

export default BridgeAction;
