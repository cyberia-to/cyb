import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { Transition } from 'react-transition-group';
import { DenomArr, FormatNumberTokens } from 'src/components';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { convertAmount, replaceSlash } from '../../../../../utils/utils';
import BtnArrow from '../BtnArrow';
import ChartTotal from '../ChartTotal';
import DetailsBalance from '../DetailsBalance';
import styles from './styles.module.scss';

const cx = require('classnames');

function RowBalancesDetails({ balance }) {
  const { tracesDenom } = useIbcDenom();
  const [isOpen, setIsOpen] = useState(false);

  const onClickBtnArrow = () => {
    setIsOpen((item) => !item);
  };

  const checkDetailsToken = useMemo(() => {
    if (Object.hasOwn(balance, 'total') && Object.keys(balance).length >= 5) {
      return true;
    }

    return false;
  }, [balance]);

  const useAmountTotal = useMemo(() => {
    if (balance.total) {
      const { amount, denom } = balance.total;

      const [{ coinDecimals }] = tracesDenom(denom);

      return convertAmount(amount, coinDecimals);
    }

    return 0;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [balance, tracesDenom]);

  const getTypeDenomKey = (key) => {
    const denom = tracesDenom(key);

    // TODO: fix
    if (!denom[0]) {
      return;
    }

    if (denom[0].denom.includes('ibc')) {
      return replaceSlash(denom[0].denom);
    }

    if (key.includes('pool') && denom[1]) {
      return `${getTypeDenomKey(denom[0].denom)}-${getTypeDenomKey(denom[1].denom)}`;
    }

    return denom[0].denom;
  };

  const getLinktoSearch = (key) => {
    return `/search/${getTypeDenomKey(key)}`;
  };

  return (
    <div style={{ display: 'grid' }}>
      <div className={styles.container}>
        <Link to={getLinktoSearch(balance.total.denom)}>
          <DenomArr denomValue={balance.total.denom} />
        </Link>

        <div
          style={{
            display: 'grid',
            gridTemplateColumns: '90px 1fr',
            alignItems: 'baseline',
            gap: '10px',
          }}
        >
          <button
            type="button"
            onClick={checkDetailsToken ? onClickBtnArrow : null}
            style={{
              display: 'grid',
              gridTemplateColumns: '8px 1fr',
              alignItems: 'center',
              gap: '10px',
              outline: 'none',
              background: 'transparent',
              border: 'none',
              cursor: checkDetailsToken ? 'pointer' : 'default',
            }}
          >
            {checkDetailsToken ? <BtnArrow open={isOpen} /> : <div />}
            <ChartTotal balance={balance} />
          </button>
          <FormatNumberTokens value={useAmountTotal} />
        </div>
        <FormatNumberTokens text={balance.price.denom} value={balance.price.amount} />
        <FormatNumberTokens text={balance.cap.denom} value={balance.cap.amount} />
      </div>
      {checkDetailsToken && (
        <Transition in={isOpen} timeout={300}>
          {(state) => {
            return (
              <div
                className={cx(
                  styles.containerDetailsBalance,
                  styles[`containerDetailsBalance${state}`],
                  styles[`length-${Object.keys(balance).length}`]
                )}
              >
                <DetailsBalance data={balance} />
              </div>
            );
          }}
        </Transition>
      )}
    </div>
  );
}

export default RowBalancesDetails;
