import { Pane } from '@cybercongress/gravity';
import { useState } from 'react';
import { useDispatch } from 'react-redux';
import { ActionBar, Input } from 'src/components';
import { setSecret } from 'src/redux/reducers/scripting';

function ActionBarSecrets({ onClickBack }: { onClickBack: () => void }) {
  const [key, setKey] = useState('');
  const [value, setValue] = useState('');
  const dispatch = useDispatch();

  const onSave = async () => {
    dispatch(setSecret({ key, value }));
    onClickBack();
  };

  return (
    <ActionBar
      button={{
        disabled: !key || !value,
        onClick: onSave,
        text: 'add secret',
      }}
      onClickBack={onClickBack}
    >
      <Pane flex={1} justifyContent="center" alignItems="center" fontSize="18px" display="flex">
        <Input
          width="250px"
          value={key}
          onChange={(e) => setKey(e.target.value)}
          placeholder="key/name"
          autoFocus
        />
        <Pane width="10px" />
        <Input
          width="250px"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="secret value"
          autoFocus
        />
      </Pane>
    </ActionBar>
  );
}

export default ActionBarSecrets;
