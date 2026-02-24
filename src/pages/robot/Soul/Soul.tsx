import { useEffect, useRef, useState } from 'react';
import { ContainerGradientText } from 'src/components';
import Switch from 'src/components/Switch/Switch';

import { useGetPassportByAddress } from 'src/containers/sigma/hooks';
import { useBackend } from 'src/contexts/backend/backend';

import { useSigningClient } from 'src/contexts/signerClient';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { setEntrypoint, setEntrypointEnabled } from 'src/redux/reducers/scripting';
import { RootState } from 'src/redux/store';

import { updatePassportParticle } from 'src/services/neuron/neuronApi';
import defaultParticleScript from 'src/services/scripting/rune/default/particle.rn';
import { saveStringToLocalStorage } from 'src/utils/localStorage';
import RuneIde, { SoulIdeHandle } from './RuneEditor/SoulIde/SoulIde';
import ScriptingActionBar from './ScriptingActionBar/ScriptingActionBar';
import styles from './Soul.module.scss';

const entrypointName = 'particle';

function Soul() {
  const dispatch = useAppDispatch();
  const [isChanged, setIsChanged] = useState(false);

  const { ipfsApi } = useBackend();

  const runeIdeRef = useRef<SoulIdeHandle | null>(null);

  const { signer, signingClient } = useSigningClient();

  const { [entrypointName]: currentEntrypoint } = useAppSelector(
    (store: RootState) => store.scripting.scripts.entrypoints
  );

  const { defaultAccount } = useAppSelector((store: RootState) => store.pocket);
  const { passport } = useGetPassportByAddress(defaultAccount);

  const [code, setCode] = useState<string>('');

  const [isLoaded, setIsLoaded] = useState(true);

  const saveScriptToPassport = async (scriptCode: string) => {
    runeIdeRef.current!.putToLog(['⚓️ saving to IPFS ...']);
    const cid = await ipfsApi?.addContent(scriptCode);
    runeIdeRef.current!.putToLog([`🏁 saving '${cid}' to passport ...`]);
    const nickname = passport?.extension.nickname;
    if (cid && nickname) {
      updatePassportParticle(nickname, cid, {
        signer,
        signingClient,
      })
        .then((_result) => {
          runeIdeRef.current!.putToLog(['', `☑️ saved as particle into your passport.`]);
        })
        .catch((error) => {
          runeIdeRef.current!.putToLog(['', `🚫 particle was not saved: ${error}.`]);
        });
    }
  };

  const saveScript = async (script: string) => {
    try {
      runeIdeRef.current!.putToLog(['saving code...']);
      setIsLoaded(false);

      saveStringToLocalStorage(entrypointName, script);

      if (!currentEntrypoint.enabled) {
        runeIdeRef.current!.putToLog(['', '☑️ saved to local storage.']);
        dispatch(setEntrypoint({ name: entrypointName, code: script }));
      } else {
        await saveScriptToPassport(script);
        dispatch(setEntrypoint({ name: entrypointName, code: script }));
      }
    } finally {
      setIsLoaded(true);
      setIsChanged(false);
    }
  };

  const onResetToDefault = async () => {
    runeIdeRef.current!.putToLog(['resetting to default...']);
    setIsLoaded(false);
    setIsChanged(false);
    setCode(defaultParticleScript);
    await saveScript(defaultParticleScript);
  };

  useEffect(() => {
    setCode(currentEntrypoint.script || defaultParticleScript);
  }, []);

  const onCancelClick = () => {
    setCode(currentEntrypoint.script);
  };

  const onSaveClick = async () => {
    const code = await runeIdeRef.current?.save();
    if (code) {
      await saveScript(code);
    }
  };

  const changeScriptEnabled = async (isOn: boolean) => {
    setIsLoaded(false);
    saveScriptToPassport(isOn ? code : '')
      .then(() => {
        dispatch(
          setEntrypointEnabled({
            name: entrypointName,
            enabled: isOn,
          })
        );
      })
      .finally(() => {
        setIsLoaded(true);
      });
  };

  if (!signer || !signingClient) {
    return <ContainerGradientText>Wallet is not connected.</ContainerGradientText>;
  }

  if (!passport) {
    return <ContainerGradientText>Passport required, for this action.</ContainerGradientText>;
  }

  return (
    <ContainerGradientText>
      <div>
        <div className={styles.settingsPanel}>
          <Switch
            value={currentEntrypoint.enabled}
            onChange={changeScriptEnabled}
            label="cybscript enabled"
          />
        </div>
        <RuneIde
          mainCode={code}
          readOnly={!isLoaded}
          ref={runeIdeRef}
          onChange={() => setIsChanged(true)}
        />
      </div>
      <ScriptingActionBar
        isChanged={isChanged}
        onSaveClick={onSaveClick}
        onCancelClick={onCancelClick}
        resetPlayGround={() => runeIdeRef.current!.clearLog()}
        addToLog={(lines) => runeIdeRef.current!.putToLog(lines)}
        nickname={passport.extension.nickname}
        resetToDefault={onResetToDefault}
        compileAndTest={(name, params) => runeIdeRef.current!.test(name, params)}
      />
    </ContainerGradientText>
  );
}

export default Soul;
