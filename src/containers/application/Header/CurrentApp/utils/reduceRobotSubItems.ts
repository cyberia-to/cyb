import { CHAIN_ID } from 'src/constants/config';
import { routes } from 'src/routes';
import { Nullable, Option } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { MenuItem, MenuItems } from 'src/types/menu';
import { Networks } from 'src/types/networks';
import getMenuItems from 'src/utils/appsMenu/appsMenu';

const reduceRobotSubItems = (passport: Nullable<Citizenship>, address: Option<string>) => {
  const passportChain = CHAIN_ID === Networks.BOSTROM && passport;

  let linkApp: string;
  if (passportChain) {
    linkApp = routes.robotPassport.getLink(passport.extension.nickname);
  } else if (address) {
    linkApp = routes.neuron.getLink(address);
  }

  return getMenuItems().reduce((acc: MenuItems, item: MenuItem) => {
    if (item.to === routes.robot.path) {
      item.subItems = !linkApp
        ? []
        : item.subItems.map((item) => ({
            ...item,
            to: `${linkApp}/${item.to}`,
          }));
    }

    return [...acc, { ...item }];
  }, []);
};

export default reduceRobotSubItems;
