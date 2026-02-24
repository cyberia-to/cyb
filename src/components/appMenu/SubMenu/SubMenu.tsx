import cx from 'classnames';
import { useMemo } from 'react';
import { NavLink } from 'react-router-dom';
import { MenuItem } from 'src/types/menu';
import { checkIsEmoji } from 'src/utils/emoji';
import styles from './SubMenu.module.scss';

interface Props {
  selectedApp: MenuItem;
  closeMenu: () => void;
}

function SubMenu({ selectedApp, closeMenu }: Props) {
  const renderData = useMemo(
    () =>
      selectedApp.subItems.length
        ? [
            {
              name: 'main',
              to: selectedApp.to,
              icon: selectedApp.icon,
            },
            ...selectedApp.subItems,
          ]
        : [],
    [selectedApp]
  );

  return (
    <div className={styles.subMenu}>
      {renderData.map((item) => (
        <NavLink
          key={item.name}
          to={item.to}
          end
          className={({ isActive }) =>
            cx(styles.navLinkItem, {
              [styles.active]: isActive,
            })
          }
          onClick={closeMenu}
        >
          {/* // clean */}
          {item.icon &&
            (checkIsEmoji(item.icon) ? (
              <span>{item.icon}</span>
            ) : (
              <img src={item.icon} className={styles.icon} alt={`${item.name} icon`} />
            ))}
          <span className={styles.nameApp}>{item.name}</span>
        </NavLink>
      ))}
    </div>
  );
}

export default SubMenu;
