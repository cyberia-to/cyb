import { useNavigate } from 'react-router-dom';
import { formatNumber } from '../../../utils/utils';
import Card from '../ui/card';
import styles from './statistics.module.scss';

type Props = {
  myEnergy: number;
  income: number;
  outcome: number;
  active?: string;
};

function Statistics({ myEnergy = 0, income = 0, outcome = 0, active }: Props) {
  const navigate = useNavigate();

  const freeEnergy = myEnergy + income - outcome;

  const onClickNavigate = (to?: string) => {
    if (!active) {
      navigate(`./${to || ''}`);
    } else {
      navigate(`../${to || ''}`, { relative: 'path' });
    }
  };

  return (
    <div className={styles.row}>
      <Card
        active={!active}
        title="Enegy"
        value={`${formatNumber(myEnergy)} W`}
        onClick={() => onClickNavigate()}
      />
      <span className={styles.operator}>+</span>
      <Card
        active={active === 'income'}
        title="Income"
        value={`${formatNumber(income)} W`}
        onClick={() => onClickNavigate('income')}
      />
      <span className={styles.operator}>-</span>
      <Card
        active={active === 'outcome'}
        title="Outcome"
        value={`${formatNumber(outcome)} W`}
        onClick={() => onClickNavigate('outcome')}
      />
      <span className={styles.operator}>=</span>
      <Card
        title="Free Energy"
        value={`${formatNumber(freeEnergy > 0 ? freeEnergy : 0)} W`}
      />
    </div>
  );
}

export default Statistics;
