import cx from 'classnames';
import SliderComponent, { SliderProps as RcSliderProps } from 'rc-slider';
import React, { useCallback, useEffect, useRef, useState } from 'react';
import imgSwap from 'src/image/exchange-arrows.svg';
import 'rc-slider/assets/index.css';
import styles from './Slider.module.scss';
import './styles.override.css';
import DenomArr from '../denom/denomArr';
import FormatNumberTokens from '../FormatNumberTokens/FormatNumberTokens';

function SpetionLabel({ value }: { value: number }) {
  let position = '';

  if (value <= 5) {
    position = 'left';
  } else if (value === 10) {
    position = 'center';
  } else {
    position = 'right';
  }
  return (
    // eslint-disable-next-line jsx-a11y/label-has-associated-control
    <label className={styles.trackLabel}>
      <div className={styles.trackMarkBgBlur} />
      <div className={cx(styles.trackMarkGradient, styles[`trackMarkGradient${position}`])} />

      <div
        className={cx(styles.trackMarkLabel, {
          [styles.trackMarkLabelColorBlue]: value === 100,
        })}
      >
        {value < 100 ? `${value}%` : 'Max'}
      </div>
    </label>
  );
}

function SphereValue({ angle, children }: { angle: number; children: React.ReactNode }) {
  return (
    <div className={cx(styles.debtAmountPosToken)} style={{ transform: `rotate(${angle}deg)` }}>
      <div
        className={styles.debtAmountPosTokenObj}
        style={{ transform: `rotate(${angle * -1}deg)` }}
      >
        {children}
      </div>
    </div>
  );
}

const SphereValueMemo = React.memo(SphereValue);

function SphereValueWithToken({
  tokenName,
  angle,
  price,
}: {
  tokenName: string;
  angle: number;
  price?: number;
}) {
  return (
    <SphereValueMemo angle={angle}>
      <DenomArr denomValue={tokenName} onlyImg tooltipStatusImg={false} />
      {price && (
        <div className={styles.imgValue}>
          <FormatNumberTokens value={price} />
        </div>
      )}
    </SphereValueMemo>
  );
}

type TokenPair = {
  tokenA: string;
  tokenB: string;
  priceA: number;
  priceB: number;
};

const angleDeg = 135;

const scaleMin = 1;
const scaleMax = 101;
const minlVal = Math.log(scaleMin);
const maxlVal = Math.log(scaleMax);

const scale = (maxlVal - minlVal) / scaleMax;

const scalePercents = [1, 2, 5, 10, 20, 50, 100];

function roundToOneDecimalPlace(num: number) {
  return Math.round(num * 10) / 10;
}
const positionToPercents = (position: number) =>
  position === 0 ? 0 : roundToOneDecimalPlace(Math.exp(minlVal + scale * position)) - 1;

const percentsToPosition = (percents: number) => (Math.log(percents + 1) - minlVal) / scale;

const scaleMarks = Object.fromEntries(
  scalePercents.map((percents) => [
    percentsToPosition(percents),
    <SpetionLabel key={`slider_mark_${percents}`} value={percents} />,
  ])
);

export type SliderProps = {
  onChange: (value: number) => void;
  onSwapClick?: () => void;
  valuePercents: number;
  disabled?: boolean;
  tokenPair?: TokenPair;
  text?: string | React.ReactNode;
};

function Slider({ onChange, onSwapClick, valuePercents, disabled, tokenPair, text }: SliderProps) {
  const [valueSilder, setValueSilder] = useState(0);
  const [currentPercents, setCurrentPercent] = useState(0);
  const [draggingMode, setDraggingMode] = useState(false);
  const [isHadleFocused, setIsHandleFocused] = useState(false);
  const draggingDetectorTimer = useRef(undefined);

  const finishDragging = useCallback(() => {
    clearTimeout(draggingDetectorTimer.current);
    if (!draggingMode) {
      onSwapClick?.();
    }

    setDraggingMode(false);
  }, [draggingMode, onSwapClick]);

  const startDragging = () => {
    draggingDetectorTimer.current = setTimeout(() => {
      clearTimeout(draggingDetectorTimer.current);
      setDraggingMode(true);
    }, 300);
  };

  useEffect(() => {
    const percents = valuePercents;

    if (percents > 100) {
      setCurrentPercent(100);
      setValueSilder(scaleMax);
      return;
    }
    if (percents <= 0) {
      setValueSilder(scaleMin);
      setCurrentPercent(0);
      return;
    }

    setCurrentPercent(percents);

    const position = percentsToPosition(percents);

    setValueSilder(position < 0 ? 0 : position);
  }, [valuePercents]);

  const onSliderChange = (position: number) => {
    // Hack to avoid glitch when click on handle
    if (isHadleFocused && !draggingMode) {
      return;
    }
    setDraggingMode(true);
    setValueSilder(position);
    requestAnimationFrame(() => {
      const value = positionToPercents(position);
      setCurrentPercent(value);
      onChange?.(value);
    });
  };

  const renderCustomHandle: RcSliderProps['handle'] = useCallback(
    ({ value }) => {
      const percents =
        currentPercents < 2 ? currentPercents.toFixed(1) : Math.round(currentPercents || 0);
      return (
        // eslint-disable-next-line jsx-a11y/no-static-element-interactions
        <div
          className={styles.debtAmountPos}
          style={{
            left: `${value - scaleMin}%`,
          }}
          onMouseEnter={() => setIsHandleFocused(true)}
          onMouseLeave={() => setIsHandleFocused(false)}
          onMouseDown={() => startDragging()}
          onTouchStart={() => startDragging()}
          onMouseUp={() => finishDragging()}
          onTouchEnd={() => finishDragging()}
        >
          <SphereValueMemo angle={90}>
            <div>{percents}%</div>
          </SphereValueMemo>
          {tokenPair && (
            <>
              <SphereValueWithToken
                tokenName={tokenPair.tokenA}
                angle={angleDeg}
                price={tokenPair.priceA}
              />
              <SphereValueWithToken
                tokenName={tokenPair.tokenB}
                angle={-angleDeg}
                price={tokenPair.priceB}
              />
            </>
          )}
          <button type="button" className={styles.buttonIcon}>
            <img src={imgSwap} alt="swap" draggable="false" />
          </button>
        </div>
      );
    },
    [tokenPair, currentPercents, finishDragging, startDragging]
  );

  return (
    <div className={styles.formWrapper}>
      <div className={styles.debtAmountSlider}>
        <div style={{ width: '100%', padding: '0 25px' }}>
          <SliderComponent
            disabled={disabled}
            value={valueSilder}
            min={scaleMin}
            max={scaleMax}
            step={1}
            handle={renderCustomHandle}
            onChange={(pos) => onSliderChange(pos)}
            marks={scaleMarks}
            trackStyle={{ backgroundColor: '#C5C5C5', height: '2px' }}
            railStyle={{ backgroundColor: '#C5C5C5', height: '2px' }}
            dotStyle={{
              background: 'none',
              border: 'none',
            }}
          />
          {text && (
            <div className={cx(styles.text, valueSilder > 80 && styles.textLeft)}>{text}</div>
          )}
        </div>
      </div>
    </div>
  );
}

export default Slider;
