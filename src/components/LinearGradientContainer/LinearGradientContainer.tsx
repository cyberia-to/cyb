import cx from 'classnames';
import styles from './LinearGradientContainer.module.scss';

export const enum Color {
  Pink = 'pink',
  Yellow = 'yellow',
  Red = 'red',
  Black = 'black',
  Green = 'green',
  Blue = 'blue',
}

export type Props = {
  active?: boolean;
  color?: Color;
  title?: string;
  children: React.ReactNode;
};

function LinearGradientContainer({ active, color, title, children }: Props) {
  return (
    <div className={styles.wrapper}>
      <div
        className={cx(styles.textbox, color && styles[color], {
          [styles.active]: active,
        })}
      >
        {children}
        <div className={cx(styles.textboxFace, styles.textboxBottomGradient)} />
        <div className={cx(styles.textboxFace, styles.textboxBottomLine)} />
      </div>
      {title && <p>{title}</p>}
    </div>
  );
}

export default LinearGradientContainer;
