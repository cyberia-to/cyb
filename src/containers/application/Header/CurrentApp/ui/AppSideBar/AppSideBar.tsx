import cx from 'classnames';
import { useRef } from 'react';
import { useAdviser } from 'src/features/adviser/context';
import useMediaQuery from 'src/hooks/useMediaQuery';
import useOnClickOutside from 'src/hooks/useOnClickOutside';
import { menuButtonId } from '../../utils/const';
import BurgerIcon from '../BurgerIcon/BurgerIcon';
import styles from './AppSideBar.module.scss';

interface Props {
  children: React.ReactNode;
  menuProps: {
    isOpen: boolean;
    toggleMenu: () => void;
    closeMenu: () => void;
  };
}

function findElementInParents(element: HTMLElement, targetSelector: string) {
  let { parentElement } = element;

  while (parentElement) {
    if (parentElement.matches(targetSelector)) {
      return parentElement;
    }
    parentElement = parentElement.parentElement;
  }
  return null;
}

function AppSideBar({ children, menuProps }: Props) {
  const mediaQuery = useMediaQuery('(min-width: 768px)');
  const { isOpen, closeMenu, toggleMenu } = menuProps;
  const ref = useRef<HTMLElement>(null);

  const { setIsOpen } = useAdviser();

  useOnClickOutside(ref, (e) => {
    if (mediaQuery) {
      return;
    }

    const buttonMenu = findElementInParents(e.target, `#${menuButtonId}`);

    if (buttonMenu) {
      return;
    }

    closeMenu();
  });

  return (
    <aside
      ref={ref}
      className={cx(styles.sideBar, {
        [styles.sideBarHide]: !isOpen,
      })}
    >
      {!mediaQuery && (
        <button
          className={styles.toggleBtn}
          onClick={() => {
            toggleMenu();
            setIsOpen(false);
          }}
          type="button"
        >
          <BurgerIcon openMenu={isOpen} />
        </button>
      )}
      {children}
    </aside>
  );
}

export default AppSideBar;
