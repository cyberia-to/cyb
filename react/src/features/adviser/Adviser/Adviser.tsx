import cx from 'classnames';
import React, { useEffect, useRef, useState } from 'react';
import { localStorageKeys } from 'src/constants/localStorageKeys';
import styles from './Adviser.module.scss';

// import TypeIt from 'typeit-react';

const adviserAudioKey = localStorageKeys.settings.adviserAudio;
const adviserVoiceKey = localStorageKeys.settings.adviserVoice;

export enum AdviserColors {
  blue = 'blue',
  green = 'green',
  red = 'red',
  purple = 'purple',
  yellow = 'yellow',
}

export type Props = {
  children: React.ReactNode | string | Element | null;
  color?: AdviserColors | keyof typeof AdviserColors;
  disabled?: boolean;
  className?: string;

  isOpen?: boolean;
  openCallback?: (isOpen: boolean) => void;
};

function prepareText(text: string) {
  let t = text;

  // replace '\n' with .
  t = t.replace(/\n/g, '. ');

  // replace emoji
  // TODO: RGI_Emoji can be used in ECMAScript 2024 soon
  // FIXME: use utils/emoji.ts
  t = t.replace(
    /[\u{1F600}-\u{1F64F}\u{1F300}-\u{1F5FF}\u{1F680}-\u{1F6FF}\u{1F700}-\u{1F77F}\u{1F780}-\u{1F7FF}\u{1F800}-\u{1F8FF}\u{1F900}-\u{1F9FF}\u{1FA00}-\u{1FA6F}\u{1FA70}-\u{1FAFF}\u{2600}-\u{26FF}\u{2700}-\u{27BF}]/gu,
    ''
  );

  return t;
}

const synth = window.speechSynthesis;

function play(text: string) {
  // replace emoji
  const cleanText = text.replace(/[\u{1F600}-\u{1F64F}]/gu, '');

  const utterThis = new SpeechSynthesisUtterance(cleanText);
  utterThis.lang = 'en-US';

  const voiceLS = localStorage.getItem(adviserVoiceKey);

  utterThis.voice =
    synth.getVoices().find((voice) => voice.name === voiceLS) || synth.getVoices()[0];

  synth.speak(utterThis);
}

function Adviser({
  children,
  color = AdviserColors.blue,
  className,
  disabled,
  openCallback,
  isOpen: forceOpen = false,
}: Props) {
  const [isOpen, setIsOpen] = useState(forceOpen);

  useEffect(() => {
    setIsOpen(forceOpen);
  }, [forceOpen]);

  useEffect(() => {
    // context infinity render
    const timeout = setTimeout(() => {
      if (openCallback && isOpen !== forceOpen) {
        openCallback(isOpen);
      }
    }, 1);

    return () => {
      clearTimeout(timeout);
    };
  }, [openCallback, isOpen, forceOpen]);

  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const audioEnabled = localStorage.getItem(adviserAudioKey) === 'true';

    if (!audioEnabled) {
      return;
    }

    const t = ref.current?.innerText;

    if (!t) {
      return;
    }
    const text = prepareText(t);

    if (text && color === AdviserColors.blue && isOpen) {
      play(text);
    }

    return () => {
      synth.cancel();
    };
  }, [color, isOpen]);

  function handleInteraction(e: React.MouseEvent | React.KeyboardEvent) {
    const { target, currentTarget } = e;

    if (
      target !== currentTarget &&
      ['a', 'button'].includes((e.target as any).tagName.toLowerCase())
    ) {
      return;
    }

    setIsOpen(!isOpen);
  }

  return (
    // maybe try use <details> tag
    <div
      role="button"
      className={cx(styles.wrapper, styles[`color_${color}`], className, {
        [styles.open]: isOpen && children,
        [styles.disabled]: disabled,
      })}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.key === 'Enter') {
          handleInteraction(e);
        }
      }}
      onClick={(e) => {
        if (window.getSelection()?.toString()) {
          return;
        }

        handleInteraction(e);
      }}
    >
      <span className={styles.summary}>Adviser</span>
      <div className={styles.content} ref={ref}>
        {/* {color !== AdviserColors.purple ? (
          <TypeIt
            // for resetting the animation
            key={children + children?.length}
            as="div"
            className={styles.typer}
            options={{
              speed: 30,
              cursor: false,
              waitUntilVisible: true,
            }}
          >
            {children}
          </TypeIt>
        ) : (
          children
        )} */}
        {children}
      </div>
    </div>
  );
}

export default Adviser;
