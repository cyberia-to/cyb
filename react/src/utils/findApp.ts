import { MenuItem, MenuItems } from 'src/types/menu';

const findSubItemFc = (subItems: MenuItem['subItems'], url: string) =>
  subItems.filter((item) => url.includes(item.to));

const findApp = (menuItems: MenuItems, url: string) => {
  // const isRobot = url.includes('@') || url.includes('neuron/');

  const findApp = menuItems.reduce((acc: MenuItems, item: MenuItem) => {
    if (item.to === url) {
      acc.push(item);
    } else if (findSubItemFc(item.subItems, url).length !== 0) {
      // const findSubItem = findSubItemFc(item.subItems, url);
      acc.push(item);
    }
    return acc;
  }, []);

  return findApp;
};

export default findApp;
