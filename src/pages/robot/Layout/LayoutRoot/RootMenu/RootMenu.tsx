import cx from 'classnames';
import { NavLink } from 'react-router-dom';
import Display from 'src/components/containerGradient/Display/Display';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import styles from './RootMenu.module.scss';

type MenuItem = {
  text: string;
  link: string;
  description?: string;
  name?: string;
  icon: string;
  isDisabled?: boolean;
};

const links: MenuItem[] = [
  {
    text: 'Log',
    link: '.',
    name: 'log',
    description: 'tweets',
    icon: '🍀',
  },

  {
    text: 'Energy',
    link: './energy',
    name: 'energy',
    description: 'watt',
    icon: '🚀',
  },
  {
    text: 'Swarm',
    link: './swarm',
    description: 'learners',
    name: 'swarm',
    icon: '💚',
  },
  {
    text: 'Security',
    description: 'reward',
    link: './security',
    name: 'rewards',
    icon: '🧑🏼‍🚀',
  },
  {
    text: 'Badges',
    link: './badges',
    name: 'badges',
    description: 'tokens',
    icon: '🥇',
  },
  {
    text: 'Karma',
    link: './karma',
    name: 'karma',
    description: '',
    icon: '🔮',
  },
  {
    text: 'Soul',
    link: './soul',
    isDisabled: true,
    // description: 'bytes',
    icon: '👻',
  },
];

type Props = {
  counts: any;
};

function RootMenu({ counts }: Props) {
  const renderLinks = (links: Array<MenuItem>) => {
    return (
      <ul className={styles.links}>
        {links.map((link, index) => {
          const count = counts[link.name] || 0;

          return (
            <li key={index}>
              <NavLink
                className={({ isActive }) => {
                  return cx({
                    [styles.active]: isActive,
                  });
                }}
                to={link.link}
                end
              >
                <div>
                  <span className={styles.icon}>{link.icon}</span>
                  <span className={styles.text}>{link.text}</span>
                </div>

                <div>
                  <span className={styles.count}>
                    {['karma', 'energy', 'rewards'].includes(link.name) ? (
                      <IconsNumber
                        value={counts[link.name]}
                        type={(() => {
                          switch (link.name) {
                            case 'rewards':
                              return 'boot';

                            default:
                              return link.name;
                          }
                        })()}
                      />
                    ) : (
                      count
                    )}
                  </span>
                  <span className={styles.description}>{link.description}</span>
                </div>
              </NavLink>
            </li>
          );
        })}
      </ul>
    );
  };

  return (
    <div className={styles.wrapper}>
      <Display noPadding>{renderLinks(links)}</Display>
    </div>
  );
}

export default RootMenu;
