import MsgType from '../../msgType/msgType';

type Props = {
  type: string;
  value: any;
  accountUser: string;
};

function MsgTypeTxs({ type, value, accountUser }: Props) {
  let typeTx = type;

  if (typeTx.includes('MsgSend') && value?.to_address === accountUser) {
    typeTx = 'Receive';
  }

  return <MsgType type={typeTx} />;
}

export default MsgTypeTxs;
