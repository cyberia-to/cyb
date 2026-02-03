import { LinkWindow } from 'src/components';
import { linkTxOsmosis } from 'src/pages/Energy/utils/utils';
import { trimString } from 'src/utils/utils';
import cx from 'classnames';
import styles from './utils.module.scss';

export function LinkWindowOsmo({ to }: { to: string }) {
  const linkTo = linkTxOsmosis(to);

  return <LinkWindow to={linkTo}>{trimString(linkTo, 18, 4)}</LinkWindow>;
}

export function ColorText({
  text,
  color,
}: {
  text: string;
  color: 'red' | 'green' | 'yellow';
}) {
  return <span className={cx(styles.colorText, styles[color])}>{text}</span>;
}
