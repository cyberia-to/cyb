import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { timeSince, trimString } from '../../../utils/utils';
import Account from '../../account/account';
import { ContainerGradientText } from '../../containerGradient/ContainerGradient';

function Titile({ cid, creator, status }) {
  const timeAgo = useMemo(() => {
    if (creator.timestamp.length > 0) {
      const { timestamp } = creator;
      const d = new Date().toUTCString();
      const time = Date.parse(d) - Date.parse(timestamp);

      if (time >= 0) {
        return time;
      }
    }
    return null;
  }, [creator]);

  return (
    <ContainerGradientText status="pink">
      <div
        style={{
          display: 'flex',
          justifyContent: 'space-between',
          alignItems: 'center',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 30 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <div
              style={{
                width: '30px',
                height: '30px',
                borderRadius: '50%',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              kw
            </div>{' '}
            <Link to={`/ipfs/${cid}`}>{trimString(cid, 3, 3)}</Link>
          </div>
          {creator.address.length > 0 && (
            <>
              <span style={{ color: '#777777' }}>by</span>
              <Account avatar address={creator.address} />
            </>
          )}
        </div>
        {timeAgo !== null && <span style={{ color: '#777777' }}>{timeSince(timeAgo)}</span>}
      </div>
    </ContainerGradientText>
  );
}

export default Titile;
