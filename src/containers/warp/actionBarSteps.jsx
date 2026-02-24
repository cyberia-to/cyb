import { ActionBar as ActionBarContainer } from '@cybercongress/gravity';
import {
  ActionBarContentText,
  Confirmed,
  Dots,
  TransactionError,
  TransactionSubmitted,
} from '../../components';
import { LEDGER } from '../../utils/config';

const { STAGE_ERROR, STAGE_SUBMITTED, STAGE_CONFIRMING, STAGE_CONFIRMED } = LEDGER;

function ActionBarStaps({ stageActionBarStaps }) {
  const { stage, clearState, txHash, txHeight, errorMessage } = stageActionBarStaps;

  if (stage === STAGE_SUBMITTED) {
    return (
      <ActionBarContainer>
        <ActionBarContentText>
          check the transaction <Dots big />
        </ActionBarContentText>
      </ActionBarContainer>
    );
  }

  if (stage === STAGE_CONFIRMING) {
    return <TransactionSubmitted />;
  }

  if (stage === STAGE_CONFIRMED) {
    return <Confirmed txHash={txHash} txHeight={txHeight} onClickBtnClose={() => clearState()} />;
  }

  if (stage === STAGE_ERROR && errorMessage !== null) {
    return <TransactionError errorMessage={errorMessage} onClickBtn={() => clearState()} />;
  }

  return null;
}

export default ActionBarStaps;
