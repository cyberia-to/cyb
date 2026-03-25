import { routes } from 'src/routes';
import { Nullable, Option } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { MenuItem, MenuItems } from 'src/types/menu';
import findApp from 'src/utils/findApp';
import reduceRobotSubItems from './reduceRobotSubItems';

const avatarSubItems = [
  { name: 'Sigma', to: 'sigma', icon: '∑' },
  { name: 'Sense', to: 'sense', icon: '👁' },
  { name: 'Time', to: 'time', icon: '⏳' },
  { name: 'Brain', to: 'brain', icon: '🧠' },
];

const findSelectAppByUrl = (
  url: string,
  passport: Nullable<Citizenship>,
  address: Option<string>
) => {
  let pathname = url;
  const isAvatar = /^\/@/.test(url);
  const isRobot = !isAvatar && (url.includes('neuron/') || url.includes('robot'));
  const isOracle = url.includes('oracle');
  const isSenate = url.includes('senate');
  const isCyberver = url.includes('cyberver');
  const isSphere = url.includes('sphere');

  const itemsMenuObj = reduceRobotSubItems(passport, address);

  if (isAvatar) {
    const match = url.match(/^\/@([^/]+)/);
    const username = match ? match[1] : '';
    const avatarBase = `/@${username}`;

    const avatarCid = passport?.extension?.avatar;
    const avatarIcon = avatarCid
      ? `https://gateway.ipfs.cybernode.ai/ipfs/${avatarCid}`
      : itemsMenuObj.find((i) => i.to === '/settings')?.icon || '';

    const avatarMenuItem: MenuItem = {
      name: `@${username}.moon`,
      icon: avatarIcon,
      to: avatarBase,
      subItems: avatarSubItems.map((sub) => ({
        ...sub,
        to: `${avatarBase}/${sub.to}`,
      })),
    };

    return [avatarMenuItem];
  }

  if (isRobot) {
    pathname = '/settings';
  }

  if (isOracle) {
    pathname = routes.oracle.path;
  }

  if (isSenate) {
    pathname = routes.senate.path;
  }

  if (isSphere) {
    pathname = routes.sphere.path;
  }

  if (isCyberver) {
    pathname = '/cyberver';
  }

  const value = findApp(itemsMenuObj, pathname);

  return value;
};

export default findSelectAppByUrl;
