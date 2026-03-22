import { useCallback, useEffect, useState } from 'react';
import { Button, Display, DisplayTitle, Input } from 'src/components';
import { useBackend } from 'src/contexts/backend/backend';
import { getIpfsOpts } from 'src/services/ipfs/config';
import { IPFSNodes, IpfsNodeType } from 'src/services/ipfs/types';
import InfoIpfsNode from 'src/features/ipfs/ipfsSettings/ipfsComponents/infoIpfsNode';
import {
  updateIpfsStateType,
  updateIpfsStateUrl,
  updateUserGatewayUrl,
} from 'src/features/ipfs/ipfsSettings/ipfsComponents/utilsComponents';

import styles from './Node.module.scss';

const backends: { type: IpfsNodeType; label: string; description: string }[] = [
  {
    type: 'external',
    label: 'External (Kubo)',
    description: 'Connect to a remote IPFS node via API',
  },
  {
    type: 'helia',
    label: 'Embedded (Helia)',
    description: 'Run a lightweight IPFS node in the browser',
  },
];

function Node() {
  const [nodeType, setNodeType] = useState<IpfsNodeType>('external');
  const [apiUrl, setApiUrl] = useState('');
  const [gatewayUrl, setGatewayUrl] = useState('');
  const { isIpfsInitialized, ipfsError, ipfsApi } = useBackend();

  useEffect(() => {
    const opts = getIpfsOpts();
    setNodeType(opts.ipfsNodeType);
    setApiUrl(opts.urlOpts);
    setGatewayUrl(opts.userGateway);
  }, []);

  const onSelectBackend = useCallback(
    (type: IpfsNodeType) => {
      setNodeType(type);
      updateIpfsStateType(type);
    },
    []
  );

  const onSaveApi = useCallback(() => {
    updateIpfsStateUrl(apiUrl);
  }, [apiUrl]);

  const onSaveGateway = useCallback(() => {
    updateUserGatewayUrl(gatewayUrl);
  }, [gatewayUrl]);

  const onReconnect = async () => {
    await ipfsApi?.stop().catch(console.error);
    await ipfsApi?.start(getIpfsOpts()).catch(console.error);
  };

  const statusColor = ipfsError
    ? '#ff4444'
    : isIpfsInitialized
    ? '#36d6ae'
    : '#ffaa00';

  const statusText = ipfsError
    ? 'error'
    : isIpfsInitialized
    ? 'connected'
    : 'connecting...';

  return (
    <Display title={<DisplayTitle title="node" />}>
      <div className={styles.container}>
        <div className={styles.status}>
          <span
            className={styles.dot}
            style={{ backgroundColor: statusColor }}
          />
          <span style={{ color: statusColor }}>{statusText}</span>
          {ipfsError && (
            <span className={styles.errorMsg}>
              {ipfsError instanceof Error ? ipfsError.message : String(ipfsError)}
            </span>
          )}
        </div>

        <div className={styles.section}>
          <div className={styles.sectionTitle}>backend</div>
          <div className={styles.backends}>
            {backends.map((b) => (
              <button
                key={b.type}
                type="button"
                className={`${styles.backendCard} ${
                  nodeType === b.type ? styles.active : ''
                }`}
                onClick={() => onSelectBackend(b.type)}
              >
                <div className={styles.backendLabel}>{b.label}</div>
                <div className={styles.backendDesc}>{b.description}</div>
              </button>
            ))}
          </div>
        </div>

        {nodeType === 'external' && (
          <div className={styles.section}>
            <div className={styles.sectionTitle}>configuration</div>
            <div className={styles.field}>
              <label>API URL</label>
              <div className={styles.inputRow}>
                <Input
                  value={apiUrl}
                  onChange={(e) => setApiUrl(e.target.value)}
                />
                <Button onClick={onSaveApi}>save</Button>
              </div>
            </div>
            <div className={styles.field}>
              <label>Gateway URL</label>
              <div className={styles.inputRow}>
                <Input
                  value={gatewayUrl}
                  onChange={(e) => setGatewayUrl(e.target.value)}
                />
                <Button onClick={onSaveGateway}>save</Button>
              </div>
            </div>
          </div>
        )}

        <div className={styles.section}>
          <InfoIpfsNode />
        </div>

        <div className={styles.actions}>
          <Button onClick={onReconnect}>Reconnect</Button>
        </div>
      </div>
    </Display>
  );
}

export default Node;
