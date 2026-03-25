import { trimString } from '../../../utils/utils';
import { CardItem } from '../codes/code';
import { CardCantainer, LinkCreator, LinkTx } from '../ui/ui';

function InitializationInfo({ initTxHash, details }) {
  return (
    <CardCantainer>
      <CardItem
        title="Inst. tx"
        value={<LinkTx txs={initTxHash}>{trimString(initTxHash, 6, 6)}</LinkTx>}
      />
      <CardItem
        title="Creator"
        value={
          <LinkCreator address={details.creator}>{trimString(details.creator, 10)}</LinkCreator>
        }
      />
      <CardItem
        title="Admin"
        value={
          details.admin ? (
            <LinkCreator address={details.creator}>{trimString(details.creator, 10)}</LinkCreator>
          ) : (
            '-'
          )
        }
      />
    </CardCantainer>
  );
}

export default InitializationInfo;
