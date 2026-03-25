import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { DenomArr, FormatNumberTokens } from 'src/components';
import { DENOM_LIQUID } from 'src/constants/config';
import { useAppData } from 'src/contexts/appData';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { replaceSlash } from '../../../utils/utils';

function PoolItemsList({ assets, token }) {
  const { tracesDenom } = useIbcDenom();
  const { marketData } = useAppData();

  const amounToken = useMemo(() => {
    if (assets && Object.hasOwn(assets, token)) {
      const amount = assets[token];
      return amount;
    }
    return 0;
  }, [assets, token]);

  const usePrice = useMemo(() => {
    if (Object.keys(marketData).length > 0 && Object.hasOwn(marketData, token)) {
      const price = new BigNumber(marketData[token]);
      return price.toNumber();
    }
    return 0;
  }, [token, marketData]);

  const useCap = useMemo(() => {
    if (usePrice > 0) {
      return new BigNumber(amounToken)
        .multipliedBy(usePrice)
        .dp(0, BigNumber.ROUND_FLOOR)
        .toNumber();
    }
    return 0;
  }, [amounToken, usePrice]);

  const getTypeDenomKey = (key: string) => {
    const [{ denom }] = tracesDenom(key);

    if (denom.includes('ibc')) {
      return replaceSlash(denom);
    }

    return denom;
  };

  const getLinktoSearch = (key) => {
    return `/search/${getTypeDenomKey(key)}`;
  };

  return (
    <div
      style={{
        display: 'grid',
        gridTemplateColumns: '100px 1fr 0.9fr 1fr',
        gap: '10px',
        alignItems: 'baseline',
        height: '40px',
      }}
    >
      <Link to={getLinktoSearch(token)}>
        <DenomArr denomValue={token} onlyText />
      </Link>
      <FormatNumberTokens marginContainer="0px" value={amounToken} text={token} />
      <FormatNumberTokens marginContainer="0px" value={usePrice} text={DENOM_LIQUID} />
      <FormatNumberTokens marginContainer="0px" value={useCap} text={DENOM_LIQUID} />
    </div>
  );
}

export default PoolItemsList;
