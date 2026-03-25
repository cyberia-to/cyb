import useMediaQuery from '../../../../hooks/useMediaQuery';
import styles from './styles.scss';
import useCanvas from './useCanvas';

function Canvas() {
  const { canvasRef } = useCanvas();
  const mediaQuery = useMediaQuery('(min-width: 768px)');

  return (
    <canvas
      className={styles.slider}
      style={{ top: mediaQuery ? '12%' : '6%' }}
      ref={canvasRef}
      id="canvasOne"
      width="300"
      height="320"
    />
  );
}

export default Canvas;
