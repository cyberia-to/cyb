import { useEffect } from 'react';
import { Link } from 'react-router-dom';
import Display from 'src/components/containerGradient/Display/Display';
import BannerHelp from 'src/containers/help/BannerHelp';
import { useAdviser } from 'src/features/adviser/context';
import { routes } from 'src/routes';

function ZeroUser() {
  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <div>
        Connect your wallet by adding a <Link to={routes.keys.path}>key</Link> to start using robot
      </div>
    );

    return () => {
      setAdviser(null);
    };
  }, [setAdviser]);

  return (
    <Display>
      <BannerHelp />
    </Display>
  );
}

export default ZeroUser;
