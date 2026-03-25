import { Battery, Pane, Text } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';
import { Link } from 'react-router-dom';
import { BASE_DENOM, CHAIN_ID } from 'src/constants/config';
import { useQueryClient } from 'src/contexts/queryClient';
import { routes } from 'src/routes';
import { Networks } from 'src/types/networks';
import { setBandwidth } from '../../redux/actions/bandwidth';
import { coinDecimals, convertResources, formatCurrency, reduceBalances } from '../../utils/utils';
import Tooltip from '../tooltip/tooltip';

const PREFIXES = [
  {
    prefix: 't',
    power: 10 ** 12,
  },
  {
    prefix: 'g',
    power: 10 ** 9,
  },
  {
    prefix: 'm',
    power: 10 ** 6,
  },
  {
    prefix: 'k',
    power: 10 ** 3,
  },
];

function ContentTooltip({ bwRemained, bwMaxValue, amounPower, countLink }) {
  let text = 'Empty battery. You have no power & energy so you cannot submit cyberlinks. ';

  if (bwMaxValue > 0) {
    text = `You have ${formatCurrency(
      amounPower,
      'W',
      2,
      PREFIXES
    )} and can immediately submit ${Math.floor(countLink)} cyberlinks. `;
  }

  return (
    <Pane zIndex={4} paddingX={10} paddingY={10} maxWidth={200}>
      <Pane marginBottom={12}>
        <Text color="#fff" size={400}>
          {text}
          <Link
            to={
              CHAIN_ID === Networks.BOSTROM
                ? routes.search.getLink('get BOOT')
                : routes.teleport.path
            }
          >
            Get {BASE_DENOM.toUpperCase()}
          </Link>
        </Text>
      </Pane>
    </Pane>
  );
}

function BandwidthBar({ tooltipPlacement }) {
  // bwRemained = 0,
  // bwMaxValue = 0,
  // countLink = 0,
  // amounPower,
  // ...props

  const [linkPrice] = useState(4);

  const queryClient = useQueryClient();

  const [countLink, setCountLink] = useState(0);
  const [priceLink, setPriceLink] = useState(0.25);

  const [amounPower, setAmounPower] = useState(0);

  const bandwidth = useSelector((state) => state.bandwidth.bandwidth);
  const { defaultAccount } = useSelector((state) => state.pocket);

  const dispatch = useDispatch();

  const bwRemained = bandwidth.remained;
  const bwMaxValue = bandwidth.maxValue;

  const bwPercent = bwMaxValue > 0 ? Math.floor((bwRemained / bwMaxValue) * 100) : 0;

  useEffect(() => {
    const getBandwidth = async () => {
      try {
        const { account } = defaultAccount;
        if (account !== null && Object.hasOwn(account, 'cyber') && queryClient) {
          const { bech32: cyberBech32 } = account.cyber;
          const responseAccountBandwidth = await queryClient.accountBandwidth(cyberBech32);

          if (responseAccountBandwidth?.neuronBandwidth) {
            const { maxValue, remainedValue } = responseAccountBandwidth.neuronBandwidth;
            dispatch(setBandwidth(remainedValue, maxValue));
            setCountLink(remainedValue / (priceLink * 1000));
          } else {
            dispatch(setBandwidth(0, 0));
            setCountLink(0);
          }
        } else {
          dispatch(setBandwidth(0, 0));
          setCountLink(0);
        }
      } catch (_error) {
        dispatch(setBandwidth(0, 0));
        setCountLink(0);
      }
    };
    getBandwidth();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [defaultAccount, priceLink, queryClient, dispatch]);

  useEffect(() => {
    const getPrice = async () => {
      if (queryClient) {
        const response = await queryClient.price();
        setPriceLink(coinDecimals(response.price.dec));
      }
    };
    getPrice();
  }, [queryClient]);

  useEffect(() => {
    const getAmounPower = async () => {
      try {
        const { account } = defaultAccount;
        if (account !== null && Object.hasOwn(account, 'cyber') && queryClient) {
          const { bech32 } = account.cyber;
          const allBalances = await queryClient.getAllBalances(bech32);
          const reduceallBalances = reduceBalances(allBalances);
          if (reduceallBalances.milliampere && reduceallBalances.millivolt) {
            const { milliampere, millivolt } = reduceallBalances;
            setAmounPower(convertResources(milliampere) * convertResources(millivolt));
          }
        } else {
          setAmounPower(0);
        }
      } catch (_error) {
        setAmounPower(0);
      }
    };
    getAmounPower();
  }, [queryClient, defaultAccount]);

  return (
    <Tooltip
      placement={tooltipPlacement || 'bottom'}
      // trigger="click"
      tooltip={
        <ContentTooltip
          bwRemained={bwRemained}
          bwMaxValue={bwMaxValue}
          linkPrice={linkPrice}
          countLink={countLink}
          amounPower={amounPower}
        />
      }
    >
      <Battery
        // {...props}
        height="10px"
        // // styleText={{ display: 'none' }}
        fontSize={12}
        maxWidth={75}
        colorText="#000"
        bwPercent={bwPercent}
        bwRemained={bwRemained}
        bwMaxValue={bwMaxValue}
        linkPrice={linkPrice}
      />
    </Tooltip>
  );
}

export default BandwidthBar;
