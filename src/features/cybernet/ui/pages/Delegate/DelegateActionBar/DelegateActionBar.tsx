import { useState } from 'react';
import { AmountDenom, Button, InputNumber } from 'src/components';

import ActionBar from 'src/components/actionBar';
import { useGetBalance } from 'src/containers/sigma/hooks/utils';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import useDelegate from '../../../hooks/useDelegate';
import useCybernetTexts from '../../../useCybernetTexts';
import useExecuteCybernetContract from '../../../useExecuteCybernetContract';

enum Steps {
  INITIAL,
  STAKE,
  UNSTAKE,
}

type Props = {
  // delegator address
  address: string;
  stakedAmount: number | undefined;
  onSuccess: () => void;
};

function DelegateActionBar({ address, stakedAmount, onSuccess }: Props) {
  const [step, setStep] = useState(Steps.INITIAL);

  const currentAddress = useAppSelector(selectCurrentAddress);

  const query = useDelegate(address);
  const isDelegateExists = !query.loading && !!query?.data;

  const balanceQuery = useGetBalance(currentAddress);
  const availableBalance = balanceQuery.data?.liquid?.amount;

  const { getText } = useCybernetTexts();

  const isOwner = currentAddress === address;

  const [amount, setAmount] = useState(0);

  const { setAdviser } = useAdviserTexts();

  function handleSuccess() {
    setStep(Steps.INITIAL);
    setAmount(0);
    balanceQuery.refetch();
    onSuccess();
  }

  const executeStake = useExecuteCybernetContract({
    query: {
      add_stake: {
        hotkey: address,
      },
    },
    funds: [
      {
        denom: 'pussy',
        amount: String(amount),
      },
    ],
    onSuccess: handleSuccess,
    successMessage: 'Stake has been successfully added',
  });

  const executeUnstake = useExecuteCybernetContract({
    query: {
      remove_stake: {
        hotkey: address,
        amount,
      },
    },
    onSuccess: handleSuccess,
    successMessage: 'Stake has been successfully removed',
  });

  const executeBecomeDelegate = useExecuteCybernetContract({
    query: {
      become_delegate: {
        hotkey: currentAddress,
      },
    },
    // onSuccess: handleSuccess,
    successMessage: `You have successfully became a ${getText('delegate')}`,
  });

  let button;
  let content;
  let onClickBack;

  function handleClickBack() {
    setStep(Steps.INITIAL);
    setAmount(0);
  }

  switch (step) {
    case Steps.INITIAL:
      if (!isDelegateExists) {
        if (isOwner) {
          content = (
            <Button
              onClick={executeBecomeDelegate.mutate}
              pending={executeBecomeDelegate.isLoading}
            >
              Become {getText('delegate')}
            </Button>
          );
        }

        break;
      }

      content = (
        <>
          <Button
            onClick={() => {
              setStep(Steps.STAKE);
            }}
          >
            Stake
          </Button>

          {stakedAmount && (
            <Button
              onClick={() => {
                setStep(Steps.UNSTAKE);
              }}
            >
              Unstake
            </Button>
          )}
        </>
      );

      setAdviser('Stake or unstake');

      break;

    case Steps.STAKE: {
      const { mutate, isReady, isLoading } = executeStake;

      content = (
        <InputNumber
          value={amount}
          disabled={isLoading}
          maxValue={availableBalance}
          onChange={(val) => setAmount(Number(val))}
        />
      );

      onClickBack = handleClickBack;

      setAdviser(
        <div>
          <p>Stake</p>
          {availableBalance >= 0 && (
            <p
              style={{
                display: 'flex',
                gap: '0 7px',
              }}
            >
              Available balance: <AmountDenom amountValue={availableBalance} denom="pussy" />
            </p>
          )}
        </div>
      );

      button = {
        text: 'Stake',
        onClick: mutate,
        disabled: !isReady || amount === 0,
        pending: isLoading,
      };

      break;
    }

    case Steps.UNSTAKE: {
      const { mutate, isReady, isLoading } = executeUnstake;

      content = (
        <InputNumber
          value={amount}
          disabled={isLoading}
          maxValue={stakedAmount}
          onChange={(val) => setAmount(Number(val))}
        />
      );

      onClickBack = handleClickBack;

      setAdviser(
        <div>
          <p>Unstake</p>
          <p
            style={{
              display: 'flex',
              gap: '0 7px',
            }}
          >
            Available balance: <AmountDenom amountValue={stakedAmount} denom="pussy" />
          </p>
        </div>
      );

      button = {
        text: 'Unstake',
        onClick: mutate,
        disabled: !isReady || amount === 0,
        pending: isLoading,
      };

      break;
    }

    default:
      break;
  }

  return (
    <ActionBar onClickBack={onClickBack} button={button}>
      {content}
    </ActionBar>
  );
}

export default DelegateActionBar;
