import { ContainerGradient } from '../../../components';
import { MusicalAddress, ParseAddressesImg } from '../components';
import ContainerAvatar from '../components/avataIpfs/containerAvatar';

function Passport({ txs, valueNickname, avatar, addressActive }) {
  return (
    <ContainerGradient txs={txs} title="Moon Citizenship">
      <div
        style={{
          height: '50px',
          display: 'grid',
          gridTemplateColumns: '1fr 1fr',
        }}
      >
        <div style={{ color: '#36D6AE' }}>{valueNickname}</div>

        <ContainerAvatar>{avatar}</ContainerAvatar>
      </div>
      <div
        style={{
          height: 'calc(100% - 50px)',
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'flex-end',
          gridGap: '20px',
        }}
      >
        {addressActive !== null && (
          <div style={{ display: 'flex', gridGap: '15px' }}>
            <ParseAddressesImg
              key={addressActive.bech32}
              address={{ address: addressActive.bech32 }}
              active
            />
          </div>
        )}
        <MusicalAddress address={addressActive.bech32} />
      </div>
    </ContainerGradient>
  );
}

export default Passport;
