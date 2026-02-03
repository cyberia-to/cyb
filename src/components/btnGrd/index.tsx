import cx from 'classnames';
import { $TsFixMe } from 'src/types/tsfix';
import React, { ComponentProps } from 'react';
import { Link } from 'react-router-dom';
import { Dots } from '../ui/Dots';
import styles from './Button.module.scss';

const audioBtn = require('../../sounds/main-button.mp3');
// const audioBtnHover = require('../../sounds/main-button-hover.mp3');

function GradientContainer({
  children,
  color,
}: {
  children: React.ReactNode;
  color?: 'yellow';
}) {
  return (
    <div className={cx(styles.GradientContainer, styles[color])}>
      {children}
    </div>
  );
}

const audioBtnObg = new Audio(audioBtn);
audioBtnObg.volume = 0.5;
// const audioBtnHoverObg = new Audio(audioBtnHover);

const playAudioClick = () => {
  audioBtnObg.play();
};

// const playAudioHover = () => {
//   audioBtnHoverObg.loop = true;
//   audioBtnHoverObg.play();
// };

// const stopAudioHover = () => {
//   audioBtnHoverObg.loop = false;
//   audioBtnHoverObg.pause();
//   audioBtnHoverObg.currentTime = 0;
// };

export type Props = {
  disabled?: boolean;
  text?: string | JSX.Element;
  img?: $TsFixMe;
  pending?: boolean;
  className?: $TsFixMe;
  pendingText?: string;
  children?: React.ReactNode;
  link?: string;
  onClick?: () => void;
  small?: boolean;
};

function Button({
  disabled,
  text,
  img,
  children,
  pendingText,
  pending,
  onClick,
  link,
  className,
  small,
  ...props
}: Props) {
  // useEffect(() => {
  //   const element = document.querySelector('#BtnGrd');

  //   element.addEventListener('click', playAudioClick, false);

  //   return () => {
  //     element.removeEventListener('click', playAudioClick);
  //   };
  // }, []);

  // useEffect(() => {
  //   const element = document.querySelector('#BtnGrd');

  //   element.addEventListener('mouseover', playAudioHover, false);
  //   element.addEventListener('mouseout', stopAudioHover, false);

  //   return () => {
  //     element.removeEventListener('mouseover', playAudioHover);
  //     element.removeEventListener('mouseout', stopAudioHover);
  //     stopAudioHover();
  //   };
  // }, []);

  function handleClick() {
    onClick?.();

    playAudioClick();
  }

  let Component: HTMLButtonElement | Link = 'button';
  let componentProps: object = {
    type: 'button',
  };

  // link can't be disabled, it is button
  if (link && !disabled) {
    Component = Link;

    const linkProps: ComponentProps<typeof Link> = {
      to: link,
    };

    if (link.startsWith('http')) {
      linkProps.target = '_blank';
      linkProps.rel = 'noreferrer noopener';
    }

    componentProps = linkProps;
  }

  return (
    <Component
      type="button"
      // id="BtnGrd"
      onClick={handleClick}
      className={cx(styles.containerBtnGrd, className, {
        [styles.smallBtn]: small,
        [styles.pending]: pending,
      })}
      disabled={disabled || pending}
      {...props}
      {...componentProps}
    >
      <GradientContainer color={pending ? 'yellow' : undefined}>
        {pending ? (
          <>
            {pendingText || 'pending'}
            <Dots />
          </>
        ) : (
          <>
            {text || children}
            {img && (
              <img style={{ width: 20, height: 20 }} alt="img" src={img} />
            )}
          </>
        )}
      </GradientContainer>
    </Component>
  );
}

export default Button;
