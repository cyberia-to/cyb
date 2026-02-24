import { useCallback } from 'react';
import { useCybernet } from './cybernet.context';
import { Texts, texts } from './cybernetTexts';

function useCybernetTexts() {
  const { selectedContract } = useCybernet();

  const type = selectedContract?.type;

  const getText = useCallback(
    (key: Texts, isPlural?: boolean) => {
      const t = type === 'graph' ? 'graph' : 'default';
      const t2 = texts[key][t];

      let text: string;

      // refactor
      if (typeof t2 === 'object') {
        text = isPlural ? t2.plural || `${t2}s` : t2.single || t2;
      } else {
        text = t2;
        if (isPlural) {
          text += 's';
        }
      }

      return text;
    },
    [type]
  );

  return {
    getText,
  };
}

export default useCybernetTexts;
