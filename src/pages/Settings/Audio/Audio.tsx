import { useState } from 'react';
import { Display } from 'src/components';
import DisplayTitle from 'src/components/containerGradient/DisplayTitle/DisplayTitle';
import Switch from 'src/components/Switch/Switch';
import { localStorageKeys } from 'src/constants/localStorageKeys';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import VoiceList from './VoiceList';

const adviserAudioKey = localStorageKeys.settings.adviserAudio;

function Audio() {
  const [adviserAudio, setAdviserAudio] = useState(
    localStorage.getItem(adviserAudioKey) === 'true'
  );

  const { setAdviser } = useAdviserTexts({
    successText: 'audio settings',
  });

  function handleSwitch(value: boolean) {
    setAdviserAudio(value);
    localStorage.setItem(adviserAudioKey, value.toString());

    setAdviser(`Adviser audio is turned ${value ? 'on' : 'off'}`, value ? 'green' : 'red');
  }

  return (
    <Display title={<DisplayTitle title="audio" />}>
      <div>
        <Switch value={adviserAudio} onChange={handleSwitch} label="Adviser audio" />

        <VoiceList />
      </div>
    </Display>
  );
}

export default Audio;
