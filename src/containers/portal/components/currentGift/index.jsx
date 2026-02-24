import BigNumber from 'bignumber.js';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { PATTERN_CYBER } from 'src/constants/patterns';
import ContainerGradient from '../../../../components/containerGradient/ContainerGradient';
import { formatNumber } from '../../../../utils/utils';
import styles from './CurrentGift.module.scss';

const GIFT_ICON = '🎁';
const BOOT_ICON = '🟢';

const keyTable = [
  'Astronaut',
  'Average Citizens',
  'Cyberpunk',
  'Extraordinary Hacker',
  'Key Opinion Leader',
  'Devil',
  'Master of the Great Web',
  'Passionate Investor',
  'True Hero of the Great Web',
];

function ItemValue({ value, title }) {
  return (
    <div className={styles.containerItemValue}>
      <div className={styles.containerItemValueValue}>{value}</div>
      <div className={styles.containerItemValueTitle}>{title}</div>
    </div>
  );
}

function ItemTable({ title, value }) {
  return (
    <div className={styles.containerItemTable}>
      <div>{title}</div>
      <div>
        {formatNumber(value)} {BOOT_ICON}
      </div>
    </div>
  );
}

const formatBonus = (amount) => {
  if (typeof amount === 'number') {
    return new BigNumber(amount).dp(1, BigNumber.ROUND_FLOOR).toNumber();
  }
  return '';
};

function TableAllocation({ currentBonus, currentGift }) {
  const itemTable = useMemo(() => {
    if (currentGift?.details) {
      const { details } = currentGift;
      return keyTable.map((item) => {
        const value = details[item] ? details[item].gift * 10 ** 6 : 0;
        return (
          <ItemTable key={item} value={Math.floor(parseFloat(value) * currentBonus)} title={item} />
        );
      });
    }
    return keyTable.map((item) => <ItemTable key={item} value={0} title={item} />);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentGift, currentBonus]);

  return <div className={styles.containerTableAllocation}>{itemTable}</div>;
}

function CurrentGift({
  currentGift,
  currentBonus,
  stateOpen,
  selectedAddress,
  initStateCard,
  totalGiftClaimed,
  totalGiftAmount,
  title,
  valueTextResult,
}) {
  const useTotalAmountByBonus = useMemo(() => {
    if (
      totalGiftClaimed &&
      totalGiftClaimed !== null &&
      totalGiftAmount &&
      totalGiftAmount !== null
    ) {
      if (totalGiftAmount.claim === totalGiftClaimed.claim) {
        return totalGiftAmount.claim;
      }

      if (currentBonus?.current) {
        const unclaimedGift = totalGiftAmount.amount - totalGiftClaimed.amount;
        const unclaimedGiftByBonus = Math.floor(unclaimedGift * currentBonus.current);
        const totalGiftByBonus = unclaimedGiftByBonus + totalGiftClaimed.claim;
        return totalGiftByBonus;
      }
    }
    return null;
  }, [totalGiftClaimed, totalGiftAmount, currentBonus]);

  const useSelectCyber = useMemo(() => {
    return selectedAddress?.match(PATTERN_CYBER);
  }, [selectedAddress]);

  const useTitle = useMemo(() => {
    if (currentGift && currentGift !== null) {
      const { amount, claim } = currentGift;

      return (
        <div
          style={{
            display: 'flex',
            width: '100%',
            justifyContent: 'space-between',
            zIndex: '1',
            alignItems: 'center',
          }}
        >
          <div style={{ color: '#1fcbff' }}>
            {title} gift {GIFT_ICON}
            {/* {useSelectCyber ? 'all gift ' : 'gift'} */}
          </div>
          <div>
            {claim ? formatNumber(parseFloat(claim)) : formatNumber(parseFloat(amount))} {BOOT_ICON}
          </div>
        </div>
      );
    }
    return <div style={{ color: '#1fcbff' }}>no gift {GIFT_ICON}</div>;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentGift, title]);

  const useBaseGift = useMemo(() => {
    if (currentGift && currentGift !== null) {
      const { amount, baseAmount } = currentGift;
      if (baseAmount) {
        return formatNumber(parseFloat(baseAmount));
      }

      return formatNumber(parseFloat(amount));
    }

    return 0;
  }, [currentGift]);

  const useCurrentBonus = useMemo(() => {
    if (currentGift && currentGift !== null) {
      const { amount, claim } = currentGift;

      const curentBonus = new BigNumber(parseFloat(claim)).dividedBy(parseFloat(amount)).toNumber();

      if ((curentBonus === 1 || claim === undefined) && currentBonus?.current) {
        const bonusByAddress = new BigNumber(parseFloat(currentBonus.current)).toNumber();

        return bonusByAddress;
      }

      if (currentGift.address.match(PATTERN_CYBER) && useTotalAmountByBonus !== null) {
        const tempCurentBonus = new BigNumber(parseFloat(useTotalAmountByBonus))
          .dividedBy(parseFloat(amount))

          .toNumber();
        return tempCurentBonus;
      }

      if (Object.hasOwn(currentGift, 'multiplier')) {
        const temp2 = new BigNumber(parseFloat(currentGift.multiplier)).toNumber();
        return temp2;
      }

      return curentBonus;
    }

    return 1;
  }, [currentBonus, currentGift, useTotalAmountByBonus]);

  const useClaimableGift = useMemo(() => {
    if (currentGift && currentGift !== null) {
      const { amount, claim, address } = currentGift;
      if (amount === claim || claim === undefined) {
        return formatNumber(Math.floor(parseFloat(amount) * useCurrentBonus));
      }

      if (claim && address.match(PATTERN_CYBER) && useTotalAmountByBonus !== null) {
        return formatNumber(parseFloat(useTotalAmountByBonus));
      }

      if (claim) {
        return formatNumber(parseFloat(claim));
      }

      return formatNumber(parseFloat(amount));
    }
    return 0;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentGift, useCurrentBonus, useTotalAmountByBonus]);

  const useValueTextResult = useMemo(() => {
    if (selectedAddress?.match(PATTERN_CYBER) && valueTextResult) {
      return valueTextResult;
    }

    if (currentGift !== null) {
      const { isClaimed } = currentGift;

      if (isClaimed) {
        return 'claimed';
      }
    }
    return 'claimable';
  }, [currentGift, valueTextResult, selectedAddress]);

  const useTitleMain = useMemo(() => {
    if (currentGift !== undefined && currentGift !== null) {
      if (useSelectCyber) {
        return (
          <>
            All
            <span style={{ margin: '0px 5px' }}>{useValueTextResult}</span>
            gift {GIFT_ICON}
          </>
        );
      }
      return (
        <>
          <span style={{ textTransform: 'capitalize', marginRight: 5 }}>{useValueTextResult}</span>
          gift {GIFT_ICON}
        </>
      );
    }
    return <div>no gift {GIFT_ICON}</div>;
  }, [useValueTextResult, useSelectCyber, currentGift]);

  return (
    <ContainerGradient
      styleLampContent="green"
      initState={initStateCard}
      userStyleContent={{
        height: '485px',
      }}
      closedTitle={useTitle}
      title={useTitleMain}
      stateOpen={stateOpen}
    >
      <div className={styles.containerCurrentGift}>
        <div className={styles.containerCurrentGiftContaiterAmountGift}>
          {useBaseGift === 0 ? (
            <ItemValue value={`${useBaseGift} ${BOOT_ICON}`} title="base gift" />
          ) : (
            <>
              <ItemValue value={`${useBaseGift} ${BOOT_ICON}`} title="base gift" />
              *
              <ItemValue value={formatBonus(useCurrentBonus)} title="bonus" />
              =
              <ItemValue
                value={`${GIFT_ICON} ${useClaimableGift} ${BOOT_ICON}`}
                title={`${useValueTextResult} gift`}
              />
            </>
          )}
        </div>
        {useBaseGift !== 0 && (
          <>
            <div>
              You have been chosen for good deeds in the cyberverse on the 5th of November 2021:
            </div>
            <TableAllocation currentBonus={useCurrentBonus} currentGift={currentGift} />
          </>
        )}
        <div>
          Please check{' '}
          <Link to="/ipfs/QmPAi1h1rwWnHkNnxnHZg28eGivpUK8wy8eciqoPSR4PHv">details of the gift</Link>
        </div>
      </div>
    </ContainerGradient>
  );
}

export default CurrentGift;
