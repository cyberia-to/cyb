import { ActionBar, Dots } from 'src/components';
import { useRelayer } from '../../../contexts/relayer';

function ActionBarRelayer({ network }: { network: string }) {
  const { isRelaying, selectChain, setSelectChain, stop } = useRelayer();

  if (isRelaying) {
    return (
      <ActionBar
        text={
          <div>
            Relayeing {selectChain} <Dots />
          </div>
        }
        button={{ text: 'stop', onClick: () => stop() }}
      />
    );
  }

  if (!isRelaying && !network) {
    return <ActionBar text="choose network to relayer" />;
  }

  if (network) {
    return (
      <ActionBar
        text={`Relay ${network}`}
        button={{ text: 'start', onClick: () => setSelectChain(network) }}
      />
    );
  }

  return null;
}

export default ActionBarRelayer;
