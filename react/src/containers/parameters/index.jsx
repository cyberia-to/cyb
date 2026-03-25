import { Pane } from '@cybercongress/gravity';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useAdviser } from 'src/features/adviser/context';

import { Loading, MainContainer, Tabs } from '../../components';
import getParamNetwork from './getParams';
import {
  BandwidthParam,
  DistributionParam,
  DmnParam,
  GovParam,
  GridParam,
  InlfationParam,
  LiquidityParam,
  RankParam,
  ResourcesParam,
  SlashingParam,
  StakingParam,
  WasmParam,
} from './tabs';

const paramsTabs = {
  staking: { text: 'Staking', to: '/network/bostrom/parameters/staking' },
  slashing: { text: 'Slashing', to: '/network/bostrom/parameters/slashing' },
  distribution: {
    text: 'Distribution',
    to: '/network/bostrom/parameters/distribution',
  },
  bandwidth: { text: 'Bandwidth', to: '/network/bostrom/parameters' },
  gov: { text: 'Governance', to: '/network/bostrom/parameters/gov' },
  rank: { text: 'Rank', to: '/network/bostrom/parameters/rank' },
  mint: { text: 'Mint', to: '/network/bostrom/parameters/mint' },
  resources: { text: 'Resources', to: '/network/bostrom/parameters/resources' },
  liquidity: { text: 'Liquidity', to: '/network/bostrom/parameters/liquidity' },
  grid: { text: 'Grid', to: '/network/bostrom/parameters/grid' },
  wasm: { text: 'Wasm', to: '/network/bostrom/parameters/wasm' },
  dmn: { text: 'DMN', to: '/network/bostrom/parameters/dmn' },
};

const initParam = {
  staking: null,
  slashing: null,
  distribution: null,
  bandwidth: null,
  gov: null,
  rank: null,
  mint: null,
  resources: null,
  grid: null,
  wasm: null,
  liquidity: null,
  dmn: null,
};

function ParamNetwork() {
  const { setAdviser } = useAdviser();
  const [dataParam, setDataParam] = useState(initParam);
  const [loading, setLoading] = useState(true);
  const { param = 'bandwidth' } = useParams();

  useEffect(() => {
    (async () => {
      const response = await getParamNetwork();
      if (response !== null) {
        setDataParam(response);
      }
      setLoading(false);
    })();
  }, []);

  useEffect(() => {
    setAdviser(
      <span>
        Parametrs are adjastable by the consensus. <br /> Everybody can propose change the parametrs
        to different value.
      </span>
    );
  }, [setAdviser]);

  if (loading) {
    return (
      <div
        style={{
          width: '100%',
          height: '50vh',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          flexDirection: 'column',
        }}
      >
        <Loading />
      </div>
    );
  }

  return (
    <MainContainer>
      <Tabs
        selected={param}
        options={Object.entries(paramsTabs).map(([key, item]) => ({
          to: item.to,
          key,
        }))}
      />
      <Pane
        display="flex"
        marginTop={20}
        marginBottom={50}
        justifyContent="center"
        flexDirection="column"
      >
        {param === 'bandwidth' && <BandwidthParam data={dataParam.bandwidth} />}
        {param === 'slashing' && <SlashingParam data={dataParam.slashing} />}
        {param === 'staking' && <StakingParam data={dataParam.staking} />}
        {param === 'distribution' && <DistributionParam data={dataParam.distribution} />}
        {param === 'gov' && <GovParam data={dataParam.gov} />}
        {param === 'rank' && <RankParam data={dataParam.rank} />}
        {param === 'mint' && <InlfationParam data={dataParam.mint} />}
        {param === 'resources' && <ResourcesParam data={dataParam.resources} />}
        {param === 'liquidity' && <LiquidityParam data={dataParam.liquidity} />}
        {param === 'grid' && <GridParam data={dataParam.grid} />}
        {param === 'wasm' && <WasmParam />}
        {param === 'dmn' && <DmnParam data={dataParam.dmn} />}
      </Pane>
    </MainContainer>
  );
}

export default ParamNetwork;
