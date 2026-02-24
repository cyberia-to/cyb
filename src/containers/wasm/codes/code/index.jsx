import { useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQueryClient } from 'src/contexts/queryClient';
import { trimString } from '../../../../utils/utils';
import styles from './styles.scss';

export function CardItem({ title, value }) {
  return (
    <div className={styles.containetCodesCardFillsItem}>
      <div className={styles.containetCodesCardFillsItemTitle}>{title}:</div>
      <div className={styles.containetCodesCardFillsItemValue}>{value}</div>
    </div>
  );
}

function Code({ data }) {
  const queryClient = useQueryClient();

  const [instantiationInfo, setInstantiationInfo] = useState(0);

  useEffect(() => {
    const getContracts = async () => {
      try {
        if (queryClient && data.id) {
          const response = await queryClient.getContracts(data.id);
          setInstantiationInfo(response.length);
        }
      } catch (error) {
        console.log(`error getContracts`, error);
        setInstantiationInfo(0);
      }
    };
    getContracts();
  }, [data, queryClient]);

  const { id, creator, checksum } = data;

  return (
    <Link to={`/libs/${data.id}`}>
      <div className={styles.containetCodesCard}>
        <div className={styles.containetCodesCardId}>#{id}</div>
        <div className={styles.containetCodesCardFills}>
          <CardItem title="Creator" value={trimString(creator, 10)} />
          <CardItem title="Checksum" value={trimString(checksum, 8, 8)} />
          <CardItem title="Instances" value={instantiationInfo} />
        </div>
      </div>
    </Link>
  );
}

export default Code;
