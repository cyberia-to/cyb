import { ContainerGradient } from '../../../components';

const linkMovie =
  'https://gateway.ipfs.cybernode.ai/ipfs/QmSWJNCBxj4m5Lpg1XGueh38NbEVDLAGsQrueD937xSnMC';

function ConnectKeplr() {
  return (
    <ContainerGradient userStyleContent={{ height: '350px' }} title="How to connect Keplr">
      <div
        style={{
          width: '100%',
          background: 'transparent',
          position: 'relative',
          height: '100%',
        }}
      >
        {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
        <video width="100%" height="100%" controls>
          <source src={linkMovie} type="video/mp4" />
        </video>
      </div>
    </ContainerGradient>
  );
}

// eslint-disable-next-line import/no-unused-modules
export default ConnectKeplr;
