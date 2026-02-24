import cx from 'classnames';
import { equals } from 'ramda';
import { useSelector } from 'react-redux';
import { Link } from 'react-router-dom';
import MusicalAddress from 'src/components/MusicalAddress/MusicalAddress';
import Pill from 'src/components/Pill/Pill';
import Loader2 from 'src/components/ui/Loader2';
import { AvataImgIpfs } from 'src/containers/portal/components/avataIpfs';
import usePassportContract from 'src/features/passport/usePassportContract';
import { RootState } from 'src/redux/store';
import { routes } from 'src/routes';
import { Citizenship } from 'src/types/citizenship';
import { AccountValue } from 'src/types/defaultAccount';
import styles from './KeyItem.module.scss';

type Props = {
  account: AccountValue;
};

function PassportLoader({
  tokenId,
  render,
}: {
  tokenId: string;
  render: (passport: Citizenship) => JSX.Element | null;
}) {
  const { data: passport } = usePassportContract<Citizenship>({
    query: {
      nft_info: {
        token_id: tokenId,
      },
    },
  });

  if (!passport) {
    return null;
  }

  return render(passport);
}

function KeyItem({ account, selected, selectKey }: Props) {
  const { name, bech32, keys, path } = account;

  const { defaultAccount } = useSelector((state: RootState) => state.pocket);

  const isActive = defaultAccount.account?.cyber?.bech32 === bech32;

  const activePassport = usePassportContract<Citizenship>({
    query: {
      active_passport: {
        address: bech32,
      },
    },
  });

  const passportIds = usePassportContract<{ tokens: string[] }>({
    query: {
      tokens: {
        owner: bech32,
      },
    },
  });

  const passportsLoading = activePassport.loading && passportIds.loading;

  const isReadOnly = keys === 'read-only';
  const isHardware = keys === 'ledger';

  function renderPassportPill(
    { extension: { nickname, avatar } }: Citizenship,
    isMainPassport?: boolean
  ) {
    return (
      <Link to={routes.robotPassport.getLink(nickname)}>
        <Pill
          image={<AvataImgIpfs cidAvatar={avatar} />}
          text={
            <>
              <span className={styles.passportName}>{nickname}</span>
              {isMainPassport && <span className={styles.passportMain}>main</span>}
            </>
          }
        />
      </Link>
    );
  }

  return (
    <div
      className={cx(styles.wrapper, {
        [styles.selected]: selected,
      })}
      onClick={() => selectKey(bech32)}
    >
      <div className={styles.imageWrapper}>
        <img src={require('./images/1.png')} alt="" />

        {isActive && <Pill text="active" color="green" className={styles.active} />}
      </div>

      <div className={styles.content}>
        key <Pill color="white" text={name || 'noname'} /> <br />
        {['keplr'].includes(keys) && (
          <>
            signed by <Pill color="red" text={keys} />{' '}
          </>
        )}
        {isHardware && (
          <>
            stored in <Pill color="red" text={keys} />
          </>
        )}{' '}
        {path && (
          <>
            and located at the path <Pill text={path.join('/')} /> <br />
          </>
        )}
        from neuron <Pill text={<MusicalAddress address={bech32} />} /> <br />
        gives <Pill color={isReadOnly ? 'blue' : 'green'} text={isReadOnly ? 'read' : 'write'} />{' '}
        access
        {(passportsLoading || passportIds.data?.tokens.length || activePassport.data) && (
          <>
            {' '}
            to avatars: <br />
          </>
        )}
        {passportsLoading && <Loader2 />}
        {(activePassport.data || !!passportIds.data?.tokens?.length) && (
          <div className={styles.passports}>
            {activePassport.data && renderPassportPill(activePassport.data, true)}
            {passportIds.data?.tokens?.map((tokenId) => {
              return (
                <PassportLoader
                  key={tokenId}
                  tokenId={tokenId}
                  render={(passport) => {
                    if (
                      activePassport.data &&
                      equals(activePassport.data.extension, passport.extension)
                    ) {
                      return null;
                    }
                    return renderPassportPill(passport);
                  }}
                />
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

export default KeyItem;
