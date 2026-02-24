import { Outlet } from 'react-router-dom';
import { useCybernet } from '../../cybernet.context';

function Verse() {
  const { selectedContract } = useCybernet();
  return (
    <>
      {/* <div
        style={{
          maxWidth: '62%',
        }}
      >
        <Display>
          <p>{JSON.stringify(selectedContract?.metadata)}</p>
          <AvataImgIpfs cidAvatar={selectedContract?.metadata?.logo} />
        </Display>
      </div> */}
      <Outlet />
    </>
  );
}

export default Verse;
