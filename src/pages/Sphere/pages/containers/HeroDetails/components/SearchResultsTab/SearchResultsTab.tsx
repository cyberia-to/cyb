import SearchResults from 'src/containers/Search/SearchResults';
import useGetIPFSHash from 'src/features/ipfs/hooks/useGetIPFSHash';
import Loader2 from 'src/components/ui/Loader2';

function SearchResultsTab({ moniker }: { moniker: string }) {
  const hash = useGetIPFSHash(moniker);
  const noComments = (
    <>
      there are no particle for this hero <br /> be the first and create one
    </>
  );

  if (!hash) {
    return <Loader2 />;
  }

  return (
    <SearchResults
      query={hash}
      noCommentText={noComments}
      // actionBarTextBtn="Comment"
    />
  );
}

export default SearchResultsTab;
