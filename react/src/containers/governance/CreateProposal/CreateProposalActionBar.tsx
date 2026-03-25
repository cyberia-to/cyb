import { coin } from '@cosmjs/stargate';
import { TextProposal } from '@cybercongress/cyber-ts/cosmos/gov/v1beta1/gov';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { ActionBar } from 'src/components';
import { useSigningClient } from 'src/contexts/signerClient';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import { routes } from 'src/routes';

type Props = {
  proposal: TextProposal;
};

function CreateProposalActionBar({ proposal }: Props) {
  const { signingClient, signerReady } = useSigningClient();
  const currentAddress = useCurrentAddress();
  const navigate = useNavigate();

  console.log(proposal);

  // temp
  const proposalText = proposal?.description;

  const { data, isLoading, error, mutate } = useMutation({
    mutationKey: ['submitProposal'],
    mutationFn: async () => {
      return signingClient!.submitProposal(
        currentAddress,
        {
          typeUrl: '/cosmos.gov.v1beta1.TextProposal',

          value: {
            title: proposal.title,
            description: proposal.description,
          },
        },
        [coin(100000000, 'boot')],
        'auto'
      );
    },
    options: {
      skip: !signerReady,
    },
  });

  const waitForTransaction = useWaitForTransaction({
    hash: data?.transactionHash,
    onSuccess: () => {
      setTimeout(() => {
        navigate(routes.senate.path);
      }, 2000);
    },
  });

  useAdviserTexts({
    isLoading: isLoading || waitForTransaction.isLoading,
    error: error?.message || waitForTransaction.error?.message,
    successText: waitForTransaction.data ? 'Proposal submitted' : undefined,
  });

  console.log(data);
  console.log(waitForTransaction.data);

  return (
    <ActionBar
      button={{
        text: 'Submit',
        onClick: mutate,
        disabled: !signerReady || isLoading || !proposalText || !proposal.title,
      }}
    />
  );
}

export default CreateProposalActionBar;
