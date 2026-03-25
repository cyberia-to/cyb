import { useBackend } from 'src/contexts/backend/backend';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';

const useIpfsAndKeplr = () => {
  const { ipfsApi, senseApi } = useBackend();
  const { signer, signingClient } = useSigningClient();
  // FIXME: refactor to use useCyberClient
  const queryClient = useQueryClient();

  return {
    ipfsApi,
    signer,
    signingClient,
    senseApi,
    queryClient,
  };
};

export default useIpfsAndKeplr;
