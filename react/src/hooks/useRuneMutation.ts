import { useEffect, useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { useScripting } from 'src/contexts/scripting/scripting';
import { IPFSContentDetails, IPFSContentDetailsMutated } from 'src/services/ipfs/types';
import { postProcessIpfContent } from 'src/services/scripting/services/postProcessing';
import { ParticleCid } from 'src/types/base';

export type RuneState = 'loading' | 'processing' | 'done';

const useRuneMutation = (cid: ParticleCid, details?: IPFSContentDetails, skip = false) => {
  const { ipfsApi } = useBackend();
  const { rune, isSoulInitialized } = useScripting();
  const [mutatedDetails, setMutatedDetails] = useState<IPFSContentDetailsMutated | undefined>(
    details
  );
  const [state, setState] = useState<RuneState>(skip ? 'done' : 'loading');

  useEffect(() => {
    if (isSoulInitialized && details && state === 'loading') {
      setState('processing');

      postProcessIpfContent(cid, details, rune, ipfsApi).then((result) => {
        setMutatedDetails(result);
        setState('done');
      });
    }
  }, [isSoulInitialized, state, details, cid, rune, ipfsApi]);

  return { state, mutatedDetails };
};

export default useRuneMutation;
