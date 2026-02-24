import cx from 'classnames';
import Tooltip from 'src/components/tooltip/tooltip';
import styles from './ButtonsGroup.module.scss';

type Props = {
  items: {
    label: string | JSX.Element;
    checked?: boolean;
    disabled?: boolean;
    name?: string;
    tooltip?: string;
  }[];
  onChange: (value: string) => void;
  type: 'radio' | 'checkbox';
};

function ButtonsGroup({ items, onChange, type }: Props) {
  return (
    <ul className={styles.wrapper}>
      {items.map((item, i) => (
        <li
          key={i}
          className={cx({
            [styles['--checked']]: item.checked,
            [styles['--disabled']]: item.disabled,
          })}
        >
          <Tooltip tooltip={!item.disabled && item.tooltip} placement="bottom" strategy="fixed">
            <label>
              <input
                type={type}
                name={item.name}
                checked={item.checked}
                onChange={() => !item.disabled && onChange(item.name || item.label)}
              />
              {item.label}
            </label>
          </Tooltip>
        </li>
      ))}
    </ul>
  );
}

export default ButtonsGroup;
