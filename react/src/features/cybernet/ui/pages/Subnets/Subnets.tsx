import { Helmet } from 'react-helmet';
import { Loading } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useCybernet } from '../../cybernet.context';
import useCybernetTexts from '../../useCybernetTexts';
import styles from './Subnets.module.scss';
import SubnetsTable from './SubnetsTable/SubnetsTable';

function Subnets() {
  const {
    subnetsQuery: { data, loading, error },
  } = useCybernet();

  // possible to refactor to 1 loop
  const rootSubnet = data?.find((subnet) => subnet.netuid === 0);
  const graphSubnets = data?.filter((subnet) => subnet.network_modality === 0);

  const { getText } = useCybernetTexts();

  useAdviserTexts({
    isLoading: loading,
    error,
    defaultText: 'explore the full list of faculties',
  });
  return (
    <>
      <Helmet>
        <title>{getText('subnetwork', true)} | cyb</title>
      </Helmet>
      {loading && <Loading />}

      {rootSubnet && (
        <Display
          noPadding
          title={
            <DisplayTitle
              title={
                <header className={styles.header}>
                  {/* <AdviserHoverWrapper adviserContent=""> */}
                  {getText('root')}
                  {/* </AdviserHoverWrapper> */}
                </header>
              }
            />
          }
        >
          <SubnetsTable data={[rootSubnet] || []} />
        </Display>
      )}

      {!!graphSubnets?.length && (
        <Display
          noPadding
          title={
            <DisplayTitle
              title={
                <header className={styles.header}>
                  {/* <AdviserHoverWrapper adviserContent=""> */}
                  {getText('subnetwork', true)}
                  {/* </AdviserHoverWrapper> */}
                </header>
              }
            />
          }
        >
          <SubnetsTable data={graphSubnets || []} />
        </Display>
      )}
    </>
  );
}

export default Subnets;
