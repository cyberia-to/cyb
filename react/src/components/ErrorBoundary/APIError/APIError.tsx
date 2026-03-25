import MainContainer from 'src/components/MainContainer';
import Socials from 'src/pages/Social/Socials';
import styles from './APIError.module.scss';

function APIError() {
  return (
    <MainContainer>
      {/* <Display title={<DisplayTitle title="App connection error" />}> */}

      <h3 className={styles.title}>API connection error 😢</h3>
      <br />

      <p
        style={{
          marginBottom: 30,
        }}
      >
        check socials for details
      </p>
      <Socials />
      {/* </Display> */}
    </MainContainer>
  );
}

export default APIError;
