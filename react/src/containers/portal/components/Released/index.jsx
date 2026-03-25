import { ContainerGradientText } from '../../../../components/containerGradient/ContainerGradient';
import styles from './styles.scss';

function Released({ released }) {
  console.log('released', released);
  return (
    <ContainerGradientText status="red">
      <div className={styles.containerReleased}>
        <div>released</div>
        {/* <div>{released}%</div> */}
      </div>
    </ContainerGradientText>
  );
}

// eslint-disable-next-line import/no-unused-modules
export default Released;
