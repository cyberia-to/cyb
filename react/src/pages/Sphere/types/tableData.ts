import { Coin } from '@cosmjs/stargate';
import { Validator } from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';

export enum RankHeroes {
  imperator = 'imperator',
  jedi = 'jedi',
  padawan = 'padawan',
  heroes = 'heroes',
  relax = 'relax',
  inactive = 'inactive',
}

export interface ValidatorTableData extends Validator {
  id: number;
  rank: RankHeroes;
  apr: number;
  powerPercent: string;
  delegation?: Coin;
}
