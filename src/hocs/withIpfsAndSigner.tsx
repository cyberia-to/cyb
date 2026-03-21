import React from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';

const withIpfsAndSigner = (Component: React.ComponentType) =>
  function WithIpfsAndKeplr(props: any) {
    const { ipfsApi, senseApi } = useBackend();
    const { signer, signingClient } = useSigningClient();
    const queryClient = useQueryClient();

    return (
      <Component
        {...props}
        ipfsApi={ipfsApi}
        signer={signer}
        signingClient={signingClient}
        senseApi={senseApi}
        queryClient={queryClient}
      />
    );
  };

export default withIpfsAndSigner;
