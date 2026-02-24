import ContainerGradient from '../../../../components/containerGradient/ContainerGradient';
import styles from './styles.module.scss';

export function ProgressBar({
  progress = 0,
  styleContainer,
  styleContainerTrack,
  rotate = 0,
  coefficient,
  amount,
}) {
  return (
    <div
      className={styles.containerProgressBar}
      style={{ ...styleContainer, transform: `rotate(${rotate}deg)` }}
    >
      {coefficient && <span style={{ transform: `rotate(-${rotate}deg)` }}>{coefficient.up}x</span>}
      <div className={styles.containerProgressBarTrack} style={styleContainerTrack}>
        <div style={{ width: `${progress}%` }} className={styles.containerProgressBarTrackProgress}>
          <div className={styles.containerProgressBarTrackProgressValue}>
            <span style={{ transform: `rotate(-${rotate}deg)` }}>{amount || `${progress}%`}</span>
          </div>
        </div>
      </div>
      {coefficient && (
        <span style={{ transform: `rotate(-${rotate}deg)` }}>{coefficient.down}x</span>
      )}
    </div>
  );
}

// progressCard
// headerTitle
// footerText
// titleValue

function ProgressCard({
  headerText = '',
  footerText = '',
  titleValue = 0,
  progress = 0,
  styleContainerTrack,
  status = 'red',
}) {
  const title = (
    <div className={styles.containerBeforeActivationTitle}>
      <div>{headerText}</div>
      <div>{titleValue}</div>
    </div>
  );

  const closedTitle = (
    <div className={styles.containerBeforeActivationTitle}>
      <div style={{ color: '#1fcbff' }}>{headerText}</div>
      <div>{titleValue}</div>
    </div>
  );

  return (
    <ContainerGradient
      title={title}
      closedTitle={closedTitle}
      styleLampTitle={status}
      styleLampContent={status}
      userStyleContent={{ height: '194px' }}
    >
      <div className={styles.containerBeforeActivation}>
        <ProgressBar progress={progress} styleContainerTrack={styleContainerTrack} />
        <div className={styles.containerBeforeActivationFooter}>
          <div>0%</div>
          <div style={{ color: '#999999' }}> {footerText}</div>
          <div>100%</div>
        </div>
      </div>
    </ContainerGradient>
  );
}

export default ProgressCard;
