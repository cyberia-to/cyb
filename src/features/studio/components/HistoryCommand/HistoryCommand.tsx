import { CmdKey } from '@milkdown/kit/core';
import { redoCommand, undoCommand } from '@milkdown/kit/plugin/history';
import cx from 'classnames';
import ButtonsGroup from 'src/components/buttons/ButtonsGroup/ButtonsGroup';
import styles from './HistoryCommand.modules.scss';
import reload from './reload.png';

function ImgReload({ reverse }: { reverse?: boolean }) {
  return (
    <img
      className={cx(styles.reloadImg, { [styles.reverse]: reverse })}
      src={reload}
      alt="reload"
    />
  );
}

const contentTypeConfig = {
  undo: {
    label: <ImgReload />,
    tooltip: 'undo',
  },
  redo: {
    label: <ImgReload reverse />,
    tooltip: 'redo',
  },
};

function HistoryCommand({ call }: { call: (command: CmdKey<unknown>) => boolean | undefined }) {
  return (
    <div className={styles.wrapper}>
      <ButtonsGroup
        type="checkbox"
        onChange={(name) => {
          let value: CmdKey<unknown>;
          switch (name) {
            case 'undo':
              value = undoCommand.key;
              break;

            case 'redo':
              value = redoCommand.key;
              break;

            default:
              break;
          }

          call(value);
        }}
        items={Object.values(contentTypeConfig).map((type) => {
          return {
            label: type.label,
            name: type.tooltip,
            checked: type === '',
            tooltip: type.tooltip,
          };
        })}
      />
    </div>
  );
}

export default HistoryCommand;
