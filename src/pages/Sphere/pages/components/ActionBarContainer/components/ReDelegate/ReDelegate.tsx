import { Validator } from '@cybercongress/cyber-ts/cosmos/staking/v1beta1/staking';
import { InputNumber, Select } from 'src/components';
import { OptionSelect, SelectOption } from 'src/components/Select';
import KeybaseAvatar from '../../../KeybaseAvatar/keybaseAvatar';

const renderOptions = (
  data: Validator[],
  selected: Validator
): SelectOption[] => {
  return data
    .filter(
      (item) =>
        !item.jailed && item.operatorAddress !== selected.operatorAddress
    )
    .map((item) => {
      const { moniker, identity } = item.description;
      return {
        text: moniker.length > 14 ? `${moniker.substring(0, 14)}...` : moniker,
        img: <KeybaseAvatar identity={identity} size="30px" />,
        value: item.operatorAddress,
      };
    });
};

type Props = {
  validatorsAll?: Validator[];
  validatorSelect: Validator;
  valueSelect: string;
  amount: string;
  maxValue: number;
  onChangeReDelegate: (val: string) => void;
  onChangeInputAmount: (val: string) => void;
};

function ReDelegate({
  validatorsAll,
  validatorSelect,
  valueSelect,
  maxValue,
  amount,
  onChangeReDelegate,
  onChangeInputAmount,
}: Props) {
  return (
    <>
      <InputNumber
        value={amount}
        maxValue={maxValue}
        onValueChange={onChangeInputAmount}
        placeholder="amount"
        width="180px"
      />
      <span>restake to:</span>
      <Select
        width="250px"
        valueSelect={valueSelect}
        onChangeSelect={(item) => onChangeReDelegate(item)}
        options={
          validatorsAll ? renderOptions(validatorsAll, validatorSelect) : []
        }
        currentValue={<OptionSelect text="pick hero" value="" />}
      />
    </>
  );
}

export default ReDelegate;
