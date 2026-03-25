import { useMemo } from 'react';
import { FormatNumberTokens } from 'src/components';
import { BASE_DENOM } from 'src/constants/config';
import ContainerGradient from '../../../../components/containerGradient/ContainerGradient';
import DonutChart from '../../../../components/DonutChart';
import { GIFT_ICON } from '../../utils';
import { ProgressBar } from '../progressCard';
import styles from './ReleaseStatus.module.scss';

type DataReleaseStatus = {
  availableRelease: number;
  released: number;
  leftRelease: number;
  alreadyClaimed?: number;
};

const dataReleaseStatusMap = (data: DataReleaseStatus) => {
  const dataTotal = [
    {
      title: 'available for release',
      color: '#ED2BE7',
      value: data.availableRelease,
    },
    {
      title: 'released',
      color: '#76FF03',
      value: data.released,
    },
    {
      title: 'left to release',
      color: '#FF5C00',
      value: data.leftRelease,
    },
  ];

  if (data.alreadyClaimed) {
    dataTotal.splice(2, 0, {
      title: 'already claimed',
      color: '#525252',
      value: data.alreadyClaimed,
    });
  }
  return dataTotal;
};

type Item = {
  title: string;
  color: string;
  value: number;
};

type Status = 'red' | 'green';

type Props = {
  amountGiftValue: number;
  status?: Status;
  progress: number;
  data: DataReleaseStatus;
  nextRelease: number;
};

function RowItem({ item }: { item: Item }) {
  return (
    <div className={styles.RowItemReleaseStatus}>
      <div style={{ color: item.color }}>{item.title}</div>
      <div>
        <FormatNumberTokens value={item.value} text={BASE_DENOM} />
      </div>
    </div>
  );
}

function ReleaseStatus({
  amountGiftValue = 0,
  status = 'green',
  progress = 0,
  data,
  nextRelease,
}: Props) {
  const title = (
    <div className={styles.title}>
      <div>release status {GIFT_ICON}</div>
      <div>
        <FormatNumberTokens value={amountGiftValue} text={BASE_DENOM} />
      </div>
    </div>
  );

  const renderDataDonutChart = useMemo(() => dataReleaseStatusMap(data), [data]);

  return (
    <ContainerGradient
      title={title}
      closedTitle={title}
      styleLampContent={status}
      userStyleContent={{ height: '475px', display: 'grid', gap: '30px' }}
    >
      <div className={styles.detailsReleaseStatus}>
        <div>
          <DonutChart data={renderDataDonutChart} />
        </div>
        <div className={styles.containerRowItem}>
          {renderDataDonutChart.map((item) => (
            <RowItem key={item.title} item={item} />
          ))}
        </div>
      </div>

      <div className={styles.descriptionRules}>
        Only the most dexterous will be lucky. The early birds get a higher bonus gift release in
        every 1000 new addresses registered.
      </div>

      <div>
        {nextRelease > 0 && (
          <div className={styles.nextRelease}>
            <span>next release after</span>
            <span className={styles.nextReleaseValue}>{nextRelease} addresses</span>
          </div>
        )}

        <ProgressBar progress={progress} />
        <div className={styles.containerProcentValue}>
          <div>0%</div>
          <div>100%</div>
        </div>
      </div>
    </ContainerGradient>
  );
}

export default ReleaseStatus;
