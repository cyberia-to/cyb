import { Link } from 'react-router-dom';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { SubnetHyperParameters } from 'src/features/cybernet/types';
import { routes } from 'src/routes';
import styles from '../../Subnet.module.scss';
import { useCurrentSubnet } from '../../subnet.context';

const config: { [K in keyof SubnetHyperParameters]: { text: string } } = {
  rho: {
    text: 'Rho',
  },
  kappa: {
    text: 'Kappa',
  },
  immunity_period: {
    text: 'Immunity Period',
  },
  min_allowed_weights: {
    text: 'Min Allowed Weights',
  },
  max_weights_limit: {
    text: 'Max Weights Limit',
  },
  tempo: {
    text: 'Tempo',
  },
  min_difficulty: {
    text: 'Min Difficulty',
  },
  max_difficulty: {
    text: 'Max Difficulty',
  },
  weights_version: {
    text: 'Weights Version',
  },
  weights_rate_limit: {
    text: 'Weights Rate Limit',
  },
  adjustment_interval: {
    text: 'Adjustment Interval',
  },
  activity_cutoff: {
    text: 'Activity Cutoff',
  },
  registration_allowed: {
    text: 'Registration Allowed',
  },
  target_regs_per_interval: {
    text: 'Target Regs Per Interval',
  },
  min_burn: {
    text: 'Min Burn',
  },
  max_burn: {
    text: 'Max Burn',
  },
  bonds_moving_avg: {
    text: 'Bonds Moving Avg',
  },
  max_regs_per_block: {
    text: 'Max Regs Per Block',
  },
};

function SubnetHyperParams() {
  const { hyperparamsQuery } = useCurrentSubnet();

  useAdviserTexts({
    isLoading: hyperparamsQuery.loading,
    error: hyperparamsQuery.error,
    defaultText: 'Subnet hyperparams',
  });

  return (
    <Display noPaddingX title={<DisplayTitle title="Hyperparams" />}>
      <ul className={styles.list}>
        {hyperparamsQuery.data &&
          Object.keys(hyperparamsQuery.data).map((item) => {
            const value = hyperparamsQuery.data[item];
            let content = value;

            if (['min_burn', 'max_burn'].includes(item)) {
              content = <span>{value.toLocaleString()} 🟣</span>;
            }

            if (item === 'registration_allowed') {
              content = <span>{value === true ? 'yes' : 'no'}</span>;
            }

            const title = config[item].text || item;

            return (
              <li key={item}>
                <Link to={routes.oracle.ask.getLink(title.toLowerCase())}>{title}</Link>
                <div>{content}</div>
              </li>
            );
          })}
      </ul>
    </Display>
  );
}

export default SubnetHyperParams;
