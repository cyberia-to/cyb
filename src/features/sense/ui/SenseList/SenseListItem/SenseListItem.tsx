import cx from 'classnames';
import { MsgSend } from 'cosmjs-types/cosmos/bank/v1beta1/tx';
import { Account, Tooltip } from 'src/components';
import Pill from 'src/components/Pill/Pill';
import { LLMAvatar } from 'src/containers/Search/LLMSpark/LLMSpark';
import { isParticle as isParticleFunc } from 'src/features/particle/utils';
import { SenseItem } from 'src/features/sense/redux/sense.redux';
import CoinsAmount, { CoinAction } from '../../components/CoinAmount/CoinAmount';

import Date from '../../components/Date/Date';
import ParticleAvatar from '../../components/ParticleAvatar/ParticleAvatar';
import { cutSenseItem } from '../../utils';
import styles from './SenseListItem.module.scss';

type Props = {
  address: string;
  date: string;
  content: string | JSX.Element;
  unreadCount: number;

  amountData?: {
    amount: MsgSend['amount'] | undefined;
    isAmountSendToMyAddress?: boolean;
  };

  status?: SenseItem['status'] | undefined;
  fromLog?: boolean;

  // maybe temp
  title?: string;
};

function SenseListItem({
  unreadCount,
  address,
  date,
  content,
  isLLM,
  status,
  amountData,
  from,
  fromLog,
  title,
}: Props) {
  const isParticle = address && isParticleFunc(address);

  let icon;
  let statusText;

  if (status === 'pending') {
    icon = '⏳';
    statusText = 'tx pending';
  } else if (status === 'error') {
    icon = '❗️';
    // TODO: add error message
    statusText = 'error';
  } else if (fromLog) {
    icon = '☘️';
    statusText = 'message from log';
  }

  const withAmount = Boolean(amountData?.amount?.length);

  const avatarContent = (
    <>
      {!isParticle && !isLLM ? (
        <Account address={address} onlyAvatar avatar sizeAvatar={50} />
      ) : (
        !isLLM && <ParticleAvatar particleId={address} />
      )}

      {isLLM && (
        <div className={styles.llmAvatar}>
          <LLMAvatar onlyImg />
        </div>
      )}

      {icon && (
        <Tooltip tooltip={statusText!}>
          <span className={styles.icon}>{icon}</span>
        </Tooltip>
      )}

      {address !== from && (
        <div className={styles.icon}>
          <Account address={from} onlyAvatar avatar sizeAvatar={20} />
        </div>
      )}
    </>
  );

  let titleJSX;

  if (isLLM) {
    titleJSX = title;
  } else if (isParticle) {
    titleJSX = title || `#${cutSenseItem(address)}`;
  } else {
    titleJSX = (
      <>
        @<Account address={address} />
      </>
    );
  }

  // return (
  //   <SenseListItemSimple avatarContent={avatarContent}/>
  // )

  return (
    <div className={styles.wrapper}>
      <div className={styles.avatar}>{avatarContent}</div>

      <h5
        className={cx(styles.title, {
          [styles.particleTitle]: isParticle,
          [styles.uppercase]: !title,
        })}
        onClickCapture={(e) => e.preventDefault()}
      >
        {titleJSX}
      </h5>

      <div
        className={cx(styles.content, {
          [styles.withAmount]: withAmount,
        })}
      >
        <p>{content}</p>

        {withAmount && (
          <div className={styles.amounts}>
            <CoinsAmount
              amount={amountData!.amount!.slice(0, 1)}
              type={
                amountData!.isAmountSendToMyAddress === false ? CoinAction.send : CoinAction.receive
              }
            />
          </div>
        )}
      </div>

      {date && <Date timestamp={date} className={styles.date} />}

      {unreadCount > 0 && (
        <Pill className={styles.unread} text={unreadCount > 99 ? '99+' : unreadCount.toString()} />
      )}
    </div>
  );
}

export default SenseListItem;

function _SenseListItemSimple({ date, title, content, avatarContent }: Props) {
  return (
    <div className={styles.wrapper}>
      <div className={styles.avatar}>{avatarContent}</div>
    </div>
    // <div className={styles.wrapper}>
    //   <div className={styles.date}>
    //     <Date timestamp={date} />
    //   </div>

    //   <div className={styles.content}>
    //     <h5 className={styles.title}>{title}</h5>
    //     <p>{content}</p>
    //   </div>
    // </div>
  );
}
