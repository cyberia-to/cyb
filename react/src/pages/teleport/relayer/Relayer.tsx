import { useMemo, useState } from 'react';
import { DenomArr, MainContainer, OptionSelect, Select } from 'src/components';
import { SelectOption } from 'src/components/Select';
import { useRelayer } from '../../../contexts/relayer';
import { TeleportContainer } from '../components/containers/Containers';
import ActionBarRelayer from './ActionBar';
import LogRelayer from './components/Relayer/LogRelayer';

function Relayer() {
  const { channels, isRelaying, relayerLog, selectChain } = useRelayer();
  const [network, setNetwork] = useState('');

  const reduceOptions = useMemo(() => {
    const tempList: SelectOption[] = [];

    if (channels) {
      Object.keys(channels).forEach((key) => {
        const item = channels[key];
        tempList.push({
          value: key,
          text: (
            <DenomArr
              type="network"
              denomValue={item.destination_chain_id}
              onlyText
              tooltipStatusText={false}
            />
          ),
          img: (
            <DenomArr
              type="network"
              denomValue={item.destination_channel_id}
              onlyImg
              tooltipStatusImg={false}
            />
          ),
        });
      });
    }

    return tempList;
  }, [channels]);

  return (
    <>
      <MainContainer width="62%">
        <TeleportContainer>
          <Select
            disabled={isRelaying}
            valueSelect={isRelaying ? selectChain : network}
            currentValue={
              <OptionSelect
                text="choose"
                img={<DenomArr denomValue="choose" onlyImg />}
                value=""
                bgrImg
              />
            }
            onChangeSelect={(item: string) => setNetwork(item)}
            width="100%"
            options={reduceOptions}
            title="choose network to relayer"
          />
        </TeleportContainer>
        <TeleportContainer>
          <LogRelayer relayerLog={relayerLog} />
        </TeleportContainer>
      </MainContainer>
      <ActionBarRelayer network={network} />
    </>
  );
}

export default Relayer;
