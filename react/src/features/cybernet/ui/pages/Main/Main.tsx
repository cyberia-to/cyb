import { Link, Navigate, useParams } from 'react-router-dom';
import { LinkWindow } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import { Stars } from 'src/containers/portal/components';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { useCybernet } from '../../cybernet.context';
import useCurrentAccountStake from '../../hooks/useCurrentAccountStake';
import useDelegate from '../../hooks/useDelegate';
import { cybernetRoutes } from '../../routes';
import useCybernetTexts from '../../useCybernetTexts';
import Banner from './Banner/Banner';
import ContractsTable from './ContractsTable/ContractsTable';
import styles from './Main.module.scss';

function Main() {
  const address = useCurrentAddress();

  const { getText } = useCybernetTexts();

  const { data } = useDelegate(address);
  const currentAddressIsDelegator = !!data;

  const { data: currentStake } = useCurrentAccountStake();
  const haveStake = currentStake?.some(({ stake }) => stake > 0);

  const { selectedContract, contracts } = useCybernet();

  const { nameOrAddress } = useParams();

  if (!nameOrAddress && contracts.length) {
    return (
      <Navigate
        to={cybernetRoutes.verse.getLink(
          'pussy',
          selectedContract?.metadata?.name || contracts[0].address
        )}
      />
    );
  }

  const {
    metadata: { name } = {},
    address: contractAddress,
    network = 'pussy',
  } = selectedContract || {};

  const contractNameOrAddress = name || contractAddress;

  const { staker_apr, validator_apr } = selectedContract?.economy || {};

  return (
    <>
      <Stars />
      <Banner />

      <div className={styles.verses}>
        <Display noPaddingX noPaddingY title={<DisplayTitle title="choose verse" />}>
          <ContractsTable />
        </Display>
      </div>

      <div className={styles.actions}>
        <div className={styles.bgWrapper}>
          <Display
            title={
              <DisplayTitle
                title={
                  <div className={styles.actionTitle}>
                    <h3>stake</h3>

                    {staker_apr && (
                      <div className={styles.apr}>
                        yield up to <br />
                        <span>{Number(selectedContract?.economy?.staker_apr).toFixed(2)}%</span>
                      </div>
                    )}
                  </div>
                }
              />
            }
          >
            <p className={styles.actionText}>learn by staking on {getText('delegate', true)}</p>
            <div className={styles.links}>
              <Link to={cybernetRoutes.delegators.getLink('pussy', contractNameOrAddress)}>
                {getText('delegate', true)}
              </Link>

              <button disabled type="button" className={styles.delegatorsBtn}>
                {getText('delegator', true)}
              </button>

              {haveStake && (
                <Link to={cybernetRoutes.myLearner.getLink('pussy', contractNameOrAddress)}>
                  my {getText('delegator')}
                </Link>
              )}
            </div>
          </Display>
        </div>

        <div className={styles.bgWrapper}>
          <Display
            title={
              <DisplayTitle
                title={
                  <div className={styles.actionTitle}>
                    <h3>mine</h3>
                    {validator_apr && (
                      <div className={styles.apr}>
                        yield up to
                        <span>{Number(selectedContract?.economy?.validator_apr).toFixed(2)}%</span>
                      </div>
                    )}
                  </div>
                }
              />
            }
          >
            <p className={styles.actionText}>teach by linking content</p>

            <div className={styles.links}>
              <Link to={cybernetRoutes.subnet.getLink('pussy', contractNameOrAddress, 0)}>
                {getText('root')}
              </Link>

              <Link to={cybernetRoutes.subnets.getLink(network, contractNameOrAddress)}>
                {getText('subnetwork', true)}
              </Link>

              {currentAddressIsDelegator && (
                <Link
                  to={cybernetRoutes.delegator.getLink(network, contractNameOrAddress, address)}
                >
                  my {getText('delegate')}
                </Link>
              )}
            </div>
          </Display>
        </div>

        <div className={styles.bgWrapper}>
          <Display
            title={
              <DisplayTitle
                title={
                  <div className={styles.actionTitle}>
                    <h3>deploy</h3>
                  </div>
                }
              />
            }
          >
            <div className={styles.links}>
              <LinkWindow to="https://github.com/cybercongress/cybertensor">
                cli and python package
              </LinkWindow>

              <LinkWindow to="https://github.com/cybercongress/cybertensor-subnet-template">
                subnet template
              </LinkWindow>

              <LinkWindow to="https://github.com/cybercongress/cybernet">
                cosmwasm contract
              </LinkWindow>

              <LinkWindow to="https://docs.spacepussy.ai">docs</LinkWindow>
            </div>
          </Display>
        </div>
      </div>
    </>
  );
}

export default Main;
