import { Option } from 'src/types';
import { IpfsContent, IpfsContentSource } from 'src/utils/ipfs/ipfs';
import styles from './DebugContentInfo.module.scss';

function DebugContentInfo({
  cid,
  source,
  content,
  status,
}: {
  cid: string;
  source: Option<IpfsContentSource>;
  content: Option<IpfsContent>;
  status: Option<string>;
}) {
  const meta = content ? content.meta : undefined;
  const measurementInfo =
    source !== 'db'
      ? ` stats(ms): ${meta?.statsTime} cat(ms): ${meta?.catTime} pin(ms): ${meta?.pinTime}`
      : '';

  return (
    <div
      className={styles.contentLoadInfo}
      onClick={async (e) => {
        e.preventDefault();
        e.stopPropagation();
        await navigator.clipboard.writeText(cid);
      }}
    >
      <div>{`source: ${source}  status: ${status} cid: ${cid}`}</div>
      <div>{`blocks: ${meta?.blocks} mime: ${meta?.mime} size: ${meta?.size} local: ${meta?.local}`}</div>
      {measurementInfo && <div>{measurementInfo}</div>}
    </div>
  );
}

export default DebugContentInfo;
