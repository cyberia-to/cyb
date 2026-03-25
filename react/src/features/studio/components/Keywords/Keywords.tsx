import cx from 'classnames';
import linkImg from 'components/search/Spark/Meta/Links/link.svg';
import arrowImg from 'images/arrow-left-img.svg';
import { KeywordsItem, useStudioContext } from '../../studio.context';
import KeywordButton from '../KeywordButton/KeywordButton';
import styles from './Keywords.module.scss';

type Props = {
  items: KeywordsItem[];
  type: 'from' | 'to';
  onClickAddBtn: () => void;
};

function Keywords({ items, type, onClickAddBtn }: Props) {
  const { removeKeywords } = useStudioContext();

  const renderItem = items.map((item, index) => {
    return (
      <KeywordButton
        text={item.text}
        key={index}
        className={styles.overflowPill}
        onClick={() => removeKeywords(type, item.cid)}
        isKeyword
        tooltip="delete keyword"
      />
    );
  });

  return (
    <div className={styles.wrapper}>
      <div
        className={cx(styles.tagContainer, {
          [styles.reverse]: type === 'from',
        })}
      >
        <img src={linkImg} alt="linkImg" className={styles.linkImg} />{' '}
        <img src={arrowImg} alt="arrow" className={styles.arrow} />{' '}
      </div>
      <div className={styles.containerPill}>
        {renderItem}

        <KeywordButton text="+" onClick={onClickAddBtn} tooltip={`add keywords ${type}`} />
      </div>
    </div>
  );
}

export default Keywords;
