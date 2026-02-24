// TODO: refactor needed
import { useEffect } from 'react';
import { Link } from 'react-router-dom';
import { LinksType } from 'src/containers/Search/types';
import useParticle from 'src/hooks/useParticle';
import { routes } from 'src/routes';
import type { IpfsContentType } from 'src/services/ipfs/types';
import { $TsFixMe } from 'src/types/tsfix';
import { getRankGrade } from '../../utils/search/utils';
import ContentIpfs from '../contentIpfs/contentIpfs';
import SearchItem from '../SearchItem/searchItem';

type ContentItemProps = {
  item?: $TsFixMe;
  cid: string;
  grade?: $TsFixMe;
  className?: string;
  parent?: string;
  linkType?: LinksType;
  setType?: (type: IpfsContentType) => void;
};

function ContentItem({
  item,
  cid,
  grade,
  linkType,
  parent: parentId,
  setType,
  className,
}: ContentItemProps): JSX.Element {
  const { details, status, hidden, content } = useParticle(cid, parentId);

  useEffect(() => {
    details?.type && setType && setType(details?.type);
  }, [details, setType]); // TODO: REFACT - setType rise infinite loop

  if (hidden) {
    return <div />;
  }

  return (
    <Link className={className} style={{ color: '#fff' }} to={routes.oracle.ask.getLink(cid)}>
      <SearchItem
        key={cid}
        linkType={linkType}
        status={status}
        grade={
          item?.rank ? getRankGrade(item.rank) : grade || { from: 'n/a', to: 'n/a', value: 'n/a' }
        }
      >
        <ContentIpfs details={details} content={content} cid={cid} search />
      </SearchItem>
    </Link>
  );
}

export default ContentItem;
