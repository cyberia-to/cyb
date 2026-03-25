import { Coin } from 'cosmjs-types/cosmos/base/v1beta1/coin';
import { CID_FOLLOW } from 'src/constants/app';
import { isParticle } from 'src/features/particle/utils';
import {
  MsgMultiSendTransaction,
  MsgSendTransaction,
} from 'src/services/backend/services/indexer/types';
import { EntityToDto } from 'src/types/dto';
import { SenseItem } from '../../redux/sense.redux';

// eslint-disable-next-line import/prefer-default-export, import/no-unused-modules
export function formatSenseItemDataToUI(
  senseItem: SenseItem,
  currentAddress: string,
  currentChatId: string
): SenseItem & {
  text: string;
  cid: string | undefined;
  amount: Coin[] | undefined;
  isAmountSendToMyAddress: boolean;
  isFollow: boolean;
} {
  const { type, memo, meta } = senseItem ?? {};

  let amount: Coin[] | undefined;
  let isAmountSendToMyAddress = false;
  let isFollow = false;
  let cid;

  switch (type) {
    case 'cosmos.bank.v1beta1.MsgSend': {
      // TODO: create meta types for particular transactions
      const { amount: a, toAddress } = meta as unknown as EntityToDto<MsgSendTransaction['value']>;
      amount = a;
      isAmountSendToMyAddress = toAddress === currentAddress;

      break;
    }

    case 'cosmos.bank.v1beta1.MsgMultiSend': {
      const v = meta as unknown as MsgMultiSendTransaction['value'];

      const a = v.outputs.find((output) => output.address === currentAddress)?.coins;

      if (a) {
        amount = a;
        isAmountSendToMyAddress = true;
      }

      break;
    }

    case 'cyber.graph.v1beta1.MsgCyberlink': {
      cid = currentChatId === meta.to ? meta.from : meta.to;

      if (meta.from === CID_FOLLOW) {
        isFollow = true;
      }

      break;
    }

    default: {
      break;
    }
  }

  // content is cid
  if (memo && isParticle(memo)) {
    cid = memo;
  }

  return {
    ...senseItem,
    text: memo || '',
    isFollow,
    cid,
    amount: amount?.filter((a) => !(a.denom === 'boot' && a.amount === '1')),
    isAmountSendToMyAddress,
  };
}
