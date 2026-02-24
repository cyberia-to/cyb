import { Pane } from '@cybercongress/gravity';
import { Link } from 'react-router-dom';
import { routes } from 'src/routes';
import { CardStatisics } from '../../components';
import ForceGraph from '../../features/cyberlinks/CyberlinksGraph/CyberlinksGraph';
import { formatNumber } from '../../utils/utils';
import AccountCount from '../brain/accountCount';
import useGetStatisticsCyber from './useGetStatisticsCyber';

function Oracle() {
  const { knowledge } = useGetStatisticsCyber();

  const { linksCount, cidsCount } = knowledge;
  return (
    <>
      <main
        style={{
          position: 'absolute',
          left: '50%',
          zIndex: 2,
          backgroundColor: 'transparent',
          transform: 'translate(-50%, 0%)',
          marginRight: '-50%',
        }}
        className="block-body"
      >
        <Pane
          marginTop={10}
          marginBottom={50}
          display="grid"
          gridTemplateColumns="repeat(auto-fit, minmax(210px, 1fr))"
          gridGap="20px"
        >
          <Link to="/graph">
            <CardStatisics
              title="Cyberlinks"
              value={formatNumber(linksCount)}
              styleContainer={{ minWidth: 'unset' }}
            />
          </Link>
          <Link to="/particles">
            <CardStatisics
              title="Particles"
              value={formatNumber(cidsCount)}
              styleContainer={{ minWidth: 'unset' }}
            />
          </Link>
          <Link to={routes.search.getLink('neurons')}>
            <CardStatisics value={<AccountCount />} title="Neurons" />
          </Link>
        </Pane>
      </main>
      <ForceGraph />
    </>
  );
}

export default Oracle;
