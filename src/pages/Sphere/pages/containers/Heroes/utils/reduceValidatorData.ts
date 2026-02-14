import {
  BondStatus,
  Validator,
} from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';
import BigNumber from 'bignumber.js';
import { Coin } from '@cosmjs/stargate';
import { RankHeroes, ValidatorTableData } from '../../../../types/tableData';

type Options = {
  bondedTokens: number;
  delegationsData: { [key: string]: Coin };
  stakingProvisions?: string;
};

const checkRank = (
  percent: number,
  id: number,
  status: ValidatorTableData['status']
): RankHeroes => {
  if (id === 0) {
    return RankHeroes.imperator;
  }

  if (status === BondStatus.BOND_STATUS_BONDED && percent < 0.33) {
    return RankHeroes.jedi;
  }

  if (status === BondStatus.BOND_STATUS_BONDED && percent < 0.67) {
    return RankHeroes.padawan;
  }

  if (status === BondStatus.BOND_STATUS_UNBONDING) {
    return RankHeroes.relax;
  }

  if (status === BondStatus.BOND_STATUS_UNBONDED) {
    return RankHeroes.inactive;
  }

  return RankHeroes.heroes;
};

function reduceValidatorData(data: Validator[], options: Options) {
  // const totalPower = data.reduce(
  //   (acc, item) =>
  //     !item.jailed
  //       ? new BigNumber(acc).plus(item.delegatorShares).toNumber()
  //       : acc,
  //   0
  // );

  const { bondedTokens, delegationsData, stakingProvisions } = options;
  return data.reduce<{ total: number; list: ValidatorTableData[] }>(
    (acc, item, id) => {
      const { jailed, operatorAddress, tokens, delegatorShares, commission } =
        item;

      acc.total = jailed
        ? acc.total
        : new BigNumber(acc.total).plus(delegatorShares).toNumber();

      const percent = new BigNumber(acc.total)
        .dividedBy(bondedTokens)
        .dp(1, BigNumber.ROUND_FLOOR)
        .toNumber();

      const delegation = delegationsData[operatorAddress] || undefined;

      const powerPercent = new BigNumber(tokens)
        .dividedBy(bondedTokens)
        .multipliedBy(100)
        .dp(2, BigNumber.ROUND_FLOOR)
        .toFixed(2);

      const stakingProvisionsByItem = stakingProvisions
        ? new BigNumber(stakingProvisions).multipliedBy(
            new BigNumber(1).minus(commission.commissionRates.rate)
          )
        : undefined;

      const estimatedApr = stakingProvisionsByItem
        ? new BigNumber(stakingProvisionsByItem)
            .dividedBy(bondedTokens)
            .toNumber()
        : 0;

      acc.list.push({
        ...item,
        delegation,
        id: id + 1,
        apr: estimatedApr,
        powerPercent,
        rank: checkRank(percent, id, item.status),
      });
      return acc;
    },
    { total: 0, list: [] }
  ).list;
}

export default reduceValidatorData;
