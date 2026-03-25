import cx from 'classnames';
import { ColorLamp } from '../types';
import styles from './DisplayTitle.module.scss';

type Props = {
  title: string | React.ReactNode;
  children?: React.ReactNode;
  color?: ColorLamp;

  // temp prop
  animationState?: string;

  // to remove padding
  inDisplay?: boolean;

  image?: React.ReactNode;
  isImgLarge?: boolean;
};

// TODO: need remove animation from this component
function DisplayTitle({
  title,
  children,
  color = 'white',
  inDisplay,
  image,
  isImgLarge,

  animationState: state = 'entered',
}: Props) {
  const content = (
    <div className={cx(styles.displayTitle, styles[color])}>
      <div className={cx(styles.displayTitleContent, styles[`displayTitleContent${state}`])}>
        {image && (
          <div
            className={cx(styles.imageWrapper, {
              [styles.large]: isImgLarge,
            })}
          >
            {image}
          </div>
        )}

        {title}

        {children && <div className={styles.content}>{children}</div>}
      </div>
    </div>
  );

  if (inDisplay) {
    return <div className={styles.noPaddingWrapper}>{content}</div>;
  }

  return content;
}

export default DisplayTitle;
