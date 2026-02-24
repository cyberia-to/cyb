/* eslint-disable no-nested-ternary */
/* eslint-disable jsx-a11y/click-events-have-key-events */
/* eslint-disable jsx-a11y/no-static-element-interactions */

import classNames from 'classnames';
import React, { useMemo, useRef, useState } from 'react';
import { $TsFixMe, $TsFixMeFunc } from 'src/types/tsfix';
import useOnClickOutside from '../../hooks/useOnClickOutside';
import LinearGradientContainer, { Color } from '../LinearGradientContainer/LinearGradientContainer';
import styles from './Select.module.scss';
import { SelectContext, useSelectContext } from './selectContext';

type OptionSelectProps = {
  text: React.ReactNode;
  value: string;
  img?: React.ReactNode;
  bgrImg?: boolean;
};

export function OptionSelect({ text, img, bgrImg, value }: OptionSelectProps) {
  const { changeSelectedOption } = useSelectContext();
  return (
    <div className={styles.listItem} onClick={() => changeSelectedOption(value)}>
      {(bgrImg || img) && (
        <div
          className={styles.bgrImg}
          style={bgrImg ? { boxShadow: 'none', background: 'transparent' } : {}}
        >
          {img}
        </div>
      )}
      <div>{text || '-'}</div>
    </div>
  );
}

export type SelectOption = {
  text: string | JSX.Element;
  value: string;
  img?: React.ReactNode;
};

type SelectProps = {
  valueSelect: $TsFixMe;
  onChangeSelect?: $TsFixMeFunc;
  children?: React.ReactNode;
  width?: string | number;
  disabled?: boolean;
  options?: SelectOption[];
  currentValue: React.ReactNode;
  color?: Color;
  title?: string;
  small?: boolean;
};

function Select({
  valueSelect,
  onChangeSelect,
  children,
  width,
  disabled,
  options,
  currentValue,
  small,
  color = Color.Yellow,
  title,
}: SelectProps) {
  const selectContainerRef = useRef<HTMLDivElement>(null);
  const [isOpen, setIsOpen] = useState(false);
  const toggling = () => {
    if (!disabled) {
      setIsOpen((item) => !item);
    }
  };

  const clickOutsideHandler = () => setIsOpen(false);

  const updateSelectedOption = (value: SelectOption['value']) => {
    if (value === currentValue?.value) {
      return;
    }

    onChangeSelect?.(value);
    setIsOpen(false);
  };

  useOnClickOutside(selectContainerRef, clickOutsideHandler);

  const { current: refCurrent } = selectContainerRef;
  const isDropUp = useMemo(() => {
    if (refCurrent) {
      // maybe improve
      const { bottom } = refCurrent.getBoundingClientRect();
      const spaceBelow = window.innerHeight - bottom;
      return spaceBelow < window.innerHeight / 2;
    }
    return false;
  }, [refCurrent]);

  function renderTitle() {
    let value = currentValue;
    if (valueSelect !== undefined && options) {
      const item = options.find((item) => item.value === valueSelect);

      if (item) {
        value = (
          <>
            {item.img} {item.text}
          </>
        );
      }
    }

    return value;
  }

  return (
    <SelectContext.Provider
      // eslint-disable-next-line react/jsx-no-constructed-context-values
      value={{
        selectedOption: valueSelect,
        changeSelectedOption: updateSelectedOption,
      }}
    >
      <div
        style={{ width: width || '120px' }}
        className={classNames(styles.dropDown, {
          [styles.small]: small,
          [styles.disabled]: disabled,
        })}
        ref={selectContainerRef}
      >
        <div className={styles.dropDownContainer}>
          <button type="button" onClick={toggling} className={styles.dropDownContainerHeader}>
            <LinearGradientContainer
              active={isOpen}
              color={disabled ? Color.Black : color}
              title={title}
            >
              <span className={styles.dropDownHeader}>{renderTitle()}</span>
            </LinearGradientContainer>
          </button>
          {isOpen && (
            <div
              className={classNames(styles.dropDownListContainer, {
                [styles.dropUp]: isDropUp,
              })}
            >
              <div className={styles.dropDownList}>
                {/* {placeholder && (
                  <OptionSelect text={placeholder} value={null} />
                )} */}
                {options
                  ? options.map((option) => {
                      const { value, text, img } = option;
                      return <OptionSelect key={value} text={text} value={value} img={img} />;
                    })
                  : children}
              </div>

              <div className={classNames(styles.ListContainer, styles.dropDownBottomLine)} />
            </div>
          )}
        </div>
      </div>
    </SelectContext.Provider>
  );
}

export default Select;
