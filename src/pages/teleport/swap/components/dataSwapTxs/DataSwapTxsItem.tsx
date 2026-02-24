import rectangle from 'images/rectangle.svg';
import swapImg from 'images/sync-outline.svg';
import { Link } from 'react-router-dom';
import { AmountDenom, CreatedAt, FormatNumberTokens } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import { Colors } from 'src/components/containerGradient/types';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { MessagesByAddressQuery } from 'src/generated/graphql';
import { CssVariables } from 'src/style/variables';
import { ArrayElement } from 'src/types';
import { getDisplayAmountReverce } from 'src/utils/utils';
import useGetResultSwap from '../../../hooks/useGetResultSwap';
import styles from './DataSwapTxs.module.scss';

function getDataOrder(value, coinDecimalsA: number) {
  const orderPrice = value.order_price;
  const tokenA = value.offer_coin.denom;
  const tokenB = value.demand_coin_denom;
  let counterPairAmount = 0;

  if ([tokenA, tokenB].sort()[0] === tokenA) {
    const swapPrice = 1 / orderPrice;
    counterPairAmount = swapPrice;
  } else {
    const amountTokenA = getDisplayAmountReverce(1, coinDecimalsA);
    counterPairAmount = orderPrice * parseFloat(amountTokenA);
  }

  return {
    counterPairAmount,
  };
}

function DataSwapTxsItem({
  item,
}: {
  item: ArrayElement<MessagesByAddressQuery['messages_by_address']>;
}) {
  const { tracesDenom } = useIbcDenom();
  const dataResultSwap = useGetResultSwap(item.transaction.height, item.transaction.logs);

  const { value } = item;

  const tokenA = value.offer_coin.denom;
  const tokenAAmount = value.offer_coin.amount;
  const tokenB = value.demand_coin_denom;

  const [{ coinDecimals: coinDecimalsA }] = tracesDenom(tokenA);
  const { counterPairAmount } = getDataOrder(value, coinDecimalsA);

  return (
    <Link to={`/network/bostrom/tx/${item.transaction_hash}`}>
      <Display color={item.transaction.success ? Colors.BLUE : Colors.RED}>
        <div className={styles.containerDataSwapTxsItem}>
          <div className={styles.containerTitle}>
            <div className={styles.containerSwapImg}>
              <img src={swapImg} alt="swapImg" /> <span>swap</span>
            </div>
            <div className={styles.containerPrice}>
              <FormatNumberTokens text={tokenA} value={1} />
              <img src={rectangle} alt="img" />
              <FormatNumberTokens
                text={tokenB}
                value={counterPairAmount}
                styleContainer={{
                  display: 'flex',
                  flexDirection: 'row-reverse',
                }}
              />
            </div>
          </div>
          <div className={styles.containerContent}>
            <div className={styles.containerTime}>
              <CreatedAt timeAt={item.transaction.block.timestamp} />
            </div>

            <div>{dataResultSwap?.success && dataResultSwap.success}</div>
            <div className={styles.containerAmountCoins}>
              <AmountDenom
                denom={tokenA}
                amountValue={
                  dataResultSwap && parseFloat(dataResultSwap.offerCoin.amount) > 0
                    ? dataResultSwap.offerCoin.amount
                    : tokenAAmount
                }
                styleValue={{ color: CssVariables.RED_COLOR }}
              />
              {dataResultSwap?.demandCoin && (
                <AmountDenom
                  denom={dataResultSwap.demandCoin.denom}
                  amountValue={dataResultSwap.demandCoin.amount}
                  styleValue={{ color: CssVariables.GREEN_LIGHT_COLOR }}
                />
              )}
            </div>
          </div>
        </div>
      </Display>
    </Link>
  );
}

export default DataSwapTxsItem;
