import HydrogenBalance from 'src/components/HydrogenBalance/HydrogenBalance';
import SideButtonLink from 'src/components/sideButtonLink/SideButtonLink';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import { routes } from 'src/routes';

function SideHydrogenBtn() {
  const currentAddress = useCurrentAddress();
  return (
    <SideButtonLink to={routes.robot.routes.sigma.path} buttonType="hydrogen">
      <HydrogenBalance address={currentAddress} />
    </SideButtonLink>
  );
}

export default SideHydrogenBtn;
