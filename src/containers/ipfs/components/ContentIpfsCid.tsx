import TextMarkdown from 'src/components/TextMarkdown';
import { QueueItemStatus } from 'src/services/QueueManager/QueueManager';
import ComponentLoader from '../../../features/ipfs/ipfsSettings/ipfsComponents/ipfsLoader';

type Props = {
  loading?: boolean;
  statusFetching?: string;
  status?: QueueItemStatus;
  cid: string;
};

// TODO: refactor this component
function ContentIpfsCid({ loading, statusFetching, status, cid }: Props) {
  // const loading = dataGetIpfsContent.loading;

  if (loading) {
    return (
      <div
        style={{
          width: '100%',
          // height: '50vh',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          flexDirection: 'column',
          marginBottom: '50px',
        }}
      >
        <ComponentLoader style={{ width: '100px', margin: '30px auto' }} />
        {statusFetching && <div style={{ fontSize: '20px' }}>{statusFetching}</div>}
      </div>
    );
  }

  if (!loading && ['error', 'timeout', 'not_found'].includes(status)) {
    return (
      <div
        style={{
          width: '100%',
          // height: '50vh',
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          flexDirection: 'column',
          marginBottom: '50px',
        }}
      >
        <TextMarkdown>{cid}</TextMarkdown>
        {/* <div style={{ fontSize: '20px' }}>IPFS content is not available</div> */}
      </div>
    );
  }

  return null;
}

export default ContentIpfsCid;
