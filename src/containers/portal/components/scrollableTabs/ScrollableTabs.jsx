import { useCallback, useState } from 'react';
import { CSSTransition, TransitionGroup as ReactCSSTransitionGroup } from 'react-transition-group';
import styles from './ScrollableTabs.scss';

const cx = require('classnames');

function Item({ level, id, ...props }) {
  const className = `level${level}`;
  return (
    <div {...props} className={cx(styles.item, styles[className])}>
      <div className={cx(styles.itemLamp, styles[`itemLamp${className}`])} />
      {id}
    </div>
  );
}

function ScrollableTabs({ items, active, setStep }) {
  const [direction, setDirection] = useState('');
  // eslint-disable-next-line unused-imports/no-unused-vars
  const [_activeItem, setActiveItem] = useState(active);

  // useEffect(() => {
  //   setActiveItem(active);
  // }, [active]);

  // useEffect(() => {
  //   setStep(activeItem);
  // }, [setStep, activeItem]);

  const generateItems = () => {
    const itemsTabs = [];
    let level;
    console.log('activeItem', active);
    for (let i = active - 1; i < active + 2; i++) {
      let index = i;
      let funcOnClick;
      if (i < 0) {
        index = Object.keys(items).length + i;
      } else if (i >= Object.keys(items).length) {
        index = i % Object.keys(items).length;
      }
      level = active - i;
      if (level < 0) {
        funcOnClick = moveRight;
      }
      if (level > 0) {
        funcOnClick = moveLeft;
      }
      itemsTabs.push(
        <CSSTransition
          key={index}
          classNames={{
            enter: styles[`${direction}-enter`],
            enterActive: styles[`${direction}-enter-active`],
            leave: styles[`${direction}-leave`],
            leaveActive: styles[`${direction}-leave-active`],
            appear: styles[`${direction}-appear`],
            appearActive: styles[`${direction}-appear-active`],
          }}
        >
          <Item key={index} id={items[index]} level={level} onClick={funcOnClick} />
        </CSSTransition>
      );
    }
    console.log('itemsTabs', itemsTabs);
    return itemsTabs;
  };

  const moveLeft = useCallback(() => {
    let newActive = active;
    newActive -= 1;
    setActiveItem(newActive < 0 ? Object.keys(items).length - 1 : newActive);
    setDirection('left');
  }, [items, active]);

  const moveRight = useCallback(() => {
    const newActive = active;
    setActiveItem((newActive + 1) % Object.keys(items).length);
    setDirection('right');
  }, [items, active]);

  return (
    <div className={cx(styles.noselect, styles.carousel)}>
      <ReactCSSTransitionGroup>{generateItems()}</ReactCSSTransitionGroup>
    </div>
  );
}

export default ScrollableTabs;
