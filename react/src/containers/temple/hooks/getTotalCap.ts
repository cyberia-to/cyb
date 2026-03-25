import BigNumber from 'bignumber.js';
import { useEffect, useState } from 'react';
import { BASE_DENOM } from 'src/constants/config';
import { useAppData } from 'src/contexts/appData';
import { useIbcDenom } from 'src/contexts/ibcDenom';
import { ibcDenomAtom } from 'src/pages/teleport/bridge/bridge';
import { getDisplayAmount } from '../../../utils/utils';

function useGetTotalCap() {
  const { marketData, dataTotalSupply } = useAppData();
  const { tracesDenom } = useIbcDenom();
  const [capData, setCapData] = useState({
    currentCap: 0,
    change: { amount: 0, time: 0 },
  });

  const [priceData, setPriceData] = useState({
    price: 0,
    change: { amount: 0, time: 0 },
  });

  useEffect(() => {
    if (Object.keys(dataTotalSupply).length > 0 && Object.keys(marketData).length > 0) {
      let cap = 0;
      Object.keys(dataTotalSupply).forEach((key) => {
        const amount = dataTotalSupply[key];
        const [{ coinDecimals }] = tracesDenom(key);
        const reduceAmount = getDisplayAmount(amount, coinDecimals);
        if (Object.hasOwn(marketData, key)) {
          const poolPrice = new BigNumber(marketData[key]);
          const tempCap = poolPrice
            .multipliedBy(Number(reduceAmount))
            .dp(0, BigNumber.ROUND_FLOOR)
            .toNumber();
          cap += tempCap;
        }
      });

      if (cap > 0) {
        const d = new Date();
        const localStorageDataCap = localStorage.getItem('lastCap-temple');
        if (localStorageDataCap !== null) {
          const oldData = JSON.parse(localStorageDataCap);
          const lastCap = new BigNumber(oldData.cap);
          let change = new BigNumber(0);
          if (new BigNumber(cap).comparedTo(lastCap)) {
            const procent = lastCap.dividedBy(cap);
            change = BigNumber(1).minus(procent).multipliedBy(100).dp(2, BigNumber.ROUND_FLOOR);
          } else {
            const procent = new BigNumber(cap).dividedBy(lastCap);
            change = BigNumber(1)
              .minus(procent)
              .multipliedBy(100)
              .dp(2, BigNumber.ROUND_FLOOR)
              .multipliedBy(-1);
          }
          // const change = new BigNumber(cap).minus(lastCap).toNumber();
          const timeChange = Date.parse(d) - Date.parse(oldData.timestamp);

          if (new BigNumber(cap).comparedTo(lastCap) !== 0 && timeChange > 0) {
            setCapData((item) => ({
              ...item,
              change: { amount: change.toNumber(), time: timeChange },
            }));
          }
        }
        setCapData((item) => ({ ...item, currentCap: cap }));
        localStorage.setItem('lastCap-temple', JSON.stringify({ cap, timestamp: d }));
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [marketData, dataTotalSupply, tracesDenom]);

  useEffect(() => {
    if (Object.keys(marketData).length > 0) {
      if (Object.hasOwn(marketData, ibcDenomAtom) && Object.hasOwn(marketData, BASE_DENOM)) {
        const priceBoot = new BigNumber(marketData[BASE_DENOM]);
        const priceAtom = new BigNumber(marketData[ibcDenomAtom]);
        const priceBootForAtom = priceAtom
          .dividedBy(priceBoot)
          .dp(0, BigNumber.ROUND_FLOOR)
          .toNumber();

        const localStorageDataPrice = localStorage.getItem('lastPrice-Boot-Atom');

        setPriceData((item) => ({
          ...item,
          price: priceBootForAtom,
        }));
        const d = new Date();

        if (localStorageDataPrice !== null) {
          const oldData = JSON.parse(localStorageDataPrice);
          const timeChange = Date.parse(d) - Date.parse(oldData.timestamp);
          let amountChangeProcent = new BigNumber(0);
          if (new BigNumber(priceBootForAtom).comparedTo(oldData.priceBootForAtom)) {
            const procent = new BigNumber(oldData.priceBootForAtom).dividedBy(priceBootForAtom);
            amountChangeProcent = new BigNumber(1)
              .minus(procent)
              .multipliedBy(100)
              .dp(2, BigNumber.ROUND_FLOOR)
              .multipliedBy(-1);
          }

          if (new BigNumber(oldData.priceBootForAtom).comparedTo(priceBootForAtom)) {
            const procent = new BigNumber(priceBootForAtom).dividedBy(oldData.priceBootForAtom);
            amountChangeProcent = new BigNumber(1)
              .minus(procent)
              .multipliedBy(100)
              .dp(2, BigNumber.ROUND_FLOOR);
          }

          if (timeChange > 0) {
            setPriceData((item) => ({
              ...item,
              change: {
                amount: amountChangeProcent.toNumber(),
                time: timeChange,
              },
            }));
          }
        }

        localStorage.setItem(
          'lastPrice-Boot-Atom',
          JSON.stringify({ priceBootForAtom, timestamp: d })
        );
      }
    }
  }, [marketData]);

  return { capData, priceData };
}

export default useGetTotalCap;
