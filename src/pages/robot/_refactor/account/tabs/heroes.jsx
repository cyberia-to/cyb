import { Icon, Pane, Text, Tooltip } from '@cybercongress/gravity';
import { useEffect } from 'react';
import Display from 'src/components/containerGradient/Display/Display';
import Table from 'src/components/Table/Table';
import { BASE_DENOM } from 'src/constants/config';
import { useAdviser } from 'src/features/adviser/context';
import { useRobotContext } from 'src/pages/robot/robot.context';
import { Account, NumberCurrency } from '../../../../../components';
import { formatCurrency, formatNumber } from '../../../../../utils/utils';
import { useGetHeroes } from '../hooks';
import hS from './heroes.module.scss';

const getDaysIn = (time) => {
  const completionTime = new Date(time);
  const timeNow = Date.now();

  const daysIn = (completionTime - timeNow) / 1000 / 60 / 60 / 24;

  return Math.round(daysIn);
};

function TextTable({ children, fontSize, color, display, ...props }) {
  return (
    <Text
      fontSize={`${fontSize || 16}px`}
      color={`${color || '#fff'}`}
      display={`${display || 'inline-flex'}`}
      {...props}
    >
      {children}
    </Text>
  );
}

function Unbonding({ amount, stages, entries }) {
  return (
    <Pane display="flex" alignItems="flex-end">
      <Pane
        // key={}
        fontSize="16px"
        display="inline"
        color="#fff"
        width="100%"
        textOverflow="ellipsis"
        overflow="hidden"
      >
        {stages > 1
          ? `${formatCurrency(amount, BASE_DENOM.toUpperCase())} in ${stages} stages`
          : `${formatCurrency(entries[0].balance, BASE_DENOM.toUpperCase())} in 
      ${getDaysIn(entries[0].completionTime)} days`}
      </Pane>
      <Tooltip
        content={entries.map((items, index) => (
          <div key={index}>
            {`${formatNumber(parseFloat(items.balance))} ${BASE_DENOM.toUpperCase()}`} in{' '}
            {getDaysIn(items.completionTime)} days
          </div>
        ))}
        position="bottom"
      >
        <Icon icon="info-sign" color="#3ab793d4" marginLeft={5} />
      </Tooltip>
    </Pane>
  );
}

function Heroes() {
  const { address, addRefetch } = useRobotContext();
  const { staking: data, refetch, loadingHeroesInfo } = useGetHeroes(address);

  useEffect(() => {
    addRefetch(refetch);
  }, [addRefetch, refetch]);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <>
        collection of heroes at one place <br />
        claim rewards now
      </>
    );
  }, [setAdviser]);

  const delegationsItem = Object.keys(data).map((key) => {
    let amount = 0;
    if (data[key].entries !== undefined) {
      data[key].entries.forEach((entry) => {
        amount += parseFloat(entry.balance);
      });
    }

    const { entries, reward, balance } = data[key];

    return {
      validator: (
        <Account
          address={key}
          containerClassName={hS.containerAccount}
          monikerClassName={hS.containerAccountMoniker}
        />
      ),
      unbondings: entries && (
        <Unbonding amount={amount} stages={entries.length} entries={entries} />
      ),
      reward: <TextTable>{reward ? <NumberCurrency amount={reward} /> : '-'}</TextTable>,
      amount: (
        <TextTable>
          <NumberCurrency amount={balance.amount} />
        </TextTable>
      ),
    };
  });

  return (
    <Display noPaddingX>
      <Table
        isLoading={loadingHeroesInfo}
        columns={[
          {
            header: 'Validator',
            accessorKey: 'validator',
            cell: (info) => info.getValue(),
          },
          {
            header: 'Unbondings',
            accessorKey: 'unbondings',
            cell: (info) => info.getValue(),
          },
          {
            header: 'Rewards',
            accessorKey: 'reward',
            cell: (info) => info.getValue(),
          },
          {
            header: 'Amount',
            accessorKey: 'amount',
            cell: (info) => info.getValue(),
          },
        ]}
        data={delegationsItem}
      />
    </Display>
  );
}

export default Heroes;
