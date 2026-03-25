import { Pane } from '@cybercongress/gravity';
import { BtnGrd, ButtonIcon } from '../../../../components';
import ActionBarContainer from './index';

const back = require('../../../../image/arrow-left-img.svg');

function ActionBarContentText({ children, gridGap, ...props }) {
  return (
    <Pane
      display="flex"
      fontSize="20px"
      justifyContent="center"
      alignItems="center"
      flexGrow={1}
      gridGap={gridGap}
      // marginRight="15px"
      {...props}
    >
      {children}
    </Pane>
  );
}

type ActionBarSteps = {
  children?: JSX.Element;
  btnText?: string;
  disabled?: boolean;
  gridGap?: number;
  onClickFnc?: () => void;
  onClickBack?: () => void;
};

function ActionBarSteps({
  children,
  btnText,
  onClickFnc,
  onClickBack,
  disabled,
  gridGap,
}: ActionBarSteps) {
  return (
    <ActionBarContainer>
      {onClickBack && (
        <ButtonIcon
          styleContainer={{ position: 'absolute', left: '0' }}
          style={{ padding: 0 }}
          img={back}
          onClick={onClickBack}
          text="previous step"
        />
      )}
      <ActionBarContentText
        gridGap={gridGap}
        //  marginLeft={onClickBack ? 30 : 0}
      >
        {children}
      </ActionBarContentText>
      {btnText && <BtnGrd disabled={disabled} onClick={onClickFnc} text={btnText} />}
    </ActionBarContainer>
  );
}

export default ActionBarSteps;
