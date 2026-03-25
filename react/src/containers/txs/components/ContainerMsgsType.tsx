import { ReactNode } from 'react';
import { MsgType } from 'src/components';
import { RowsContainer } from '../../../components/Row/Row';
import styles from './ContainerMsgsType.module.scss';

function ContainerMsgsType({ type, children }: { type: string; children: ReactNode }) {
  return (
    <div className={styles.container}>
      <div className={styles.type}>
        Type: <MsgType type={type} />
      </div>
      <RowsContainer>{children}</RowsContainer>
    </div>
  );
}

export default ContainerMsgsType;
