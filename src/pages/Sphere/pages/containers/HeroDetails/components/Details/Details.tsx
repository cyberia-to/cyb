import Display from 'src/components/containerGradient/Display/Display';
import { Account, LinkWindow, Row, RowsContainer } from 'src/components';
import { formatNumber, fromBech32, trimString } from 'src/utils/utils';
import { Validator } from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';
import Pill, { Props as PropsPill } from 'src/components/Pill/Pill';
import React from 'react';

import styles from './Details.module.scss';
import big from '../../utils/big';

const dateFormat = require('dateformat');

function Col({
  title,
  value,
  color,
}: {
  title: string;
  value: React.ReactNode;
  color?: PropsPill['color'];
}) {
  return (
    <Pill
      color={color}
      text={
        <span className={styles.containerColItem}>
          <span className={styles.containerColItemValue}>{value}</span>
          <span className={styles.containerColItemTitle}>{title}</span>
        </span>
      }
    />
  );
}

type Options = {
  apr?: string;
  power?: number;
  delegateAddress?: string;
  selfStake?: number;
  delegatorStake?: number;
  delegations?: string;
};

function Details({ data, options }: { data: Validator; options: Options }) {
  // const { rate, maxChangeRate, maxRate } = data.commission.commissionRates;
  const delegateAddress = fromBech32(data.operatorAddress);

  return (
    <Display>
      <div className={styles.wrapperDisplay}>
        <RowsContainer>
          <Row
            title="Operator Address"
            value={trimString(data.operatorAddress, 16, 4)}
          />
          <Row
            title="Address"
            value={<Account avatar sizeAvatar={30} address={delegateAddress} />}
          />
          {data.description.website && (
            <Row
              title="website"
              value={
                <LinkWindow to={data.description.website}>
                  {data.description.website}
                </LinkWindow>
              }
            />
          )}
          {data.description.details && (
            <Row title="Details" value={data.description.details} />
          )}

          {data.jailed && (
            <>
              <Row
                title="Unbonding Height"
                value={formatNumber(BigInt(data.unbondingHeight).toString())}
              />
              <Row
                title="Unbonding Time"
                value={dateFormat(data.unbondingTime, 'dd/mm/yy hh:mm tt')}
              />
            </>
          )}
        </RowsContainer>
        <div className={styles.wrapperCols}>
          <Col value={`${options.apr || 0} %`} title="APR" color="red" />
          <Col value={`${options.power || 0} %`} title="Power" color="green" />
          <Col
            value={`${big(data.commission.commissionRates.rate)
              .multipliedBy(100)
              .toNumber()} %`}
            title="Commission"
            color="green"
          />
          <Col
            value={`${options.selfStake || 0} %`}
            title="Self Stake"
            color="blue"
          />
          <Col
            value={`${options.delegatorStake || 0} %`}
            title="Delegator Shares"
            color="blue"
          />
          <Col
            value={options.delegations || 0}
            title="Delegators"
            color="blue"
          />
        </div>
      </div>
    </Display>
  );
}

export default Details;
