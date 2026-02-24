import { Coin } from '@cosmjs/launchpad';
import cx from 'classnames';
import downArrow from 'images/down-arrow.png';
import ImgArrow from 'images/fromToIbc.svg';
import complete from 'images/statusTx/complete.png';
import pending from 'images/statusTx/pending.png';
import refunded from 'images/statusTx/refunded.png';
import timeout from 'images/statusTx/timeout.png';
import upArrow from 'images/up-arrow.png';
import { useMemo } from 'react';
import { Account, DenomArr, FormatNumberTokens } from 'src/components';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { HistoriesItem, StatusTx } from 'src/features/ibc-history/HistoriesItem';
import { CssVariables } from 'src/style/variables';
import { getDisplayAmount, trimString } from 'src/utils/utils';
import { TxsType } from '../../../type';
import styles from './DataHistoryItems.module.scss';

const mapStatusTxImg = {
  [StatusTx.COMPLETE]: complete,
  [StatusTx.PENDING]: pending,
  [StatusTx.TIMEOUT]: timeout,
  [StatusTx.REFUNDED]: refunded,
};

export function TypeTsx({ sourceChainId }: { sourceChainId: string }) {
  const isCyberChain = sourceChainId === CHAIN_ID;

  return (
    <div className={styles.containerTypeTsx}>
      <img src={isCyberChain ? upArrow : downArrow} alt="imgArrow" />
      <div>{isCyberChain ? TxsType.Withdraw : TxsType.Deposit}</div>
    </div>
  );
}

export function AmountSend({ coin, sourceChainId }: { coin: Coin; sourceChainId: string }) {
  const { tracesDenom } = useIbcDenom();
  const typeTxs = sourceChainId === CHAIN_ID ? TxsType.Withdraw : TxsType.Deposit;
  const amountDenom = useMemo(() => {
    if (tracesDenom) {
      const [{ coinDecimals }] = tracesDenom(coin.denom);
      const amount = getDisplayAmount(coin.amount, coinDecimals);
      return amount;
    }

    return 0;
  }, [coin, tracesDenom]);

  return (
    <FormatNumberTokens
      text={coin.denom}
      value={amountDenom}
      styleValue={{
        color:
          typeTxs === TxsType.Deposit ? CssVariables.GREEN_LIGHT_COLOR : CssVariables.RED_COLOR,
        fontSize: '18px',
      }}
    />
  );
}

export function Status({ status }: { status: StatusTx }) {
  return (
    <div className={styles.containerTypeTsx}>
      <img src={mapStatusTxImg[status]} alt={status} />
      <div>{status}</div>
    </div>
  );
}

function AddressIbc({ address }: { address: string }) {
  return <div className={styles.addressIbc}>{trimString(address, 8, 2)}</div>;
}

function addressType(address: string) {
  if (address.match(PATTERN_CYBER)) {
    return <Account address={address} />;
  }
  return <AddressIbc address={address} />;
}

export function RouteAddress({ item }: { item: HistoriesItem }) {
  const { recipient, sender, sourceChainId, destChainId } = item;

  return (
    <div className={styles.containerRouteAddress}>
      <div className={styles.containerAddressChainId}>
        {addressType(sender)}
        <DenomArr type="network" onlyImg denomValue={sourceChainId} />
      </div>
      <img src={ImgArrow} alt="ImgArrow" className={styles.fromToIbcImg} />
      <div className={cx(styles.containerAddressChainId, styles.containerAddressChainIdMarginLeft)}>
        {addressType(recipient)}
        <DenomArr type="network" onlyImg denomValue={destChainId} />
      </div>
    </div>
  );
}
