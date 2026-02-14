import { Nullable, Option } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { routes } from 'src/routes';
import findApp from 'src/utils/findApp';
import reduceRobotSubItems from './reduceRobotSubItems';

const findSelectAppByUrl = (
  url: string,
  passport: Nullable<Citizenship>,
  address: Option<string>
) => {
  let pathname = url;
  const isRobot =
    url.includes('@') || url.includes('neuron/') || url.includes('robot');
  const isOracle = url.includes('oracle');
  const isSenate = url.includes('senate');
  const isCyberver = url.includes('cyberver');
  const isSphere = url.includes('sphere');

  const itemsMenuObj = reduceRobotSubItems(passport, address);

  if (isRobot) {
    pathname = routes.robot.path;
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
