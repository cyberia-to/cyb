import { Pane } from '@cybercongress/gravity';
import { useCallback, useEffect, useState } from 'react';
import { Button, Display, DisplayTitle, Input } from 'src/components';

import Select from 'src/containers/warp/components/Select';
import { useBackend } from 'src/contexts/backend/backend';
import { AdviserColors } from 'src/features/adviser/Adviser/Adviser';
import { useAdviser } from 'src/features/adviser/context';
import { getIpfsOpts } from 'src/services/ipfs/config';
import { IPFSNodes } from 'src/services/ipfs/types';
import { invoke } from '@tauri-apps/api/core';
import BtnPassport from '../../../containers/portal/pasport/btnPasport';
import Drive from '../Drive';
import ErrorIpfsSettings from './ErrorIpfsSettings';
import InfoIpfsNode from './ipfsComponents/infoIpfsNode';
import ComponentLoader from './ipfsComponents/ipfsLoader';
import {
  ContainerKeyValue,
  renderOptions,
  updateIpfsStateType,
  updateIpfsStateUrl,
  updateUserGatewayUrl,
} from './ipfsComponents/utilsComponents';

const dataOpts = [IPFSNodes.EXTERNAL, IPFSNodes.EMBEDDED, IPFSNodes.HELIA];

function IpfsSettings() {
  const [valueSelect, setValueSelect] = useState(IPFSNodes.HELIA);
  const [valueInput, setValueInput] = useState('');
  const [valueInputGateway, setValueInputGateway] = useState('');
  const { isIpfsInitialized, ipfsError: failed, ipfsApi } = useBackend();

  useEffect(() => {
    const lsTypeIpfs = localStorage.getItem('ipfsState');
    if (lsTypeIpfs !== null) {
      const lsTypeIpfsData = JSON.parse(lsTypeIpfs);
      const { ipfsNodeType, urlOpts, userGateway } = lsTypeIpfsData;
      setValueSelect(ipfsNodeType);
      setValueInput(urlOpts);
      if (userGateway) {
        setValueInputGateway(userGateway);
      }
    }
  }, []);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    let text;
    let status: AdviserColors;
    if (!isIpfsInitialized) {
      text = 'trying to connect to ipfs...';
      status = 'yellow';
    } else {
      text = (
        <>
          manage and store neurones public data drive <br />
          drive storing data forever before the 6th great extinction
        </>
      );
    }

    setAdviser(text, status);
  }, [setAdviser, isIpfsInitialized]);

  const onChangeSelect = (item) => {
    setValueSelect(item);
    updateIpfsStateType(item);
  };

  const setNewUrl = useCallback(() => {
    updateIpfsStateUrl(valueInput);
  }, [valueInput]);

  const setNewUrlGateway = useCallback(() => {
    updateUserGatewayUrl(valueInputGateway);
  }, [valueInputGateway]);

  const onClickReConnect = async () => {
    if (process.env.IS_TAURI) {
      try {
        console.log('Restarting IPFS');

        await invoke('stop_ipfs');
        await invoke('start_ipfs');

        console.log('IPFS restarted');
      } catch (error) {
        console.error('Error restarting IPFS', error);
      }
    }

    await ipfsApi?.stop().catch(console.error);
    await ipfsApi?.start(getIpfsOpts()).catch(console.error);
  };

  const stateProps = {
    valueInput,
    valueSelect,
    setValueInput,
    dataOpts,
    onClickReConnect,
    onChangeSelect,
    pending: !isIpfsInitialized,
    setNewUrl,
  };

  if (failed) {
    return <ErrorIpfsSettings stateErrorIpfsSettings={stateProps} />;
  }

  return (
    <Display title={<DisplayTitle title="drive" />}>
      <div style={{ display: 'grid', gap: '20px' }}>
        <Drive />
        <div style={{ display: 'flex', justifyContent: 'space-between' }}>
          <div>
            <ContainerKeyValue>
              <div>client</div>

              <Select
                width="300px"
                valueSelect={valueSelect}
                textSelectValue={valueSelect !== '' ? valueSelect : ''}
                onChangeSelect={(item) => onChangeSelect(item)}
                custom
                disabled={!isIpfsInitialized}
              >
                {renderOptions(dataOpts)}
              </Select>
            </ContainerKeyValue>

            {valueSelect === 'external' && (
              <>
                <ContainerKeyValue>
                  <div>api</div>

                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: '280px 50px',
                      gap: '20px',
                      position: 'relative',
                    }}
                  >
                    <Input
                      value={valueInput}
                      onChange={(e) => setValueInput(e.target.value)}
                    />
                    <BtnPassport
                      style={{ maxWidth: '100px' }}
                      typeBtn="blue"
                      onClick={() => setNewUrl()}
                    >
                      edit
                    </BtnPassport>
                  </div>
                </ContainerKeyValue>
                <ContainerKeyValue>
                  <div>gateway</div>

                  <div
                    style={{
                      display: 'grid',
                      gridTemplateColumns: '280px 50px',
                      gap: '20px',
                      position: 'relative',
                    }}
                  >
                    <Input
                      value={valueInputGateway}
                      onChange={(e) => setValueInputGateway(e.target.value)}
                    />
                    <BtnPassport
                      style={{ maxWidth: '100px' }}
                      typeBtn="blue"
                      onClick={() => setNewUrlGateway()}
                    >
                      edit
                    </BtnPassport>
                  </div>
                </ContainerKeyValue>
              </>
            )}
          </div>
          <div>
            <InfoIpfsNode />
          </div>
        </div>

        {!isIpfsInitialized && (
          <ComponentLoader
            style={{ margin: '20px auto 10px', width: '100px' }}
          />
        )}
        <Pane
          width="100%"
          display="flex"
          marginBottom={20}
          padding={10}
          justifyContent="center"
          alignItems="center"
          flexDirection="column"
        >
          <Button onClick={onClickReConnect}>Reconnect</Button>
        </Pane>
        {/* <ActionBar>
          <Button onClick={onClickReConnect}>Reconnect</Button>
          <Button onClick={console.log}>Sync drive</Button>
        </ActionBar> */}
      </div>
    </Display>
  );
}

export default IpfsSettings;
