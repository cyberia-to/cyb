import _ from 'lodash';
import { useActiveMenuItem } from 'src/hooks/useActiveMenuItem';
import getMenuItems from 'src/utils/appsMenu/appsMenu';
import styles from './CircularMenu.module.scss';
import CircularMenuItem from './CircularMenuItem';

declare module 'react' {
  interface CSSProperties {
    '--diameter'?: string;
    '--delta'?: string;
    '--i'?: string;
  }
}

function CircularMenu({ circleSize }) {
  const chunkSize = 7;
  const linkChunks = _.chunk(getMenuItems(), chunkSize);

  const { isActiveItem, activeItem } = useActiveMenuItem(getMenuItems());

  const calculateDiameter = (index, circleSize) => {
    const menuCircleDiameter = circleSize / 2 + 45 * (index + 1) - 10;
    const nextLevelMenuAngle = index === 1 ? -28 : 0; // decreases the angle of second menu layer
    const menuItemsAngle = index === 1 ? 16 : 23; // angle in which menu items spread around the brain
    return { menuCircleDiameter, nextLevelMenuAngle, menuItemsAngle };
  };

  return (
    <div>
      {linkChunks.map((chunk, index) => {
        const { menuCircleDiameter, nextLevelMenuAngle, menuItemsAngle } = calculateDiameter(
          index,
          circleSize
        );
        return (
          <div
            key={index}
            className={styles.circle}
            style={{ width: circleSize, height: circleSize }}
          >
            <div
              className={styles.menu}
              style={{
                '--diameter': `${menuCircleDiameter}px`,
                '--delta': `${nextLevelMenuAngle}deg`,
                '--i': `${menuItemsAngle}deg`,
              }}
            >
              {chunk.map((item, index) => {
                const isSelected = activeItem?.name === item.name;
                return (
                  <li key={index}>
                    <CircularMenuItem
                      item={item}
                      onClick={() => isActiveItem(item)}
                      selected={isSelected}
                    />
                  </li>
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}

export default CircularMenu;
