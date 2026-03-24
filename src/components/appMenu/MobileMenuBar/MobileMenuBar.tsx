import cx from 'classnames';
import React, { useEffect, useRef, useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { Color } from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { Input } from 'src/components';
import { useActiveMenuItem } from 'src/hooks/useActiveMenuItem';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import getMenuItems from 'src/utils/appsMenu/appsMenu';
import { replaceSlash } from 'src/utils/utils';
import { setFocus, setValue } from 'src/containers/application/Header/Commander/commander.redux';
import useOnClickOutside from 'src/hooks/useOnClickOutside';
import styles from './MobileMenuBar.module.scss';

const fixedValue = '~';

const presets = [
  { icon: '🔍', label: 'Oracle', prefix: '' },
  { icon: '🔗', label: 'Cyberlink', prefix: 'cyberlink:' },
  { icon: '📡', label: 'Send', prefix: 'send:' },
  { icon: '🧠', label: 'Brain', prefix: 'brain:' },
];

const networks = ['bostrom', 'osmosis', 'cosmos'];

function MobileMenuBar() {
  const menuItems = getMenuItems();
  const { isActiveItem } = useActiveMenuItem(menuItems);
  const navigate = useNavigate();
  const commander = useAppSelector((store) => store.commander);
  const dispatch = useAppDispatch();
  const inputRef = useRef<HTMLInputElement>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState(false);
  const [activeNetwork, setActiveNetwork] = useState(0);
  const mountedRef = useRef(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      mountedRef.current = true;
    }, 500);
    return () => clearTimeout(timer);
  }, []);

  useOnClickOutside(barRef, () => {
    if (expanded) {
      setExpanded(false);
      dispatch(setFocus(false));
      inputRef.current?.blur();
    }
  });

  function onChange(event: React.ChangeEvent<HTMLInputElement>) {
    dispatch(setValue(event.target.value.replace(fixedValue, '')));
  }

  function handleFocus() {
    if (!mountedRef.current) return;
    setExpanded(true);
    dispatch(setFocus(true));
  }

  function submit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!commander.value) return;
    navigate(routes.search.getLink(replaceSlash(commander.value)));
    dispatch(setFocus(false));
    setExpanded(false);
    inputRef.current?.blur();
  }

  function handlePreset(prefix: string) {
    dispatch(setValue(prefix));
    inputRef.current?.focus();
  }

  return (
    <>
      {expanded && <div className={styles.overlay} />}
      <div ref={barRef} className={cx(styles.bar, { [styles.expanded]: expanded })}>
        {expanded && (
          <div className={styles.expandedContent}>
            <div className={styles.presets}>
              {presets.map((p) => (
                <button
                  key={p.label}
                  type="button"
                  className={styles.presetBtn}
                  onClick={() => handlePreset(p.prefix)}
                >
                  <span className={styles.presetIcon}>{p.icon}</span>
                  <span className={styles.presetLabel}>{p.label}</span>
                </button>
              ))}
            </div>
            <div className={styles.networks}>
              {networks.map((net, i) => (
                <button
                  key={net}
                  type="button"
                  className={cx(styles.networkBtn, { [styles.networkActive]: i === activeNetwork })}
                  onClick={() => setActiveNetwork(i)}
                >
                  {net}
                </button>
              ))}
            </div>
          </div>
        )}
        <div className={styles.mainRow}>
          <div className={styles.icons}>
            {menuItems.map((item, index) => {
              const active = isActiveItem(item);
              const isDisabled = 'disabled' in item && item.disabled;
              if (isDisabled) {
                return (
                  <span
                    key={index}
                    className={cx(styles.menuItem, styles.disabled)}
                  >
                    <img
                      src={item.icon}
                      className={styles.icon}
                      alt={item.name}
                    />
                  </span>
                );
              }
              return (
                <NavLink
                  key={index}
                  to={item.to}
                  className={cx(styles.menuItem, { [styles.active]: active })}
                >
                  <img
                    src={item.icon}
                    className={cx(styles.icon, {
                      [styles.portalGlow]: item.name === 'Portal',
                    })}
                    alt={item.name}
                  />
                </NavLink>
              );
            })}
          </div>
          <form className={styles.commander} onSubmit={submit}>
            <Input
              ref={inputRef}
              color={Color.Pink}
              value={fixedValue + commander.value}
              focusedProps={expanded}
              onChange={onChange}
              onFocus={handleFocus}
              autoComplete="off"
              className={styles.input}
            />
          </form>
        </div>
      </div>
    </>
  );
}

export default MobileMenuBar;
