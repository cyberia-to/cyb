import { Pane, Text } from '@cybercongress/gravity';
import { ProposalStatus } from 'cosmjs-types/cosmos/gov/v1beta1/gov';
import { BASE_DENOM } from 'src/constants/config';
import { formatNumber } from '../../utils/search/utils';
import Tooltip from '../tooltip/tooltip';
import styles from './styles.module.scss';

const submitted = require('../../image/ionicons_svg_ios-battery-full.svg');
const voting = require('../../image/ionicons_svg_ios-people.svg');
const passed = require('../../image/ionicons_svg_ios-checkmark-circle.svg');
const rejected = require('../../image/ionicons_svg_ios-close-circle.svg');
const failed = require('../../image/ionicons_svg_ios-remove-circle.svg');
const defaultIcon = require('../../image/ionicons_svg_ios-warning.svg');

const toFixedNumber = (number, toFixed) => {
  return Math.floor(number * 10 ** toFixed) / 10 ** toFixed;
};

// function Legend({ color, text, ...props }) {
//   return (
//     <Pane display="flex" alignItems="center" {...props}>
//       <Pane
//         width={12}
//         height={12}
//         borderRadius="2px"
//         display="inline-block"
//         marginRight={10}
//         backgroundColor={color}
//         verticalalign="middle"
//       />
//       <Text color="#fff" fontSize="16px">
//         {text}
//       </Text>
//     </Pane>
//   );
// }

export const Votes = ({ finalVotes }) => {
  try {
    return (
      <div className={styles.votesContainer}>
        <div
          className={`${styles.voteSection} ${styles.voteYes}`}
          style={{ width: `${toFixedNumber(finalVotes.yes, 2)}%` }}
        >
          <Tooltip
            placement="top"
            tooltip={<div>Yes: {toFixedNumber(finalVotes.yes, 2)}%</div>}
            contentStyle={{ flexGrow: 1 }}
          />
        </div>
        <div
          className={`${styles.voteSection} ${styles.voteAbstain}`}
          style={{ width: `${toFixedNumber(finalVotes.abstain, 2)}%` }}
        >
          <Tooltip
            tooltip={`Abstain: ${toFixedNumber(finalVotes.abstain, 2)}%`}
            placement="top"
            contentStyle={{ flexGrow: 1 }}
          />
        </div>
        <div
          className={`${styles.voteSection} ${styles.voteNo}`}
          style={{ width: `${toFixedNumber(finalVotes.no, 2)}%` }}
        >
          <Tooltip
            tooltip={`No: ${toFixedNumber(finalVotes.no, 2)}%`}
            placement="top"
            contentStyle={{ flexGrow: 1 }}
          />
        </div>
        <div
          className={`${styles.voteSection} ${styles.voteNoWithVeto}`}
          style={{ width: `${toFixedNumber(finalVotes.noWithVeto, 2)}%` }}
        >
          <Tooltip
            tooltip={`No With Veto: ${toFixedNumber(finalVotes.noWithVeto, 2)}%`}
            placement="top"
            contentStyle={{ flexGrow: 1 }}
          />
        </div>
      </div>
    );
  } catch (_error) {
    return (
      <div className={styles.votesContainer}>
        <div className={styles.voteSection} style={{ width: '0%' }}>
          <Tooltip placement="top" tooltip="Yes: 0%" />
        </div>
        <div className={styles.voteSection} style={{ width: '0%' }}>
          <Tooltip tooltip="Abstain: 0%" placement="top" />
        </div>
        <div className={styles.voteSection} style={{ width: '0%' }}>
          <Tooltip tooltip="No: 0%" placement="top" />
        </div>
        <div className={styles.voteSection} style={{ width: '0%' }}>
          <Tooltip tooltip="No With Veto: 0%" placement="top" />
        </div>
      </div>
    );
  }
};

export function IconStatus({ status, size, text, ...props }) {
  let imgIcon;
  let statusText = '';

  switch (status) {
    case ProposalStatus.PROPOSAL_STATUS_DEPOSIT_PERIOD: {
      imgIcon = submitted;
      statusText = 'deposit period';
      break;
    }
    case ProposalStatus.PROPOSAL_STATUS_VOTING_PERIOD: {
      imgIcon = voting;
      statusText = 'voting period';
      break;
    }
    case ProposalStatus.PROPOSAL_STATUS_PASSED: {
      imgIcon = passed;
      statusText = 'passed';
      break;
    }
    case ProposalStatus.PROPOSAL_STATUS_REJECTED: {
      imgIcon = rejected;
      statusText = 'rejected';
      break;
    }
    case ProposalStatus.PROPOSAL_STATUS_FAILED: {
      imgIcon = failed;
      statusText = 'failed';
      break;
    }
    default: {
      imgIcon = defaultIcon;
    }
  }
  return (
    <Pane display="flex" alignItems="center" {...props}>
      <Pane marginRight={5} width={size || 20} height={size || 20}>
        <img src={imgIcon} alt="icon" />
      </Pane>
      {text && (
        <Text fontSize="16px" color="#fff">
          {statusText}
        </Text>
      )}
    </Pane>
  );
}

export function Deposit({ totalDeposit, minDeposit }) {
  let procentDeposit = 0;

  if (totalDeposit > minDeposit) {
    procentDeposit = (minDeposit / totalDeposit) * 100;
  } else {
    procentDeposit = (totalDeposit / minDeposit) * 100;
  }

  return (
    <Pane
      backgroundColor="#ffffff14"
      borderRadius={5}
      overflow="visible"
      height={10}
      width="100%"
      display="flex"
      position="relative"
    >
      <Pane
        backgroundColor="#3ab793"
        display="flex"
        height="100%"
        borderRadius={5}
        position="absolute"
        width={`${totalDeposit < minDeposit ? procentDeposit : 100}%`}
      >
        <Pane position="absolute" left="100%" top="20px">
          <Text
            whiteSpace="nowrap"
            color="#fff"
            position="relative"
            right="50%"
            width="100%"
            className="tooltip-text-deposit"
          >
            Total Deposit {formatNumber(totalDeposit)} {BASE_DENOM.toUpperCase()}
          </Text>
        </Pane>
      </Pane>

      {totalDeposit > minDeposit && (
        <Pane
          backgroundColor="#007bff"
          display="flex"
          height="100%"
          borderRadius={5}
          position="absolute"
          zIndex={1}
          right={0}
          width={`${100 - procentDeposit}%`}
        />
      )}
    </Pane>
  );
}

export function Item({ title, value, ...props }) {
  return (
    <Pane {...props} display="flex">
      <Text minWidth="150px" color="#fff" fontSize="16px">
        {title}:{' '}
      </Text>
      <Text color="#fff" fontSize="16px">
        {value}
      </Text>
    </Pane>
  );
}
