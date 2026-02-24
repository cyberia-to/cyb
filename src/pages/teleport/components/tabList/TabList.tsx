import { useLocation } from 'react-router-dom';
import { Tabs } from 'src/components';
import { TypePages } from '../../type';

function TabListTeleport() {
  const location = useLocation();
  const locationSplit = location.pathname.replace(/^\/|\/$/g, '').split('/');
  const active = Object.values(TypePages).find((item) => item === locationSplit[1]);

  if (!active) {
    return null;
  }
  return (
    <Tabs selected={active} options={Object.keys(TypePages).map((key) => ({ to: key, key }))} />
  );
}

export default TabListTeleport;
