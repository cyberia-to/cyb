import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { useBackend } from 'src/contexts/backend/backend';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import { encodeSlash } from '../../utils/utils';
import SearchResults from '../Search/SearchResults';

function ProposalsDetailTableComments() {
  const { proposalId } = useParams();
  const { ipfsApi } = useBackend();
  const proposalQuery = `bostrom proposal ${proposalId}`;
  const [processedQuery, setProcessedQuery] = useState('');
  const noComments = (
    <>
      there are no comments to this particle <br /> be the first and create one
    </>
  );
  useEffect(() => {
    const processQuery = async () => {
      const hash = await getIpfsHash(encodeSlash(proposalQuery));
      if (hash) {
        setProcessedQuery(hash);
      } else {
        const newHash = await ipfsApi.addContent(proposalQuery);
        setProcessedQuery(newHash);
      }
    };

    processQuery();
  }, [proposalQuery, ipfsApi]);

  return (
    <div style={{ marginTop: '20px' }}>
      <SearchResults query={processedQuery} noCommentText={noComments} actionBarTextBtn="Comment" />
    </div>
  );
}

export default ProposalsDetailTableComments;
