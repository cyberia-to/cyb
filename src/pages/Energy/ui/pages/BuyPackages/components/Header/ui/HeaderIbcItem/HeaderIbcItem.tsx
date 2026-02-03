import { useAppSelector } from 'src/redux/hooks';
import { StatusTx } from 'src/features/ibc-history/HistoriesItem';
import StatusIbc from '../../../StatusIbc/StatusIbc';

function HeaderIbcItem() {
  const { ibcResult } = useAppSelector((state) => state.energy);

  let contentStatus = <>send to bostrom via IBC</>;

  if (ibcResult) {
    contentStatus = <StatusIbc />;
  }

  if (ibcResult && ibcResult.status === StatusTx.COMPLETE) {
    contentStatus = <>sended to bostrom</>;
  }

  return <span>{contentStatus}</span>;
}

export default HeaderIbcItem;
