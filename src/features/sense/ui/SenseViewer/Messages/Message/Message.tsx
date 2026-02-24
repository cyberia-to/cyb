import cx from 'classnames';
import { MsgSend } from 'cosmjs-types/cosmos/bank/v1beta1/tx';
import { Link, useNavigate } from 'react-router-dom';
import { Account } from 'src/components';
import { SenseItem } from 'src/features/sense/redux/sense.redux';
import { routes } from 'src/routes';
import CoinsAmount, { CoinAction } from '../../../components/CoinAmount/CoinAmount';
import Date from '../../../components/Date/Date';
import styles from './Message.module.scss';

type Props = {
  date: string | Date;
  transactionHash: string;
  content: string | JSX.Element;
  amountData?: {
    amount: MsgSend['amount'] | undefined;
    isAmountSendToMyAddress?: boolean;
  };

  myMessage?: boolean;
  cid?: string;
  fromLog?: boolean;
  status?: SenseItem['status'];
  avatar?: JSX.Element;
};

function Message({
  content,
  date,
  amountData,
  fromLog,
  transactionHash,
  from,
  myMessage,
  cid,
  avatar,
  status,
}: Props) {
  const navigate = useNavigate();

  function handleNavigate() {
    navigate(cid ? routes.oracle.ask.getLink(cid) : routes.txExplorer.getLink(transactionHash));
  }

  return (
    <div
      className={cx(styles.wrapper, {
        [styles.myMessage]: myMessage,
        [styles.pending]: status === 'pending',
        [styles.error]: status === 'error',
      })}
    >
      <div className={styles.dateBlock}>
        <Link
          className={cx(styles.tx, {
            [styles[`status_${status}`]]: status,
          })}
          to={routes.txExplorer.getLink(transactionHash)}
        >
          <Date timestamp={date} timeOnly />
        </Link>

        {
          fromLog && from && !myMessage && (
            <Link to={routes.neuron.getLink(from)}>
              <Account address={from} sizeAvatar="20px" avatar onlyAvatar />
            </Link>
          )
          // <Tooltip tooltip="message from log">
          // <span className={styles.icon}>🍀</span>
          // </Tooltip>
        }

        {avatar}
      </div>

      {/* <Link> not good here, because children may have links  */}
      {/* Twitter example */}
      <div
        role="button"
        tabIndex={0}
        onClick={(e) => {
          // inner link clicked
          if (e.target instanceof HTMLAnchorElement) {
            return;
          }
          // if text not selected
          if (window.getSelection()?.toString()) {
            return;
          }

          handleNavigate();
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            handleNavigate();
          }
        }}
        className={styles.content}
      >
        {content}

        {!!amountData?.amount?.length && (
          <div className={styles.amount}>
            <CoinsAmount
              amount={amountData.amount}
              type={
                amountData.isAmountSendToMyAddress === false ? CoinAction.send : CoinAction.receive
              }
            />
          </div>
        )}
      </div>
    </div>
  );
}

export default Message;
