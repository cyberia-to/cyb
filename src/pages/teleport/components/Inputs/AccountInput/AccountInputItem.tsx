import { Account } from 'src/components';
import { SliceState } from 'src/features/passport/passports.redux';
import { Nullable } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { trimString } from 'src/utils/utils';
import { DataItem } from '../type';
import styles from './AccountInput.module.scss';

type PropsOptionItem = {
  data: DataItem;
};

type OnClickByNickname = (owner: string, nckname: string | undefined) => void;

type Props = {
  address: string;
  item: Nullable<Citizenship>;
  onClickByNickname: OnClickByNickname;
};

function OptionItem({ data }: PropsOptionItem) {
  const { nickname, owner } = data;

  const ownerTrim = trimString(owner, 10, 3);

  return (
    <div className={styles.containerOptionItem}>
      <Account
        avatar
        disabled
        address={owner}
        monikerClassName={styles.containerNickName}
        containerClassName={styles.containerAccount}
      />

      <span className={styles.containerOwner}>{nickname && ownerTrim}</span>
    </div>
  );
}

function AccountInputOption({ address, item, onClickByNickname }: Props) {
  let owner = address;
  let nickname: string | undefined;
  let cidAvatar;

  if (item) {
    const { extension } = item;

    nickname = extension.nickname;
    owner = item.owner;
    cidAvatar = extension.avatar;
  }

  return (
    <button
      key={owner}
      onClick={() => onClickByNickname(owner, nickname)}
      type="button"
      className={styles.containerButtonOption}
    >
      <OptionItem data={{ nickname, cidAvatar, owner }} />
    </button>
  );
}

function AccountInputOptionList({
  data,
  onClickByNickname,
}: {
  data: SliceState;
  onClickByNickname: OnClickByNickname;
}) {
  return (
    <div className={styles.containerOption}>
      {Object.keys(data).map((key) => (
        <AccountInputOption
          key={key}
          address={key}
          item={data[key]?.data}
          onClickByNickname={onClickByNickname}
        />
      ))}
    </div>
  );
}

export default AccountInputOptionList;
