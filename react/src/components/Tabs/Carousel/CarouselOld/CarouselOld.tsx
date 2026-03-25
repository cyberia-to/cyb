/* eslint-disable react-hooks/rules-of-hooks */
import { useCallback, useEffect, useState } from 'react';
import TabItem, { Position } from '../../TabItem/TabItem';
import styles from './CarouselOld.module.scss';

const cx = require('classnames');

type Slide = {
  step?: number;
  title?: string;
};

export type Props = {
  slides: Slide[];
  speed?: number;
  activeStep: number;
  setStep: (index: number) => void;
  transitionSpeed?: number;
  slideWidth?: number;
  disableNext?: boolean;
  disableMode?: boolean;
  heightSlide?: string;
};

/**
 * @deprecated use Carousel
 */
function Carousel({
  slides = [],
  speed = 3000,
  activeStep,
  setStep,
  transitionSpeed = 500,
  slideWidth = 200,
  disableNext,
  disableMode,
  heightSlide,
}: Props) {
  if (slides.length < 2) {
    console.error('Please provide more slides');
    return null;
  }

  const [visibleSlide, setVisibleSlide] = useState(1);
  const [stateSlides, setStateSlides] = useState(slides);
  const [leftAndRightDisabled, setLeftAndRightDisabled] = useState(false);

  useEffect(() => {
    setVisibleSlide(activeStep);
  }, [activeStep]);

  // useEffect with an empty array as the second parameter
  // will run only once, when the component mounts
  // this makes it an ideal place to trigger this functionality
  useEffect(() => {
    const slidesWithClones = [...slides];
    slidesWithClones.unshift({});
    slidesWithClones.push({});
    setStateSlides(slidesWithClones);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [slides]);

  // Whenever the left and right arrows are disabled
  // We want to enable them again after a specific
  // period of time, this is to prevent problematic
  // spamming of these controls during our clone
  // slide-cloning/swapping mechanism
  // Probably a better way to handle this though
  useEffect(() => {
    if (leftAndRightDisabled) {
      setTimeout(() => {
        setLeftAndRightDisabled(false);
      }, transitionSpeed * 2);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [leftAndRightDisabled, transitionSpeed]);

  const calculateLeftMargin = () => {
    return `-${visibleSlide * slideWidth - slideWidth}px`;
  };

  const slideDimensionStyles = () => {
    return { width: `${slideWidth}px` };
  };

  const setActiveItem = useCallback(
    (index: number) => {
      if (index !== 0 && index <= slides.length && !disableMode) {
        // setVisibleSlide(index);
        if (index <= visibleSlide) {
          setStep(index);
        } else if (!disableNext && index > visibleSlide) {
          setStep(index);
        }
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [disableNext, visibleSlide, disableMode, setStep, slides.length]
  );

  return (
    <div
      className={styles.carousel}
      style={{ maxWidth: `${slideWidth * 3}px`, height: heightSlide || '42px' }}
    >
      <div
        className={styles.slidesContainer}
        // style={slideDimensionStyles()}
      >
        <div
          id="slides"
          className={cx(styles.slides, styles.transition)}
          style={{ left: calculateLeftMargin() }}
        >
          {stateSlides.map((slide, index) => {
            let typeTab: Position | undefined;

            if (index + 1 === visibleSlide) {
              typeTab = Position.Left;
            }
            if (index - 1 === visibleSlide) {
              typeTab = Position.Right;
            }

            return (
              <TabItem
                type={typeTab}
                key={index}
                onClick={() => setActiveItem(index)}
                isSelected={index === visibleSlide}
                text={slide.title || ''}
                step={slide.step}
                style={slideDimensionStyles()}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default Carousel;
