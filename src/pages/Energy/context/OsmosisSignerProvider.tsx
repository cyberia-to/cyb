import { SigningStargateClient } from '@cosmjs/stargate';
import React, { useContext, useEffect, useMemo, useState } from 'react';
import { getKeplr } from 'src/utils/keplrUtils';
import { OfflineSigner } from '@cosmjs/proto-signing';
import { getSigningOsmosisClient } from 'osmojs';
import { Option } from 'src/types';
import { useAppSelector } from 'src/redux/hooks';
import { CHAIN_ID_OSMO, RPC_OSMO } from '../constants';

const OsmosisSignerContext = React.createContext<{
  address?: string;
  signingClient?: SigningStargateClient;
}>({
  address: undefined,
  signingClient: undefined,
});

export function useOsmosisSign() {
  const signingClient = useContext(OsmosisSignerContext);
  return signingClient;
}

function OsmosisSignerProvider({ children }: { children: React.ReactNode }) {
  const { defaultAccount } = useAppSelector((state) => state.pocket);
  const [address, setAddress] = useState<Option<string>>(undefined);
  const [signingClient, setSigningClient] =
    useState<Option<SigningStargateClient>>(undefined);

  async function initSigner() {
    const windowKeplr = await getKeplr();

    if (!windowKeplr) {
      return;
    }

    await windowKeplr.enable(CHAIN_ID_OSMO);

    const offlineSigner: OfflineSigner = await windowKeplr.getOfflineSignerAuto(
      CHAIN_ID_OSMO
    );

    const [{ address }] = await offlineSigner.getAccounts();

    setAddress(address);

    const stargateClient = await getSigningOsmosisClient({
      rpcEndpoint: RPC_OSMO,
      signer: offlineSigner,
    });

    setSigningClient(stargateClient);
  }

  useEffect(() => {
    (async () => {
      const windowKeplr = await getKeplr();
      if (windowKeplr) {
        initSigner();
      }
    })();
  }, [defaultAccount]);

  const value = useMemo(
    () => ({
      address,
      signingClient,
    }),
    [address, signingClient]
  );

  return (
    <OsmosisSignerContext.Provider value={value}>
      {children}
    </OsmosisSignerContext.Provider>
  );
}

export default OsmosisSignerProvider;
