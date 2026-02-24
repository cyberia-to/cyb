import { ActionBar, ContainerGradient, Input, LinkWindow } from '../../../components';
import BtnPasport from '../../../containers/portal/pasport/btnPasport';
import CodeSnipet from './ipfsComponents/codeSnipet';
import { ContainerKeyValue } from './ipfsComponents/utilsComponents';

const ipfsDaemon = `$ ipfs daemon
Initializing daemon...
API server listening on /ip4/127.0.0.1/tcp/5001
`;

const ipfsHeaders = `$ ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["*"]'

$ ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "POST", "GET"]'`;

function ErrorIpfsSettings({ stateErrorIpfsSettings }) {
  const {
    valueInput,
    setValueInput,
    onClickReConnect,
    setNewUrl,
  } = stateErrorIpfsSettings;

  return (
    <ContainerGradient
      togglingDisable
      styleLampContent="red"
      title="Could not connect to the IPFS API"
      userStyleContent={{
        minHeight: 'auto',
        height: 'unset',
        display: 'grid',
        gap: '20px',
      }}
    >
      <div>
        Check out the installation guide in the{' '}
        <LinkWindow to="https://docs.ipfs.tech/how-to/command-line-quick-start/">
          IPFS Docs
        </LinkWindow>{' '}
        , or try these common fixes:
      </div>
      <div>1. Is your IPFS daemon running? Try starting or restarting it from your terminal:</div>
      <CodeSnipet src={ipfsDaemon} />
      <div>
        2. Is your IPFS API configured to allow{' '}
        <LinkWindow to="https://github.com/ipfs/ipfs-webui#configure-ipfs-api-cors-headers">
          cross-origin (CORS)
        </LinkWindow>{' '}
        requests? If not, run these commands and then start your daemon from the terminal:
      </div>
      <CodeSnipet src={ipfsHeaders} />
      <div>
        3. Is your IPFS API on a port other than 5001? If your node is configured with a{' '}
        <LinkWindow to="https://github.com/ipfs/kubo/blob/master/docs/config.md#addresses">
          custom API address
        </LinkWindow>
        , enter it here.
      </div>
      <div
        style={{
          display: 'grid',
          gridTemplateColumns: '300px auto',
          gap: '10px',
        }}
      >
        <Input
          value={valueInput}
          onChange={(e) => setValueInput(e.target.value)}
        />
        <BtnPasport
          style={{ maxWidth: '100px', position: 'static' }}
          typeBtn="blue"
          onClick={() => setNewUrl()}
        >
          edit
        </BtnPasport>
      </div>

      <ActionBar
        button={{
          text: 'reconnect',
          onClick: onClickReConnect,
        }}
      />
    </ContainerGradient>
  );
}

export default ErrorIpfsSettings;
