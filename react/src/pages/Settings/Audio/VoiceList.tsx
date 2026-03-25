import { useEffect, useState } from 'react';
import { Select } from 'src/components';
import { localStorageKeys } from 'src/constants/localStorageKeys';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';

const { adviserVoice } = localStorageKeys.settings;

function VoiceList() {
  const [voices, setVoices] = useState<SpeechSynthesisVoice[]>([]);
  const [voice, setVoice] = useState(localStorage.getItem(adviserVoice));

  const { setAdviser } = useAdviserTexts({
    successText: 'audio settings',
  });

  useEffect(() => {
    const loadVoices = () => {
      const voices = window.speechSynthesis.getVoices();
      setVoices(voices);
    };

    loadVoices();

    window.speechSynthesis.onvoiceschanged = loadVoices;
  }, []);

  return (
    <div>
      <h1>Available Voices</h1>
      <ul>
        <Select
          width={300}
          valueSelect={voice}
          onChangeSelect={(value) => {
            localStorage.setItem(adviserVoice, value);
            setVoice(value);

            setAdviser(`selected voice: ${value}`);
          }}
          options={voices.map((voice) => {
            return {
              value: voice.name,
              text: `${voice.name} (${voice.lang})`,
            };
          })}
        />
      </ul>
    </div>
  );
}

export default VoiceList;
