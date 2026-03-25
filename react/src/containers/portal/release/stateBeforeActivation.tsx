import { Link } from 'react-router-dom';
import { ContainerGradientText } from 'src/components';
import { ColorLamp } from 'src/components/containerGradient/ContainerGradient';
import { ProgressCard } from '../components';
import { BOOT_ICON } from '../utils';

function InfoBaner({ title, text, status }: { title: string; text: string; status?: ColorLamp }) {
  return (
    <ContainerGradientText status={status}>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: '20px',
          padding: '15px 0',
        }}
      >
        <div style={{ color: '#36D6AE', fontSize: '22px' }}>{title}</div>
        <div style={{ fontSize: '18px', color: '#fff', textAlign: 'center' }}>{text}</div>
      </div>
    </ContainerGradientText>
  );
}

function StateBeforeActivation({
  useBeforeActivation,
  progress,
}: {
  useBeforeActivation: number | string;
  progress: number;
}) {
  return (
    <>
      <ProgressCard
        titleValue={`${useBeforeActivation} addresses`}
        headerText="before activation"
        footerText="addresses registered"
        progress={progress}
      />
      <ContainerGradientText status="green">
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            gap: '20px',
            padding: '15px 0',
          }}
        >
          <div style={{ fontSize: '22px' }}>🚀 How to reach the target?</div>
        </div>
      </ContainerGradientText>
      <Link to="/genesis">
        <InfoBaner
          title="📼 Watch the Genesis story"
          text="An inspiring and comical 40 minutes diving into the birth of
              Superintelligence."
        />
      </Link>
      <Link to="/ipfs/QmbYDfhXr8y3DTqZggGumKHKDb6h7ChJr1bfSKZcY9eP96">
        <InfoBaner
          title="🗣 Share tweet"
          text="Spread the word about Superintelligence! The more you share, the
              quicker we reach our goal."
        />
      </Link>
      <Link to="/teleport">
        <InfoBaner
          title={`${BOOT_ICON} Buy BOOT`}
          text="Higher BOOT price = more attractive gift."
          status="pink"
        />
      </Link>
    </>
  );
}

export default StateBeforeActivation;
