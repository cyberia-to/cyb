import cx from 'classnames';
import React from 'react';
import { NavLink } from 'react-router-dom';
import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import { useRobotContext } from '../../robot.context';
import styles from './RobotMenu.module.scss';

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
    text: 'Sigma',
    link: './sigma',
    description: 'hydrogen',
    name: 'sigma',
    icon: 'Σ',
  },
  {
    text: 'Timeline',
    link: './timeline',
    name: 'txs',
    description: 'txs',
    icon: '🚥',
  },
  {
    text: 'Chat',
    link: './chat',
    isDisabled: true,
    icon: '💬',
    description: 'msg',
  },
  {
    text: 'Badges',
    link: './badges',
    name: 'badges',
    description: 'tokens',
    icon: '🥇',
  },
  {
    text: 'Items',
    link: './items',
    isDisabled: true,
    icon: '🖼',
    description: 'tokens',
  },
  {
    text: 'Security',
    description: 'reward',
    link: './security',
    name: 'rewards',
    icon: '🧑🏼‍🚀',
  },
  {
    text: 'Skills',
    isDisabled: true,
    link: './skills',
    description: 'active',
    icon: '🍄',
  },
  {
    text: 'Rights',
    link: './rights',
    isDisabled: true,
    icon: '🙌',
  },

  // second menu

  {
    text: 'Energy',
    link: './energy',
    name: 'energy',
    description: 'watt',
    icon: '🚀',
  },
  {
    text: 'Drive',
    name: 'drive',
    description: '',
    link: './drive',
    icon: '🟥',
  },
  {
    text: 'Swarm',
    link: './swarm',
    description: 'learners',
    name: 'swarm',
    icon: '💚',
  },

  {
    text: 'Sense',
    link: './sense',
    icon: '🧬',
    description: 'unread',
    name: 'sense',
  },
  {
    text: 'Brain',
    link: './brain',
    icon: '🧠',
    description: 'cyberlinks',
    name: 'cyberlinks',
  },
  {
    text: 'Log',
    link: '.',
    name: 'log',
    description: 'tweets',
    icon: '🍀',
  },
  {
    text: 'Karma',
    link: './karma',
    name: 'karma',
    description: 'points',
    icon: '🔮',
  },
  {
    text: 'Soul',
    link: './soul',
    // isDisabled: true,
    // description: 'bytes',
    icon: '👻',
  },
];

type Props = {
  counts: any;
  isRight?: boolean;
};

const splitIndex = 8;

function RobotMenu({ counts, isRight }: Props) {
  const { address, isLoading } = useRobotContext();

  let linksToRender = [];

  if (!isRight) {
    linksToRender = links.slice(0, splitIndex);
  } else {
    linksToRender = links.slice(splitIndex, links.length);
  }

  function renderLinks(links: MenuItem[], isMirror?: boolean) {
    return (
      <ul
        className={cx(styles.links, {
          [styles.mirror]: isMirror,
        })}
      >
        {links.map((link, index) => {
          let { description, text, icon } = link;
          let count = counts[link.name] || 0;

          if (link.name === 'drive' && counts.drive) {
            const [value, measurement] = counts.drive.split(' ');
            description = measurement;
            count = value;
          }
          const newUser = !isLoading && !address;

          if (newUser && link.name === 'sigma') {
            description = '';
            count = '-';
            icon = '🤖';
            text = 'Robot';
          }

          function selectTag(content: React.ReactNode) {
            if ((newUser && !['sigma', 'drive'].includes(link.name)) || link.isDisabled) {
              return (
                <button
                  type="button"
                  title={link.isDisabled ? 'Page is under construction' : undefined}
                  className={styles.disabled}
                >
                  {content}
                </button>
              );
            }

            return (
              <NavLink
                className={({ isActive }) => {
                  return cx({
                    [styles.active]: isActive,
                  });
                }}
                to={link.link}
                end
              >
                {content}
              </NavLink>
            );
          }

          return (
            <li key={index}>
              {selectTag(
                <>
                  <span className={styles.text}>{text}</span>
                  <span className={styles.count}>
                    {['karma', 'sigma', 'energy', 'rewards'].includes(link.name) ? (
                      <IconsNumber
                        value={counts[link.name]}
                        type={(() => {
                          switch (link.name) {
                            case 'sigma':
                              return 'hydrogen';

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

                  {/* <span className={styles.new}>+123</span> */}

                  <span className={styles.icon}>{icon}</span>
                  <span className={styles.description}>{description}</span>
                </>
              )}
            </li>
          );
        })}
      </ul>
    );
  }

  return renderLinks(linksToRender, isRight);
}

export default RobotMenu;
