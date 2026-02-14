import { useRef, useState } from 'react';
import { Transition } from 'react-transition-group';
import cx from 'classnames';
import { Display, DisplayTitle } from 'src/components';
import ArrowToggle from 'src/components/ArrowToggle/ArrowToggle';
import styles from './TransitionContainer.module.scss';

function TransitionContainer({
  children,
  title,
  titleOptions,
  isOpenState = true,
}: {
  children: React.ReactNode;
  title: React.ReactNode;
  titleOptions?: React.ReactNode;
  isOpenState?: boolean;
}) {
  const ref = useRef(null as null | HTMLDivElement);
  const [isOpen, setIsOpen] = useState(isOpenState);

  const minHeight = ref.current?.clientHeight;

  const onClickBtnOpen = () => {
    setIsOpen((item) => !item);
  };

  return (
    <Display
      noPadding
      title={
        <DisplayTitle
          title={
            <div className={styles.titleContainer}>
              <span>
                {title}{' '}
                <button
                  type="button"
                  onClick={onClickBtnOpen}
                  className={styles.btnOpenIcon}
                >
                  <ArrowToggle isOpen={isOpen} />
                </button>
              </span>
              {titleOptions}
            </div>
          }
        />
      }
    >
      <Transition in={isOpen} timeout={300}>
        {(state) => {
          return (
            <div
              ref={ref}
              style={
                minHeight
                  ? {
                      height: `${isOpen ? minHeight : 0}px`,
                    }
                  : undefined
              }
              className={cx(
                styles.containerTransition,
                styles[`containerTransition${state}`]
                // styles[`length-${Object.keys(balance).length}`]
              )}
            >
              {children}
            </div>
          );
        }}
      </Transition>
    </Display>
  );
}

export default TransitionContainer;
