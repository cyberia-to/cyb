import cx from 'classnames';
import React from 'react';
import Tooltip from 'src/components/tooltip/tooltip';
import styles from './AddFile.module.scss';

export type Props = {
  isRemove: boolean;
} & React.HTMLAttributes<HTMLButtonElement>;

function AddFile({ isRemove, onClick }: Props) {
  const buttonText = `${!isRemove ? 'Add' : 'Remove'} file`;

  return (
    <Tooltip tooltip={buttonText}>
      <button
        type="button"
        aria-label={buttonText}
        className={cx(styles.button, {
          [styles.remove]: isRemove,
        })}
        onClick={onClick}
      >
        {buttonText}
      </button>
    </Tooltip>
  );
}

const AddFileButton = AddFile;

export default AddFileButton;
