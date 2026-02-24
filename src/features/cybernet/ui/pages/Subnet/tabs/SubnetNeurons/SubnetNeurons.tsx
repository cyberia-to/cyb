import Display from 'src/components/containerGradient/Display/Display';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useCurrentContract } from 'src/features/cybernet/ui/cybernet.context';
import useCybernetTexts from 'src/features/cybernet/ui/useCybernetTexts';
import { checkIsMLVerse } from 'src/features/cybernet/ui/utils/verses';
import { useCurrentSubnet } from '../../subnet.context';
import SubnetMentorsActionBar from './SubnetMentorsActionBar/SubnetMentorsActionBar';
import SubnetNeuronsTable from './SubnetNeuronsTable/SubnetNeuronsTable';

type Props = {
  addressRegisteredInSubnet: boolean;
};

function SubnetNeurons({ addressRegisteredInSubnet }: Props) {
  const { isRootSubnet } = useCurrentSubnet();

  const { getText } = useCybernetTexts();
  const { type } = useCurrentContract();

  const isMLVerse = checkIsMLVerse(type);

  useAdviserTexts({
    defaultText: `${getText(isRootSubnet ? 'root' : 'subnetwork')} ${getText(
      isRootSubnet ? 'rootValidator' : 'delegate',
      true
    )}`,
  });

  return (
    <Display noPaddingX noPaddingY>
      <SubnetNeuronsTable />

      {addressRegisteredInSubnet && !isRootSubnet && !isMLVerse && <SubnetMentorsActionBar />}
    </Display>
  );
}

export default SubnetNeurons;
