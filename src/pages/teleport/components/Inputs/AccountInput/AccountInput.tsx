import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { Account, Input } from 'src/components';
import ButtonsGroup from 'src/components/buttons/ButtonsGroup/ButtonsGroup';
import LinearGradientContainer, {
  Color,
} from 'src/components/LinearGradientContainer/LinearGradientContainer';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { getPassportByNickname } from 'src/containers/portal/utils';
import { useQueryClient } from 'src/contexts/queryClient';
import {
  SliceState,
  selectAccountsPassports,
  selectCommunityPassports,
} from 'src/features/passport/passports.redux';
import useDebounce from 'src/hooks/useDebounce';
import useOnClickOutside from 'src/hooks/useOnClickOutside';
import { useAppSelector } from 'src/redux/hooks';
import { Networks } from 'src/types/networks';
import { fromBech32 } from 'src/utils/utils';
import { TypeRecipient } from '../type';
import contains from '../utils';
import styles from './AccountInput.module.scss';
import AccountInputListContainer from './AccountInputContainer';
import AccountInputOptionList from './AccountInputItem';

type Props = {
  recipient: string | undefined;
  setRecipient: React.Dispatch<React.SetStateAction<string | undefined>>;
  title?: string;
};

const PLACEHOLDER_TITLE = 'choose recipient';

function AccountInput({ recipient, setRecipient, title = PLACEHOLDER_TITLE }: Props) {
  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const inputElem = useRef(null);
  const selectContainerRef = useRef(null);
  const { debounce } = useDebounce();
  const firstEffectOccured = useRef(false);

  const [isOpen, setIsOpen] = useState(false);
  const [selectedTypeRecipient, setSelectedTypeRecipient] = useState(TypeRecipient.friends);
  const [valueRecipient, setValueRecipient] = useState<string>('');
  const [listRecipient, setListRecipient] = useState<SliceState>({});

  const selectCommunity = useAppSelector(selectCommunityPassports);
  const { accounts } = useAppSelector(selectAccountsPassports);

  const clickOutsideHandler = () => setIsOpen(false);

  useOnClickOutside(selectContainerRef, clickOutsideHandler);

  useEffect(() => {
    if (recipient?.length && recipient.match(PATTERN_CYBER)) {
      clickOutsideHandler();
    }
  }, [recipient, clickOutsideHandler]);

  const selectedDataRecipient = useMemo(() => {
    if (selectedTypeRecipient === TypeRecipient.my) {
      return accounts;
    }

    if (selectedTypeRecipient === TypeRecipient.following) {
      return selectCommunity.following;
    }

    return selectCommunity.friends;
  }, [selectedTypeRecipient, accounts, selectCommunity]);

  function handleSetRecipient(value: string) {
    let v = value;
    if (CHAIN_ID === Networks.SPACE_PUSSY) {
      v = fromBech32(value, 'pussy');
    }
    setRecipient(v);
  }

  const getRecipient = useCallback(
    async (value: string) => {
      if (!value) {
        setRecipient(undefined);
        setListRecipient({});
        return;
      }

      if (value.match(PATTERN_CYBER)) {
        handleSetRecipient(value);
        setListRecipient({});
        return;
      }

      let listRecipientTemp: SliceState = {};
      if (Object.keys(selectedDataRecipient).length > 0) {
        const result = contains(value, selectedDataRecipient);
        if (Object.keys(result).length) {
          listRecipientTemp = { ...result };
        }
      }

      const resultHandle = await await getPassportByNickname(queryClient, value);

      if (resultHandle) {
        const resultHandleRecipient = {
          [resultHandle.owner]: { loading: false, data: resultHandle },
        };

        listRecipientTemp = { ...resultHandleRecipient, ...listRecipientTemp };
      }

      setListRecipient(listRecipientTemp);
    },
    [selectedDataRecipient, queryClient, setRecipient, handleSetRecipient]
  );

  const handleSearch = useCallback(
    debounce((inputVal: string) => getRecipient(inputVal), 500),
    []
  );

  const onChangeRecipient = useCallback(
    (inputVal: string) => {
      if (!inputVal.length) {
        setListRecipient(selectedDataRecipient);
      }
      setValueRecipient(inputVal);
      handleSearch(inputElem.current?.value);
    },
    [handleSearch, selectedDataRecipient]
  );

  useEffect(() => {
    if (!firstEffectOccured.current) {
      firstEffectOccured.current = true;
      const param = Object.fromEntries(searchParams.entries());
      if (Object.keys(param).length > 0) {
        const { recipient: recipientParam } = param;
        if (recipientParam) {
          setValueRecipient(recipientParam);
          handleSearch(recipientParam);
        }
      }
    }
  }, [handleSearch, searchParams]);

  const useListRecipient = useMemo(() => {
    if (!valueRecipient.length || !Object.keys(listRecipient).length) {
      return selectedDataRecipient;
    }
    return listRecipient;
  }, [listRecipient, valueRecipient, selectedDataRecipient]);

  const onClickByNickname = (owner: string, nickname: string | undefined) => {
    setValueRecipient(nickname || owner);

    handleSetRecipient(owner);
    clickOutsideHandler();
  };

  if (!isOpen && recipient) {
    return (
      <button type="button" onClick={() => setIsOpen(true)} className={styles.containerBtnValue}>
        <LinearGradientContainer color={Color.Green} title={title}>
          <Account avatar disabled address={recipient} styleUser={{ height: '42px' }} />
        </LinearGradientContainer>
      </button>
    );
  }

  return (
    <div className={styles.containerListAndInput} ref={selectContainerRef}>
      <Input
        ref={inputElem}
        id="recipient"
        value={valueRecipient}
        onChange={(e) => onChangeRecipient(e.target.value)}
        title={title}
        color={!recipient ? Color.Red : Color.Green}
        classNameTextbox={styles.contentValueInput}
        onFocusFnc={() => setIsOpen(true)}
      />

      {isOpen && (
        <AccountInputListContainer>
          <div className={styles.containerButtonText}>
            <ButtonsGroup
              type="radio"
              items={Object.values(TypeRecipient).map((recipType) => {
                return {
                  label: recipType,
                  name: recipType,
                  checked: selectedTypeRecipient === recipType,
                };
              })}
              onChange={(val: TypeRecipient) => setSelectedTypeRecipient(val)}
            />
          </div>

          {useListRecipient && Object.keys(useListRecipient).length > 0 && (
            <AccountInputOptionList data={useListRecipient} onClickByNickname={onClickByNickname} />
          )}
        </AccountInputListContainer>
      )}
    </div>
  );
}

AccountInput.displayName = 'AccountInput';

export default AccountInput;
