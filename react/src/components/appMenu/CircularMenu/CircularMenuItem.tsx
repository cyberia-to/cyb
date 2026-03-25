import cx from 'classnames';
import { NavLink } from 'react-router-dom';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import { MenuItem } from 'src/types/menu';
import styles from './CircularMenuItem.module.scss';

interface Props {
  item: MenuItem;
  onClick: () => void;
  selected: boolean;
}

function CircularMenuItem({ item, onClick, selected }: Props) {
  const isExternal = item.to.startsWith('http');
  const isDisabled = 'disabled' in item && item.disabled;

  return (
    <div className={cx(styles.menuItem, { [styles.active]: selected, [styles.disabled]: isDisabled })}>
      <AdviserHoverWrapper adviserContent={isDisabled ? 'temporary closed for maintenance' : item.name}>
        {isDisabled ? (
          <span className={styles.navLink}>
            <img src={item.icon} className={styles.icon} alt="img" />
          </span>
        ) : (
          <NavLink
            to={item.to}
            onClick={onClick}
            className={styles.navLink}
            {...(isExternal && { target: '_blank', rel: 'noreferrer noopener' })}
          >
            <img src={item.icon} className={cx(styles.icon, { [styles.portalGlow]: item.name === 'Portal' })} alt="img" />
            {isExternal && <span className={styles.external}></span>}
          </NavLink>
        )}
      </AdviserHoverWrapper>
    </div>
  );
}

export default CircularMenuItem;
