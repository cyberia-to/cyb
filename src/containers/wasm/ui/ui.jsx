import ReactJson from 'react-json-view';
import { Link } from 'react-router-dom';
import styles from './styles.scss';

function JsonView({ src, strLength }) {
  return (
    <ReactJson
      src={src}
      name={false}
      displayDataTypes={false}
      displayObjectSize={false}
      collapseStringsAfterLength={strLength ?? 24}
      theme="twilight"
    />
  );
}

const jsonInputStyle = {
  container: { display: 'flex', flexDirection: 'column' },
  body: { order: '1' },
  warningBox: { order: '2' },
};

function FlexWrapCantainer({ children, ...props }) {
  return (
    <div {...props} className={styles.containerFlexWrapCantainer}>
      {children}
    </div>
  );
}

function CardCantainer({ children, ...props }) {
  return (
    <div {...props} className={styles.containerCardCantainer}>
      {children}
    </div>
  );
}

function LinkTx({ children, txs }) {
  return <Link to={`/network/bostrom/tx/${txs}`}>{children}</Link>;
}

function LinkCreator({ children, address }) {
  return <Link to={`/network/bostrom/contract/${address}`}>{children}</Link>;
}

function ContainerCardStatisics({ children }) {
  return <div className={styles.containerCardStatisics}>{children}</div>;
}

function ContainerCol({ children }) {
  return <div className={styles.containerCol}>{children}</div>;
}

export {
  jsonInputStyle,
  JsonView,
  FlexWrapCantainer,
  CardCantainer,
  LinkTx,
  LinkCreator,
  ContainerCardStatisics,
  ContainerCol,
};
