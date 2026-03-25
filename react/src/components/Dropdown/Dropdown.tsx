import cx from 'classnames';
import { useState } from 'react';
import styles from './Dropdown.module.scss';

type Props = {
  options: {
    label: string;
    value: string;
  }[];

  value: string;
  onChange: (value: string) => void;
};

function Dropdown({ options = [], value, onChange }: Props) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <div className={styles.dropdown}>
      {/*  REFACTOR: use Triangle component */}
      <button type="button" onClick={() => setIsOpen(!isOpen)}>
        {value
          ? options.find((option) => option.value === value)?.label
          : options[0]?.label || 'Select'}
      </button>

      {isOpen && (
        <ul>
          {options.map(({ label, value: val }) => {
            return (
              <li
                key={val}
                className={cx({
                  [styles.active]: val === value,
                })}
              >
                <button
                  type="button"
                  onClick={() => {
                    onChange(val);
                    setIsOpen(false);
                  }}
                >
                  {label}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export default Dropdown;
