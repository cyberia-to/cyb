import { Outlet } from 'react-router-dom';
import { Helmet } from 'react-helmet';
import { MainContainer } from 'src/components';

function SphereLayout() {
  return (
    <MainContainer>
      <Helmet>
        <title>sphere | cyb</title>
      </Helmet>

      <Outlet />
    </MainContainer>
  );
}

export default SphereLayout;
