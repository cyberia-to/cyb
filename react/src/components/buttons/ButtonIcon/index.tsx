import cx from 'classnames';
import type { CSSProperties } from 'react';
import { $TsFixMe } from 'src/types/tsfix';
import Tooltip, { TooltipProps } from '../../tooltip/tooltip';
import styles from './ButtonIcon.module.scss';

export type Props = {
  img: $TsFixMe;
  active?: boolean;
  disabled?: boolean;
  text?: TooltipProps['tooltip'];
  placement?: TooltipProps['placement'];
  styleContainer?: $TsFixMe;
  style?: CSSProperties;
  onClick?: () => void;
};

function ButtonIcon({
  img,
  active,
  disabled,
  text,
  placement = 'top',
  styleContainer,
  onClick,
  ...props
}: Props) {
  const button = (
    <button
      type="button"
      className={cx(styles.container, {
        [styles.activeIcon]: active,
      })}
      disabled={disabled}
      onClick={onClick}
      {...props}
    >
      <img src={img} alt={`${text || 'icon'}`} />
    </button>
  );
  return (
    <div style={styleContainer}>
      {text ? (
        <Tooltip placement={placement} tooltip={<span>{text}</span>}>
          {button}
        </Tooltip>
      ) : (
        button
      )}
    </div>
  );
}

export default ButtonIcon;
