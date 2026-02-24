import { useLocation } from 'react-router-dom';
import { MenuItem } from 'src/types/menu';

// eslint-disable-next-line import/prefer-default-export
export const useActiveMenuItem = (menuItems: MenuItem[]) => {
  const location = useLocation();
  const isActiveItem = (item: MenuItem) => {
    if (location.pathname === item.to) {
      return true;
    }
    if (
      item.to === '/robot' &&
      (location.pathname.includes('@') || location.pathname.includes('neuron/'))
    ) {
      return true;
    }
    if (item.to === '/senate' && location.pathname.startsWith('/senate/')) {
      return true;
    }

    return item.subItems?.some((subItem) => location.pathname === subItem.to);
  };

  const activeItem = menuItems.find((item) => isActiveItem(item)) || null;

  return { isActiveItem, activeItem };
};
