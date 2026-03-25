import { MainContainer } from 'src/components';
import Display from 'src/components/containerGradient/Display/Display';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { Social } from 'src/pages/Social/Social';
import Socials from './Socials';

export const HUB_LINK = 'https://docs.cyb.ai/#/page/aicosystem';

// TODO: folder is dirty, can be refactored
function Social() {
  useAdviserTexts({
    defaultText: 'join our community 🤖',
  });

  return (
    <MainContainer>
      <Display title={<DisplayTitle title="Socials" />} noPaddingX>
        <Socials />
      </Display>
    </MainContainer>
  );
}

export default Social;
