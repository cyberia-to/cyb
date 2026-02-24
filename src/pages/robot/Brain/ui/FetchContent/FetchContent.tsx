import { Cid, Dots } from 'src/components';
import ContentIpfs from 'src/components/contentIpfs/contentIpfs';
import useParticleDetails from 'src/features/particle/useParticleDetails';

function FetchContent({ cid, parentId }: { cid: string; parentId: string }) {
  const { data, loading } = useParticleDetails(
    cid!,
    {
      skip: Boolean(!cid),
    },
    parentId
  );

  let content;

  if (loading) {
    content = (
      <span>
        resolving particle <Dots />
      </span>
    );
  } else {
    content = <ContentIpfs details={data} cid={cid} content={data?.content} search />;
  }

  return <Cid cid={cid}>{content}</Cid>;
}

export default FetchContent;
