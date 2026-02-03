import { Route, Routes } from 'react-router-dom';
import HistoryContextProvider from 'src/features/ibc-history/historyContext';
import EnergyMain from './ui/pages/Main/Main';
import OsmosisRpcProvider from './context/OsmosisRpcProvider';
import EnergyProvider from './context/Energy.context';
import OsmosisSignerProvider from './context/OsmosisSignerProvider';
import Layout from './ui/Layout/Layout';

function EnergyRouter() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<EnergyMain />} />
      </Route>
    </Routes>
  );
}

function Energy() {
  return (
    <OsmosisRpcProvider>
      <OsmosisSignerProvider>
        <HistoryContextProvider>
          <EnergyProvider>
            <EnergyRouter />
          </EnergyProvider>
        </HistoryContextProvider>
      </OsmosisSignerProvider>
    </OsmosisRpcProvider>
  );
}

export default Energy;
