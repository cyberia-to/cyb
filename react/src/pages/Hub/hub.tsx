import { Route, Routes } from 'react-router-dom';
import Channels from './containers/Channels/Channels';
import Networks from './containers/Networks/Networks';
import Tokens from './containers/Tokens/Tokens';
import Layout from './Layout/Layout';

function Hub() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route path="tokens" element={<Tokens />} />
        <Route path="networks" element={<Networks />} />
        <Route path="channels" element={<Channels />} />
      </Route>
    </Routes>
  );
}

export default Hub;
