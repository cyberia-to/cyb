import { Route, Routes } from 'react-router-dom';
import SphereContextProvider from './Sphere.context';
import SphereLayout from './Layout/SphereLayout';
import Heroes from './pages/containers/Heroes/Heroes';
import HeroDetails from './pages/containers/HeroDetails/HeroDetails';

function SphereRoutes() {
  return (
    <Routes>
      <Route path="/" element={<SphereLayout />}>
        <Route index element={<Heroes />} />
        <Route path="hero/:address/" element={<HeroDetails />} />
        <Route path="hero/:address/:tab" element={<HeroDetails />} />
      </Route>
    </Routes>
  );
}

function Sphere() {
  return (
    // <ChainProvider>
    <SphereContextProvider>
      <SphereRoutes />
    </SphereContextProvider>
    // </ChainProvider>
  );
}

export default Sphere;
