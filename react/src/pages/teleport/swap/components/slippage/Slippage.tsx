import styles from './Slippage.module.scss';

type Props = {
  value: number;
};

function Slippage({ value }: Props) {
  return (
    <div className={styles.container}>
      slippage: <span className={styles.value}>{value}%</span>
    </div>
  );
}

export default Slippage;
