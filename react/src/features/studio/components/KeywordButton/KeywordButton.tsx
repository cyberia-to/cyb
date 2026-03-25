import cx from 'classnames';
import { ReactNode } from 'react';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import styles from './KeywordButton.module.scss';

type Props = {
  text: ReactNode;
  onClick: () => void;
  tooltip?: string;
  isKeyword?: boolean;
  className?: string;
};

function KeywordButton({ text, onClick, tooltip, isKeyword, className }: Props) {
  return (
    <AdviserHoverWrapper adviserContent={tooltip || ''}>
      <button
        className={cx(styles.keywordBtn, className, {
          [styles.isKeyword]: isKeyword,
        })}
        onClick={onClick}
        type="button"
      >
        {text}
      </button>
    </AdviserHoverWrapper>
  );
}

export default KeywordButton;
