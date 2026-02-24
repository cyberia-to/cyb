import { ActionBar, Display } from 'src/components';
import styles from './UnderConstruction.module.scss';

function UnderConstruction() {
  return (
    <>
      <Display>
        <div className={styles.wrapper}>
          <img src={require('./under-construction.png')} alt="Under construction" />

          <h5>
            the page is <br /> under construction
          </h5>
        </div>
      </Display>

      <ActionBar
        button={{
          text: 'Back',
          onClick: () => window.history.back(),
        }}
      />
    </>
  );
}

export default UnderConstruction;
