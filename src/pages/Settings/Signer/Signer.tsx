import React, { useCallback } from 'react';
import { Display, DisplayTitle } from 'src/components';
import Switch from 'src/components/Switch/Switch';
import { setConfirmation } from 'src/redux/features/signer';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';

import { localStorageKeys } from 'src/constants/localStorageKeys';
import * as styles from './Signer.style';

function Signer() {
  const dispatch = useAppDispatch();
  const confirmation = useAppSelector((state) => state.signer.confirmation);
  const onChange = useCallback(
    (checked: boolean) => {
      dispatch(setConfirmation(checked));
      localStorage.setItem(
        localStorageKeys.signer.confirmation,
        JSON.stringify(checked)
      );
    },
    [dispatch]
  );

  return (
    <Display
      noPadding
      title={<DisplayTitle title="Signer confirmation page" />}
    >
      <div style={styles.switchWrapper}>
        <Switch value={confirmation} label="enabled" onChange={onChange} />
      </div>
    </Display>
  );
}

export default React.memo(Signer);
