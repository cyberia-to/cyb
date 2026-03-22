import cx from 'classnames';
import { useCallback, useState } from 'react';
import { NavLink } from 'react-router-dom';
import { Display } from 'src/components';
import styles from './SettingsMenu.module.scss';

type MenuItem = {
  text: string;
  link: string;
  icon: string;
};

const links: Array<MenuItem[]> = [
  [
    {
      text: 'Drive',
      link: '.',
      icon: '🟥',
    },
  ],
  [
    {
      text: 'Keys',
      link: './keys',
      icon: '🗝',
    },
  ],
  process.env.IS_TAURI || !window.keplr
    ? [
        {
          text: 'Signer',
          link: './signer',
          icon: '🖋️',
        },
      ]
    : undefined,
  [
    {
      text: 'Tokens',
      link: './tokens',
      icon: '🟢',
    },
    {
      text: 'Networks',
      link: './networks',
      icon: '🌐',
    },
    {
      text: 'Channels',
      link: './channels',
      icon: '📡',
    },
  ],
  // [
  //   {
  //     text: 'Audio',
  //     link: './audio',
  //     icon: '🔊',
  //   },
  // ],
  [{ text: 'Hotkeys', link: './hotkeys', icon: '⌨️' }],
  [{ text: 'LLM', link: './llm', icon: '👾' }],
].filter(Boolean);

function SettingsMenu() {
  const [expanded, setExpanded] = useState(false);

  const handleNav = useCallback(() => {
    setExpanded(false);
  }, []);

  return (
    <div
      className={cx(styles.wrapper, { [styles.expanded]: expanded })}
      onClick={() => setExpanded((prev) => !prev)}
    >
      <Display>
        <div className={styles.links}>
          {links.map((link, indexUl) => (
            <ul key={indexUl}>
              {link.map((item, index) => (
                <li key={index}>
                  <NavLink
                    className={({ isActive }) =>
                      cx({
                        [styles.active]: isActive,
                      })
                    }
                    to={item.link}
                    end
                    onClick={handleNav}
                  >
                    <span className={styles.icon}>{item.icon}</span>
                    <span className={styles.text}>{item.text}</span>
                  </NavLink>
                </li>
              ))}
            </ul>
          ))}
        </div>
      </Display>
    </div>
  );
}

export default SettingsMenu;
