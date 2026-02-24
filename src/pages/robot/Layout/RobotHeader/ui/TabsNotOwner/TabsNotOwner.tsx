import { useLocation } from 'react-router-dom';
import { Tabs } from 'src/components';
import styles from './TabsNotOwner.module.scss';
import PillUsers from './ui/PillUsers';

const tabsPages = {
  sense: {
    text: 'sense',
    link: './sense',
    icon: '🧬',
    description: 'unread',
    name: 'sense',
  },
  brain: {
    text: 'brain',
    link: './brain',
    icon: '🧠',
    description: 'cyberlinks',
    name: 'cyberlinks',
  },
  robot: {
    text: 'robot',
    link: './',
    icon: '',
    description: 'robot',
    name: 'log',
  },
  time: {
    text: 'time',
    link: './time',
    name: 'txs',
    description: 'txs',
    icon: '⏱',
  },
  sigma: {
    text: '',
    link: './sigma',
    description: 'hydrogen',
    name: 'sigma',
    icon: 'Σ',
  },
};

function TabsNotOwner({ menuCounts }: { menuCounts: typeof tabsPages }) {
  const location = useLocation();
  const locationSplit = location.pathname.replace(/^\/|\/$/g, '').split('/');
  const neuronSplit = locationSplit[0] === 'neuron' ? locationSplit.slice(1) : locationSplit;

  const active = Object.keys(tabsPages).find((item) => {
    return item === neuronSplit[1];
  });

  return (
    <Tabs
      selected={active || 'robot'}
      options={Object.entries(tabsPages).map(([key, item]) => ({
        to: item.link,
        key,
        text: (
          <div className={styles.wrapper}>
            <span className={styles.icon}>{item.icon}</span>
            <span>{item.text}</span>
            <span>
              <PillUsers name={item.name} value={menuCounts[item.name] || 0} />
            </span>
          </div>
        ),
      }))}
    />
  );
}

export default TabsNotOwner;
