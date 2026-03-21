import { Route, Routes } from 'react-router-dom';
import IpfsSettings from 'src/features/ipfs/ipfsSettings';
import Hub from '../Hub/hub';
import Keys from '../Keys/Keys';
import Hotkeys from '../robot/Hotkeys/Hotkeys';
import Audio from './Audio/Audio';
import Layout from './Layout/Layout';
import LLM from './LLM/LLM';
import Signer from './Signer/Signer';

function Settings() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<IpfsSettings />} />
        <Route path="keys" element={<Keys />} />
        <Route path="signer" element={<Signer />} />
        <Route path="audio" element={<Audio />} />
        <Route path="llm" element={<LLM />} />

        <Route path="hotkeys" element={<Hotkeys />} />

        <Route path="/*" element={<Hub />} />
      </Route>
    </Routes>
  );
}

export default Settings;
