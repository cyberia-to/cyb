import { Link } from 'react-router-dom';
import { FormatNumberTokens } from 'src/components';
import { cybernetRoutes } from 'src/features/cybernet/ui/routes';
import styles from './styles.module.scss';

const iconObj = {
  liquid: '💧',
  frozen: '❄️',
  melting: '☀️',
  growth: '🎋',
  commission: '💫',
  cyberver: '❄️',
};

function RowItem({ text, value, cap }) {
  return (
    <div className={styles.containerRowItem}>
      <div className={styles.containerRowItemValueBalance}>
        <div className={styles.containerRowItemValueBalanceText}>
          {text === 'cyberver' ? <Link to={cybernetRoutes.sigma.getLink()}>{text}</Link> : text}{' '}
          {iconObj[text] ? iconObj[text] : ''}
        </div>
        <div className={styles.containerRowItemValueBalanceValue}>
          <FormatNumberTokens value={value.amount} />
        </div>
      </div>
      <div className={styles.containerRowItemMarket}>
        <FormatNumberTokens value={cap.amount} text={cap.denom} />
      </div>
    </div>
  );
}

export default RowItem;
