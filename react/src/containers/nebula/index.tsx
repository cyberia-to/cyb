/* eslint-disable no-await-in-loop */

import BigNumber from 'bignumber.js';
import { useEffect, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import TokenChange from 'src/components/TokenChange/TokenChange';
import { DENOM_LIQUID } from 'src/constants/config';
import { useAppData } from 'src/contexts/appData';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { v4 as uuidv4 } from 'uuid';
import { ContainerGradient, DenomArr, FormatNumberTokens, MainContainer } from '../../components';
import { getDisplayAmount, replaceSlash } from '../../utils/utils';
// import { getMarketData } from './getMarketData';
import { ColItem, NebulaImg, RowItem } from './components';

function Title({ capData }: { capData: { currentCap: number; change: number } }) {
  return (
    <div
      style={{
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        width: '100%',
        // paddingBottom: '20px',
      }}
    >
      <div style={{ fontSize: '22px', width: '112px', height: '112px' }}>
        <NebulaImg />
      </div>
      <TokenChange total={capData.currentCap} change={capData.change} />
    </div>
  );
}

function Nebula() {
  const { tracesDenom } = useIbcDenom();
  const { dataTotalSupply, marketData } = useAppData();
  const [capData, setCapData] = useState({ currentCap: 0, change: 0 });

  useAdviserTexts({
    defaultText: 'welcome to nebula',
  });

  useEffect(() => {
    if (Object.keys(dataTotalSupply).length > 0) {
      let cap = 0;
      Object.keys(dataTotalSupply).forEach((key) => {
        const amount = dataTotalSupply[key];
        const [{ coinDecimals }] = tracesDenom(key);
        const reduceAmount = getDisplayAmount(amount, coinDecimals);
        if (Object.keys(marketData).length > 0 && Object.hasOwn(marketData, key)) {
          const poolPrice = new BigNumber(marketData[key]);
          const tempCap = poolPrice
            .multipliedBy(Number(reduceAmount))
            .dp(0, BigNumber.ROUND_FLOOR)
            .toNumber();
          cap += tempCap;
        }
      });

      if (cap > 0) {
        const localStorageDataCap = localStorage.getItem('lastCap');
        if (localStorageDataCap !== null) {
          const lastCap = new BigNumber(localStorageDataCap);
          let change = 0;
          change = new BigNumber(cap).minus(lastCap).toNumber();
          setCapData((item) => ({ ...item, currentCap: cap }));
          if (new BigNumber(cap).comparedTo(lastCap) !== 0) {
            setCapData((item) => ({ ...item, change }));
          }
        } else {
          setCapData({ currentCap: cap, change: 0 });
        }
        localStorage.setItem('lastCap', cap);
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataTotalSupply, marketData, tracesDenom]);

  const dataRenderItems = useMemo(() => {
    let dataObj = {};
    if (Object.keys(dataTotalSupply).length > 0) {
      Object.keys(dataTotalSupply).forEach((key) => {
        const amount = dataTotalSupply[key];
        let price = 0;
        let cap = 0;
        const [{ coinDecimals }] = tracesDenom(key);
        const reduceAmount = getDisplayAmount(amount, coinDecimals);

        if (Object.keys(marketData).length > 0 && Object.hasOwn(marketData, key)) {
          const poolPrice = new BigNumber(marketData[key]);
          cap = poolPrice
            .multipliedBy(Number(reduceAmount))
            .dp(0, BigNumber.ROUND_FLOOR)
            .toNumber();
          price = poolPrice.toNumber();
        }
        dataObj[key] = {
          supply: reduceAmount,
          price,
          cap,
        };
      });
    }
    if (Object.keys(dataObj).length > 0) {
      const sortable = Object.fromEntries(
        Object.entries(dataObj).sort(([, a], [, b]) => b.cap - a.cap)
      );
      dataObj = sortable;
    }
    return dataObj;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataTotalSupply, marketData, tracesDenom]);

  const getTypeDenomKey = (key) => {
    const denom = tracesDenom(key);

    if (denom[0].denom.includes('ibc')) {
      return replaceSlash(denom[0].denom);
    }

    if (key.includes('pool')) {
      return `${getTypeDenomKey(denom[0].denom)}-${getTypeDenomKey(denom[1].denom)}`;
    }

    return denom[0].denom;
  };

  const getLinktoSearch = (key) => {
    return `/search/${getTypeDenomKey(key)}`;
  };

  const itemRowMarketData = useMemo(() => {
    return Object.keys(dataRenderItems).map((key) => {
      const keyItem = uuidv4();
      return (
        <RowItem key={keyItem}>
          <ColItem>
            <Link to={getLinktoSearch(key)}>
              <DenomArr denomValue={key} />
            </Link>
          </ColItem>
          <ColItem justifyContent="flex-end">
            <FormatNumberTokens
              // text={key}
              value={dataRenderItems[key].supply}
              tooltipStatusImg={false}
            />
          </ColItem>
          <ColItem justifyContent="flex-end">
            <FormatNumberTokens
              text={DENOM_LIQUID}
              value={dataRenderItems[key].price}
              tooltipStatusImg={false}
            />
          </ColItem>
          <ColItem justifyContent="flex-end">
            <FormatNumberTokens
              value={dataRenderItems[key].cap}
              text={DENOM_LIQUID}
              tooltipStatusImg={false}
            />
          </ColItem>
        </RowItem>
      );
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dataRenderItems, getLinktoSearch]);

  return (
    <MainContainer>
      <ContainerGradient
        userStyleContent={{ minHeight: 'auto', height: 'unset' }}
        title={<Title capData={capData} />}
        togglingDisable
      >
        <div>{itemRowMarketData}</div>
      </ContainerGradient>
    </MainContainer>
  );
}

export default Nebula;
