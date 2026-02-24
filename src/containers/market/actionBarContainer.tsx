import { ActionBar, Pane } from '@cybercongress/gravity';
import { Link } from 'react-router-dom';
import { useDevice } from 'src/contexts/device';
import { AccountValue } from 'src/types/defaultAccount';
import { trimString } from '../../utils/utils';
import ActionBarContainer from '../Search/ActionBarContainer';

type Props = {
  addressActive: AccountValue | null;

  keywordHash: string;
  updateFunc: any;
  rankLink: string | null;
  textBtn: string;
};

function ActionBarCont({ addressActive, keywordHash, updateFunc, rankLink, textBtn }: Props) {
  const { isMobile: mobile } = useDevice();

  if (!mobile && addressActive) {
    if (addressActive.keys !== 'read-only') {
      return (
        <ActionBarContainer
          keywordHash={keywordHash}
          update={updateFunc}
          rankLink={rankLink}
          textBtn={textBtn}
        />
      );
    }
    return (
      <ActionBar>
        <Pane fontSize="18px">
          this {trimString(addressActive.bech32, 8, 6)} cyber address is read-only
        </Pane>
      </ActionBar>
    );
  }

  return (
    <ActionBar>
      <Pane fontSize="18px">
        add cyber address in your <Link to="/">pocket</Link>
      </Pane>
    </ActionBar>
  );
}

export default ActionBarCont;
