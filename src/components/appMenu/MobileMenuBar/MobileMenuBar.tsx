import cx from 'classnames';
import React, { useEffect, useRef, useState } from 'react';
import { NavLink, useNavigate } from 'react-router-dom';
import { Color } from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { Input } from 'src/components';
import { useActiveMenuItem } from 'src/hooks/useActiveMenuItem';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { getMobileMenuItems } from 'src/utils/appsMenu/appsMenu';
import { replaceSlash } from 'src/utils/utils';
import { setFocus, setValue } from 'src/containers/application/Header/Commander/commander.redux';
import useOnClickOutside from 'src/hooks/useOnClickOutside';
import styles from './MobileMenuBar.module.scss';

const fixedValue = '~';

const actions = [
  { icon: '🔍', label: 'Search', id: 'search' },
  { icon: '🔗', label: 'Cyberlink', id: 'cyberlink' },
] as const;

function MobileMenuBar() {
  const menuItems = getMobileMenuItems();
  const { isActiveItem } = useActiveMenuItem(menuItems);
  const navigate = useNavigate();
  const commander = useAppSelector((store) => store.commander);
  const dispatch = useAppDispatch();
  const inputRef = useRef<HTMLInputElement>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState(false);
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

  function handleAction(id: string) {
    if (!commander.value) return;
    const val = replaceSlash(commander.value);

    if (id === 'search') {
      navigate(routes.search.getLink(val));
    } else if (id === 'cyberlink') {
      navigate(`/studio?particle=${encodeURIComponent(val)}`);
    }

    dispatch(setFocus(false));
    dispatch(setValue(''));
    setExpanded(false);
    inputRef.current?.blur();
  }

  return (
    <>
      {expanded && <div className={styles.overlay} />}
      <div ref={barRef} className={cx(styles.bar, { [styles.expanded]: expanded })}>
        {expanded && (
          <div className={styles.expandedContent}>
            <div className={styles.actions}>
              {actions.map((a) => (
                <button
                  key={a.id}
                  type="button"
                  className={styles.actionBtn}
                  onClick={() => handleAction(a.id)}
                >
                  <span className={styles.actionIcon}>{a.icon}</span>
                  <span className={styles.actionLabel}>{a.label}</span>
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
              isTextarea
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
