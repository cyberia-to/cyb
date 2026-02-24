import { Link } from 'react-router-dom';
import { CardStatisics } from '../../../components';
import { formatNumber } from '../../../utils/utils';
import AccountCount from '../accountCount';

function KnowledgeTab({ linksCount, cidsCount, accountsCount, inlfation }) {
  try {
    return (
      <>
        <CardStatisics title="Cyberlinks" value={formatNumber(linksCount)} />

        <Link to="/particles">
          <CardStatisics title="Particles" value={formatNumber(cidsCount)} link />
        </Link>
        <CardStatisics title="Neurons" value={<AccountCount />} />

        <CardStatisics title="Inflation" value={`${formatNumber(inlfation * 100, 2)} %`} />
      </>
    );
  } catch (error) {
    console.log(error);
    return <div>oops...</div>;
  }
}

export default KnowledgeTab;
