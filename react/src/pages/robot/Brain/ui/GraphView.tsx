import CyberlinksGraphContainer from 'src/features/cyberlinks/CyberlinksGraph/CyberlinksGraphContainer';
import useGraphLimit from '../useGraphLimit';

function GraphView({ address }: { address?: string }) {
  const { limit } = useGraphLimit(500);

  return (
    <div>
      <p
        style={{
          textAlign: 'center',
          zIndex: 10,
          position: 'relative',
          marginBottom: 20,
        }}
      >
        Limit is: {limit.toLocaleString()}
      </p>

      <CyberlinksGraphContainer address={address} limit={limit} toPortal type="3d" />
    </div>
  );
}

export default GraphView;
