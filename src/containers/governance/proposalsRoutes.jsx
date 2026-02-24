import { ProposalStatus } from 'cosmjs-types/cosmos/gov/v1beta1/gov';
import { Navigate, Route, Routes } from 'react-router-dom';
import ProposalsDetailTableComments from './ProposalsDetailTableComments';
import ProposalsIdDetailTableVoters from './proposalsDetailTableVoters';
import ProposalsIdDetail from './proposalsIdDetail';
import styles from './styles.module.scss';
import Layout from './tabsLayout';

function ProposalsRoutes({ proposals, tallying, tally, totalDeposit, updateFunc }) {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<Navigate replace to="comments" />} />
        <Route path="comments" element={<ProposalsDetailTableComments />} />
        <Route
          path="meta"
          element={
            <div className={styles.meta}>
              <ProposalsIdDetail
                proposals={proposals}
                tallying={tallying}
                tally={tally}
                totalDeposit={totalDeposit}
              />
            </div>
          }
        />
        <Route
          path="voters"
          element={
            proposals.status > ProposalStatus.PROPOSAL_STATUS_DEPOSIT_PERIOD ? (
              <ProposalsIdDetailTableVoters updateFunc={updateFunc} />
            ) : null
          }
        />
      </Route>
    </Routes>
  );
}

export default ProposalsRoutes;
