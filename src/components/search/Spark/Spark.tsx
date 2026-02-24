import cx from 'classnames';
import ContentItem from 'src/components/ContentItem/contentItem';
import { LinksType } from 'src/containers/Search/types';
import { useDevice } from 'src/contexts/device';

import { useHover } from 'src/hooks/useHover';
import type { IpfsContentType } from 'src/services/ipfs/types';
import Creator from './LeftMeta/Creator/Creator';
import RankButton from './LeftMeta/RankButton/RankButton';
import Meta from './Meta/Meta';
import styles from './Spark.module.scss';

type Props = {
  cid: string;
  handleContentType: (type: IpfsContentType) => void;
  handleRankClick?: (cid: string) => void;
  itemData: {};
  query: string;
  linkType: LinksType;
  selfLinks?: boolean;
};

function Spark({
  cid,
  itemData,
  query,
  linkType,
  rankSelected,
  handleContentType,
  handleRankClick,
  selfLinks,
}: Props) {
  const { isMobile } = useDevice();
  const [ref, hovering] = useHover();

  return (
    <div className={cx(styles.wrapper, 'hover-rank')} ref={ref}>
      {!isMobile && hovering && (
        <>
          <div className={styles.left}>
            <Creator cid={cid} onlyTime={selfLinks} />
            {handleRankClick && (
              <RankButton cid={cid} rankSelected={rankSelected} handleRankClick={handleRankClick} />
            )}
          </div>

          {/* TODO: refact. meta should be moved inside contentItem and exclude fetchParticle from that  */}
          {!selfLinks && (
            <div className={styles.right}>
              <Meta cid={cid} />
            </div>
          )}
        </>
      )}

      <ContentItem
        setType={handleContentType}
        cid={cid}
        item={itemData}
        parent={query}
        linkType={linkType}
        className={styles.searchItem}
      />
    </div>
  );
}

export default Spark;
