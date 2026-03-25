import { Route, Routes } from 'react-router-dom';
import RelayerContextProvider from '../../contexts/relayer';
import Bridge from './bridge/bridge';
import Layout from './Layout/Layout';
import TeleportMainScreen from './mainScreen/TeleportMainScreen';
import Relayer from './relayer/Relayer';
import Send from './send/send';
import Swap from './swap/swap';
import TeleportContextProvider from './Teleport.context';

function TeleportRouter() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<TeleportMainScreen />} />
        <Route path="swap" element={<Swap />} />
        <Route path="send" element={<Send />} />
        <Route path="bridge" element={<Bridge />} />
        <Route path="relayer" element={<Relayer />} />
      </Route>
    </Routes>
  );
}

function Teleport() {
  return (
    <TeleportContextProvider>
      <RelayerContextProvider>
        <TeleportRouter />
      </RelayerContextProvider>
    </TeleportContextProvider>
  );
}

export default Teleport;
