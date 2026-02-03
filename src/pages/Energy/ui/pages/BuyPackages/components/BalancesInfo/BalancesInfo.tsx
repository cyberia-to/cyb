import { Coin } from '@cosmjs/stargate';
import { useMemo, useState } from 'react';
import { AvailableAmount, Color, DenomArr, Select } from 'src/components';
import { SelectOption } from 'src/components/Select';
import { CHAIN_ID_OSMO, sellTokensSymbol } from 'src/pages/Energy/constants';
import {
  getOsmoAssetByDenom,
  newShiftedMinus,
  symbolToOsmoDenom,
} from 'src/pages/Energy/utils/utils';
import {
  Col,
  GridContainer,
} from 'src/pages/teleport/components/containers/Containers';
import { useAppSelector } from 'src/redux/hooks';
import styles from './BalancesInfo.module.scss';

const listTokens = [sellTokensSymbol[0]];

const imgDenom = (src: string) => (
  <img
    src={src}
    alt="demon"
    style={{
      width: 20,
      height: 20,
      verticalAlign: 'middle',
    }}
  />
);

const reduceOptions = () => {
  const tempList: SelectOption[] = [];

  listTokens.forEach((key) => {
    const denomInfo = getOsmoAssetByDenom(symbolToOsmoDenom(key) || '');
    if (!denomInfo) {
      return;
    }
    const { symbol } = denomInfo;

    // console.log('denomInfo', denomInfo);
    // if (selected !== key) {
    tempList.push({
      value: key,
      text: (
        <DenomArr
          denomValue={symbol.toLowerCase()}
          onlyText
          tooltipStatusText={false}
        />
      ),
      img: imgDenom(denomInfo.logo_URIs?.svg),
    });
    // }
  });

  return tempList;
};

function BalancesInfo({ balancesTokenSell }: { balancesTokenSell?: Coin }) {
  const { selectPlan } = useAppSelector((state) => state.energy);
  const [valueSelect, setValueSelect] = useState(listTokens[0]);

  const balance = useMemo(() => {
    if (!balancesTokenSell) {
      return 0;
    }
    return newShiftedMinus(balancesTokenSell).amount;
  }, [balancesTokenSell]);

  const tokenIn = useMemo(() => {
    if (!selectPlan) {
      return 0;
    }
    return newShiftedMinus(selectPlan.tokenIn).amount;
  }, [selectPlan]);

  return (
    <GridContainer>
      <Col>
        <AvailableAmount
          color={Color.Blue}
          amountToken={tokenIn}
          title="package price"
        />
        <AvailableAmount amountToken={balance} />
      </Col>
      <Col>
        <Select
          valueSelect={valueSelect}
          currentValue={valueSelect}
          onChangeSelect={(item: string) => setValueSelect(item)}
          width="130px"
          options={reduceOptions()}
          title="choose token"
        />
        <Select
          valueSelect={CHAIN_ID_OSMO}
          currentValue={CHAIN_ID_OSMO}
          disabled
          width="130px"
          options={[
            {
              value: CHAIN_ID_OSMO,
              text: <span>{CHAIN_ID_OSMO}</span>,
              img: (
                <DenomArr
                  denomValue="osmo"
                  onlyImg
                  type="network"
                  tooltipStatusImg={false}
                />
              ),
            },
          ]}
          title="network"
        />
      </Col>
    </GridContainer>
  );
}

export default BalancesInfo;
