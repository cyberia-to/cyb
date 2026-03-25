import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import ContainerGradient from '../../../../components/containerGradient/ContainerGradient';
import { formatNumber } from '../../../../utils/utils';
import { ProgressBar } from '../progressCard';
import styles from './styles.scss';

const linkMovie =
  'https://gateway.ipfs.cybernode.ai/ipfs/QmX138nJEXWjf37M85HjWwBzY88H3ULpJZNHgFPwfCQycA';

function AboutGift({ coefficient, stateOpen, initStateCard, addressesClaimed }) {
  const useProgress = useMemo(() => {
    if (coefficient && coefficient !== null) {
      const maxValue = coefficient.up - coefficient.down;
      const currentValue = coefficient.current - coefficient.down;
      if (currentValue > 0) {
        const progress = new BigNumber(currentValue).dividedBy(maxValue);
        const curentProcent = new BigNumber(1)
          .minus(progress)
          .multipliedBy(100)
          .dp(1, BigNumber.ROUND_FLOOR)
          .toNumber();
        return curentProcent;
      }
    }
    return 0;
  }, [coefficient]);

  const current = new BigNumber(coefficient.current).dp(1, BigNumber.ROUND_FLOOR).toNumber();

  return (
    <ContainerGradient
      userStyleContent={{ height: '710px' }}
      styleLampContent="purple"
      title="about gift"
    >
      <div
        style={{
          width: '100%',
          background: 'transparent',
          position: 'relative',
          height: '300px',
          marginBottom: '10px',
        }}
      >
        {/*  eslint-disable-next-line jsx-a11y/media-has-caption */}
        <video width="100%" height="100%" controls>
          <source src={linkMovie} type="video/mp4" />
        </video>
      </div>
      <div style={{ marginBottom: '10px' }}>
        Hurry up! {formatNumber(parseFloat(addressesClaimed))} addresses already claimed a gift.
        Only the most dexterous will be lucky. The early birds get a higher bonus.
      </div>
      <div>
        70% of BOOT in the genesis of the Bostrom bootloader is allocated to ~4 million addresses in
        Ethereum, Cosmos, Osmosis and Terra.
      </div>
      <div className={styles.containerAboutGiftProgressBar}>
        <div className={styles.containerAboutGiftProgressBarTitle}>Current bonus</div>
        <ProgressBar
          progress={useProgress}
          styleContainer={{ margin: '0', gridGap: '10px' }}
          styleContainerTrack={{ padding: '0' }}
          rotate={8}
          coefficient={coefficient}
          amount={`${current} x`}
        />
        <div className={styles.containerAboutGiftProgressBarFooter}>
          <div>0%</div>
          <div style={{ color: '#999999' }}>tokens claimed</div>
          <div>100%</div>
        </div>
      </div>
      <div>Gift to be released after 100 000 addresses claimed.</div>
    </ContainerGradient>
  );
}

export default AboutGift;
