/* eslint-disable react-hooks/rules-of-hooks */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import useMediaQuery from '../../../hooks/useMediaQuery';
import TabItem, { Position } from '../TabItem/TabItem';
import styles from './Carousel.module.scss';

const cx = require('classnames');

interface Slide {
  step?: number;
  title?: string | JSX.Element;
}

type Color = 'green' | 'blue';

type CarouselProps = {
  slides: Slide[];
  // speed?: number;
  activeStep?: number;
  // setStep: (step: number) => void;
  // transitionSpeed?: number;
  // slideWidth?: number;
  heightSlide?: string;
  // disableNext?: boolean;
  disableMode?: boolean;
  displaySlide?: number;
  color?: Color;
  noAnimation?: boolean;
  onChange: (index: number) => void;
};

function Carousel({
  slides = [],
  speed = 3000,
  activeStep = 1,
  setStep,
  transitionSpeed = 500,
  slideWidth = 200,
  heightSlide,
  disableNext,
  onChange,
  noAnimation,
  color = 'green',
  disableMode,
  displaySlide = 3,
}: CarouselProps) {
  const query = useMediaQuery('(min-width: 768px)');
  const [itemWidth, setItemWidth] = useState(0);
  const [displaySlideState, setDisplaySlideState] = useState(displaySlide);
  const [visibleSlide, setVisibleSlide] = useState(1);
  const [hasTransitionClass, setHasTransitionClass] = useState(!noAnimation);
  const changeDisplay = useRef(false);

  useEffect(() => {
    if (displaySlide > 3) {
      if (!query) {
        setDisplaySlideState(3);
        changeDisplay.current = true;
      }
      if (query && changeDisplay.current) {
        setDisplaySlideState(displaySlide);
        changeDisplay.current = false;
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, displaySlide]);

  if (slides.length < 2) {
    console.error('Please provide more slides');
    return null;
  }

  useEffect(() => {
    setVisibleSlide(slides.length * 2 + activeStep);

    const resizeCarousel = () => {
      const { clientWidth } = document.getElementById('containerCarousel');

      setItemWidth(clientWidth / displaySlideState);
    };

    resizeCarousel();
    window.addEventListener('resize', resizeCarousel);

    return () => {
      window.removeEventListener('resize', resizeCarousel);
    };
  }, [activeStep, slides, displaySlideState]);

  const calculateLeftMargin = useMemo(() => {
    if (itemWidth !== 0) {
      return `-${visibleSlide * itemWidth - itemWidth * Math.floor(displaySlideState / 2)}px`;
    }
    return '0px';
  }, [visibleSlide, itemWidth, displaySlideState]);

  useEffect(() => {
    if (noAnimation) {
      return;
    }

    // make carousel infinite
    if (visibleSlide > slides.length * 2) {
      // keep index near the middle of the list when moving left
      // setNavDisabled(true); // disable nav buttons while silently moving to different card
      setTimeout(() => {
        // disable transition animation after card slides, then snap to same card prior in the list
        setHasTransitionClass(false);
        setVisibleSlide(visibleSlide - slides.length);
      }, 500);
    }
    // eslint-disable-next-line eqeqeq
    if (visibleSlide === slides.length) {
      // keep index near the middle of the list when moving right
      // setNavDisabled(true);
      setTimeout(() => {
        setHasTransitionClass(false);
        setVisibleSlide(visibleSlide + slides.length);
      }, 500);
    }
    if (hasTransitionClass === false) {
      // turn transition animation back on after 25ms
      setTimeout(() => {
        setHasTransitionClass(true);
        // setNavDisabled(false);
      }, 500 / 10);
    }
  }, [visibleSlide, noAnimation, hasTransitionClass, slides.length]);

  const setActiveItem = useCallback(
    (index: number) => {
      if (!disableMode) {
        setVisibleSlide(index);
      }
    },
    [disableMode]
  );

  const newItemList = slides.concat(slides, slides, slides);

  return (
    <div
      className={cx(styles.carousel, styles[`color_${color}`])}
      id="containerCarousel"
      style={{
        height: heightSlide || '42px',
      }}
    >
      <div className={styles.slidesContainer} style={{ width: `${itemWidth}px` }}>
        <div
          id="slides"
          className={cx(styles.slides, {
            [styles.transition]: hasTransitionClass,
          })}
          style={{ left: calculateLeftMargin }}
        >
          {newItemList.map((slide, index) => {
            let typeTab: Position | undefined;
            if (index + Math.floor(displaySlideState / 2) === visibleSlide) {
              typeTab = Position.Left;
            }
            if (index - Math.floor(displaySlideState / 2) === visibleSlide) {
              typeTab = Position.Right;
            }
            return (
              // eslint-disable-next-line jsx-a11y/click-events-have-key-events, jsx-a11y/no-static-element-interactions
              <TabItem
                type={typeTab}
                key={index}
                disable={disableMode}
                onClick={() => {
                  setActiveItem(index);
                  onChange?.(slides.indexOf(slide));
                }}
                isSelected={index === visibleSlide}
                text={slide.title || ''}
                style={{
                  width: `${itemWidth}px`,
                  height: heightSlide || '42px',
                }}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}

export default Carousel;
