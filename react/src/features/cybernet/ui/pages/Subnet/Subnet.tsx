import { Route, Routes, useParams } from 'react-router-dom';
import { Tabs } from 'src/components';
import Loader2 from 'src/components/ui/Loader2';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useCybernetTexts from '../../useCybernetTexts';
import ActionBar from './SubnetActionBar/SubnetActionBar';
import SubnetHeader from './SubnetHeader/SubnetHeader';
import SubnetProvider, { useCurrentSubnet } from './subnet.context';
import SubnetInfo from './tabs/SubnetInfo/SubnetInfo';
import SubnetNeurons from './tabs/SubnetNeurons/SubnetNeurons';
import SubnetSubnets from './tabs/SubnetSubnets/SubnetSubnets';
import Weights from './tabs/Weights/Weights';

function Subnet() {
  const { ...rest } = useParams();
  const tab = rest['*'];

  // const {selectedContract} = useCybernet();

  const { subnetQuery, neuronsQuery, subnetRegistrationQuery, isRootSubnet } = useCurrentSubnet();

  const { getText } = useCybernetTexts();

  useAdviserTexts({
    isLoading: subnetQuery.loading,
    loadingText: `loading ${getText('subnetwork')}`,
    error: subnetQuery.error || neuronsQuery.error,
    // defaultText: 'subnet',
  });

  const addressRegisteredInSubnet = subnetRegistrationQuery.data !== null;

  const tabs = [
    {
      to: './info',
      key: 'info',
      text: 'info',
    },
    {
      to: './',
      key: 'delegates',
      text: getText(isRootSubnet ? 'rootValidator' : 'delegate', true),
    },

    {
      to: './grades',
      key: 'grades',
      text: 'grades',
      disabled: isRootSubnet,
    },
  ];

  if (isRootSubnet) {
    tabs.push({
      to: './faculties',
      key: 'faculties',
      text: getText('subnetwork', true),
    });
  }

  return (
    <>
      <SubnetHeader />

      <Tabs options={tabs} selected={tab || 'delegates'} />

      <Routes>
        <Route
          path="/"
          element={<SubnetNeurons addressRegisteredInSubnet={addressRegisteredInSubnet} />}
        />

        <Route path="/info" element={<SubnetInfo />} />

        {subnetQuery.data?.subnetwork_n > 0 && <Route path="/grades" element={<Weights />} />}

        <Route
          path="/faculties"
          element={<SubnetSubnets addressRegisteredInSubnet={addressRegisteredInSubnet} />}
        />
      </Routes>

      {subnetQuery.loading && <Loader2 />}

      <ActionBar />
    </>
  );
}

export default function SubnetWithProvider() {
  return (
    <SubnetProvider>
      <Subnet />
    </SubnetProvider>
  );
}
