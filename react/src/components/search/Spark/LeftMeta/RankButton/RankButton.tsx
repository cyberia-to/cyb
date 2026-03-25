import cx from 'classnames';
import styles from './RankButton.module.scss';

function RankButton({ cid, rankSelected, handleRankClick }) {
  return (
    <button
      className={cx('hover-rank-contentItem', styles.rank, {
        [styles.rankSelected]: rankSelected,
      })}
      type="button"
      onClick={() => handleRankClick(cid)}
    >
      <svg
        width="26"
        height="24"
        viewBox="0 0 26 24"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          id="Vector"
          d="M13.8885 1.54119C13.7169 1.2088 13.3741 1 13 1C12.6259 1 12.2831 1.2088 12.1115 1.54119L8.94996 7.6638L1.86162 8.65421C1.47957 8.70759 1.16229 8.97596 1.04629 9.34386C0.930281 9.71177 1.03623 10.1136 1.31856 10.3765L6.4194 15.1257L5.21772 21.8234C5.15083 22.1962 5.30029 22.5748 5.6038 22.8013C5.9073 23.0279 6.31276 23.0636 6.65115 22.8935L13 19.7018L19.3488 22.8935C19.6872 23.0636 20.0927 23.0279 20.3962 22.8013C20.6997 22.5748 20.8492 22.1962 20.7823 21.8234L19.5806 15.1257L24.6814 10.3765C24.9638 10.1136 25.0697 9.71177 24.9537 9.34386C24.8377 8.97596 24.5204 8.70759 24.1384 8.65421L17.05 7.6638L13.8885 1.54119Z"
          stroke="#F62BFD"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>

      {/* <Rank
          hash={cid}
          rank={exponentialToDecimal(
            parseFloat(itemData.rank || rankInfo?.rank).toPrecision(3)
          )}
          grade={itemData?.grade || rankInfo?.grade}
          onClick={() => handleRankClick(cid)}
        /> */}
    </button>
  );
}

export default RankButton;
