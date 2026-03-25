import { useEffect, useState } from 'react';
import { useTracesNetworks } from '../../hooks/useTracesNetworks';
import Tooltip from '../tooltip/tooltip';

function TextNetwork({ network, tooltipStatus }) {
  const { chainInfo } = useTracesNetworks(network);
  const [textNetwork, setTextNetwork] = useState(network);
  const [tooltipText, setTooltipText] = useState(network);

  useEffect(() => {
    if (Object.hasOwn(chainInfo, 'chainName')) {
      const { chainName } = chainInfo;
      setTextNetwork(chainName);
      setTooltipText(chainName);
    } else {
      setTextNetwork(network.toUpperCase());
    }
  }, [chainInfo, network]);

  if (tooltipStatus) {
    return (
      <div>
        <Tooltip placement="top" tooltip={<div>{tooltipText}</div>}>
          <span>{textNetwork}</span>
        </Tooltip>
      </div>
    );
  }

  return <span>{textNetwork}</span>;
}

export default TextNetwork;
