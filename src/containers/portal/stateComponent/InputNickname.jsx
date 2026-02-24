import { ContainerGradient, Input } from '../../../components';
import { steps } from '../citizenship/utils';

function InputNickname({ valueNickname, step, onChangeNickname }) {
  return (
    <ContainerGradient>
      {(step === steps.STEP_NICKNAME_CHOSE || step === steps.STEP_NICKNAME_INVALID) && (
        <div style={{ width: '160px' }}>
          <Input
            value={valueNickname}
            onChange={onChangeNickname}
            placeholder="choose username"
            autoComplete="off"
            textalign="end"
            style={{ fontSize: '16px' }}
          />
        </div>
      )}
      {step === steps.STEP_NICKNAME_APROVE && (
        <div style={{ color: '#36D6AE' }}>{valueNickname}</div>
      )}
    </ContainerGradient>
  );
}

export default InputNickname;
