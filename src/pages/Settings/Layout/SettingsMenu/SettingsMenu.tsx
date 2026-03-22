import cx from 'classnames';
import { useCallback, useEffect, useRef, useState } from 'react';
import { NavLink, useLocation } from 'react-router-dom';
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
  const wrapperRef = useRef<HTMLDivElement>(null);
  const location = useLocation();

  // collapse on route change (covers browser back/forward)
  useEffect(() => {
    setExpanded(false);
  }, [location.pathname]);

  // collapse on any click outside the menu
  useEffect(() => {
    if (!expanded) return;

    const handleOutside = (e: MouseEvent) => {
      if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
        setExpanded(false);
      }
    };

    document.addEventListener('click', handleOutside);
    return () => document.removeEventListener('click', handleOutside);
  }, [expanded]);

  const handleToggle = useCallback(() => {
    setExpanded((prev) => !prev);
  }, []);

  const handleItemClick = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    setExpanded(false);
  }, []);

  return (
    <div
      ref={wrapperRef}
      className={cx(styles.wrapper, { [styles.expanded]: expanded })}
      onClick={handleToggle}
    >
      <Display>
        <div className={styles.links}>
          {links.map((link, indexUl) => (
            <ul key={indexUl}>
              {link.map((item, index) => (
                <li key={index} onClick={handleItemClick}>
                  <NavLink
                    className={({ isActive }) =>
                      cx({
                        [styles.active]: isActive,
                      })
                    }
                    to={item.link}
                    end
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
