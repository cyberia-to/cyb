import { Cid } from 'src/components';
import useParticleDetails from 'src/features/particle/useParticleDetails';
import { trimString } from 'src/utils/utils';

function FetchContentFrom({ cid, parentId }: { cid: string; parentId: string }) {
  const { data, loading } = useParticleDetails(
    cid!,
    {
      skip: Boolean(!cid),
    },
    parentId
  );

  let content = trimString(cid);

  if (!loading && data?.type === 'text' && data.content && data.content?.length <= 100) {
    content = data.content;
  }

  return <Cid cid={cid}>{content}</Cid>;
}

export default FetchContentFrom;
