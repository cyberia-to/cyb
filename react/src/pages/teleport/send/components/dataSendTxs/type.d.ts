import { TxBody } from 'cosmjs-types/cosmos/tx/v1beta1/tx';
import { Option } from 'src/types';

type TxsResponseCustom = {
  code: number;
  height: string;
  txhash: string;
  timestamp: string;
  tx: {
    '@type': string;
    body: TxBody;
  };
};

export type DataSendTxs = {
  data: Option<TxsResponseCustom[]>;
  fetchNextPage: () => void;
  hasNextPage: boolean;
};
