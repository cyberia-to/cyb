import { Pane } from '@cybercongress/gravity';
import { chromeDark, ObjectInspector } from '@tableflip/react-inspector';
import { NoItems } from '../../../components';
import { formatCurrency } from '../../../utils/utils';
import styles from './metaInfo.module.scss';

const objectInspectorTheme = {
  ...chromeDark,
  BASE_FONT_SIZE: '13px',
  BASE_LINE_HEIGHT: '19px',
  TREENODE_FONT_SIZE: '13px',
  TREENODE_LINE_HEIGHT: '19px',
};

export const PREFIXES = [
  {
    prefix: 'T',
    power: 1024 * 10 ** 9,
  },
  {
    prefix: 'G',
    power: 1024 * 10 ** 6,
  },
  {
    prefix: 'M',
    power: 1024 * 10 ** 3,
  },
  {
    prefix: 'K',
    power: 1024,
  },
];

function RowItem({ text, value }) {
  return (
    <Pane display="flex" marginBottom={10}>
      <Pane width={50}>{text}:</Pane>
      <Pane>{value}</Pane>
    </Pane>
  );
}

// TODO: seems not used and need to be removed
function MetaInfo({ data, cid }) {
  try {
    if (!data) {
      return <div>loading....</div>;
    }
    return (
      <>
        <RowItem text="CID" value={cid} />
        <RowItem text="SIZE" value={formatCurrency(data.size, 'B', 3, PREFIXES)} />
        {data.blockSizes && <RowItem text="LINKS" value={data.blockSizes.length} />}
        <RowItem text="DATA" value="" />

        <div className={styles.objectInspector}>
          <ObjectInspector
            showMaxKeys={100}
            data={data}
            theme={objectInspectorTheme}
            // expandPaths={toExpandPathsNotation(localPath)}
          />
        </div>
      </>
    );
  } catch (_error) {
    return <NoItems text="oops..." />;
  }
}

export default MetaInfo;
