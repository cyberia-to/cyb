import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import styles from './styles.module';

function Columns({ title, children }) {
  return (
    <div className={styles.columnItem}>
      <Display noPaddingY title={<DisplayTitle title={title} />}></Display>

      {children}
    </div>
  );
}

export default Columns;
