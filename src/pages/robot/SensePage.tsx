import Taverna from 'src/containers/taverna';
import SenseRoutingWrapper from 'src/features/sense/ui/SenseRoutingWrapper';
import { useAppSelector } from 'src/redux/hooks';
import { useRobotContext } from './robot.context';

function SensePage() {
  const keys = useAppSelector((store) => store.pocket.defaultAccount.account?.cyber.keys);

  if (!keys || keys === 'read-only') {
    return <Taverna />;
  }

  return <SenseRoutingWrapper />;
}

export default SensePage;
