import cx from 'classnames';
import React, { useEffect, useState } from 'react';
import TextareaAutosize from 'react-textarea-autosize';
import LinearGradientContainer, { Color } from '../LinearGradientContainer/LinearGradientContainer';
import styles from './Input.module.scss';

export type Props = {
  color?: Color;
  width?: string;
  title?: string;
  className?: string;
  classNameTextbox?: string;
  focusedProps?: boolean;
  isTextarea?: boolean;
  type?: 'text' | 'password';
  error?: string;
  value: string;
  maxValue?: number;
  onChange?: (e: React.ChangeEvent<HTMLInputElement>) => void;
  onBlurFnc?: () => void;
  onFocusFnc?: () => void;
  disabled?: boolean;
} & React.InputHTMLAttributes<HTMLInputElement>;

const dataPercentAttribute = 'data-percent';

const Input = React.forwardRef<HTMLInputElement, Props>(
  (
    {
      color,
      placeholder,
      onChange,
      title,
      width,
      type = 'text',
      value,
      autoFocus,
      disabled,
      className,
      classNameTextbox,
      focusedProps,
      maxValue,
      isTextarea,
      onBlurFnc,
      onFocusFnc,
      error,
      ...props
    },
    ref
  ) => {
    const [focused, setFocused] = useState(focusedProps || false);

    useEffect(() => {
      if (!ref?.current) {
        return;
      }

      if (focused) {
        ref.current.focus();
      } else {
        ref.current.blur();
      }
    }, [focused, ref]);

    useEffect(() => {
      if (typeof focusedProps === 'boolean') {
        setFocused(focusedProps);
      }
    }, [focusedProps]);

    const Tag = isTextarea ? TextareaAutosize : 'input';

    const handlerOnFocused = () => {
      setFocused(true);

      if (onFocusFnc) {
        onFocusFnc();
      }
    };

    const handlerOnBlur = () => {
      setFocused(false);

      if (onBlurFnc) {
        onBlurFnc();
      }
    };

    function handleMax(e: React.MouseEvent<HTMLButtonElement, MouseEvent>) {
      const target = e.target as HTMLButtonElement;
      const percent = target.getAttribute(dataPercentAttribute);
      const newValue = ((maxValue! * Number(percent)) / 100).toFixed(0);

      // FIXME: need refactor other components
      onChange({
        target: {
          value: String(newValue),
          // issue with number input
          focus() {},
        },
      });
    }

    return (
      <div
        className={cx(
          styles.textbox,
          focused && styles.focused,
          isTextarea && styles.textarea,
          classNameTextbox
        )}
        style={{ width }}
      >
        <LinearGradientContainer
          active={focusedProps || focused}
          color={disabled ? Color.Black : color}
          title={title}
        >
          {focused && !!maxValue && (
            <div className={styles.amount}>
              {[25, 50, 75, 100].map((percent) => {
                return (
                  <button
                    key={percent}
                    type="button"
                    // onMouseDown be called before onBlur
                    onMouseDown={handleMax}
                    {...{ [dataPercentAttribute]: percent }}
                  >
                    {percent === 100 ? 'max' : `${percent}%`}
                  </button>
                );
              })}
            </div>
          )}

          <Tag
            type={type}
            ref={ref}
            value={value}
            className={className}
            onChange={onChange}
            disabled={disabled}
            // eslint-disable-next-line jsx-a11y/no-autofocus
            autoFocus={autoFocus}
            onFocus={handlerOnFocused}
            onBlur={handlerOnBlur}
            placeholder={placeholder}
            autoComplete="off"
            {...props}
          />
        </LinearGradientContainer>
        {error && <div className={styles.error}>{error}</div>}
      </div>
    );
  }
);

Input.displayName = 'Input';

export default Input;
