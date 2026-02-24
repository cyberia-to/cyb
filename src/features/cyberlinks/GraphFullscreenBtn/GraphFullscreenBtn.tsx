import { useState } from 'react';
import { Button } from 'src/components';
import { PORTAL_ID } from 'src/containers/application/App';
import AdviserHoverWrapper from 'src/features/adviser/AdviserHoverWrapper/AdviserHoverWrapper';
import useEventListener from 'src/hooks/dom/useEventListener';
import styles from './GraphFullscreenBtn.module.scss';
import expandIcon from './images/expand.svg';
import minimizeIcon from './images/minimize.svg';

export function useFullscreen() {
  const [isFullscreen, setIsFullscreen] = useState(false);

  function handleFullscreenChange() {
    setIsFullscreen(document.fullscreenElement !== null);
  }

  useEventListener('fullscreenchange', handleFullscreenChange, document);

  function toggleFullscreen(element: HTMLElement = document.documentElement) {
    if (!document.fullscreenElement) {
      element.requestFullscreen();
    } else if (document.exitFullscreen) {
      document.exitFullscreen();
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    // if input is focused, do not handle keydown
    if (['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName)) {
      return;
    }
    if (event.key === 'f') {
      toggleFullscreen(document.getElementById(PORTAL_ID));
    }
  }

  // listen F key
  useEventListener('keydown', handleKeyDown);

  return {
    isFullscreen,
    toggleFullscreen,
  };
}

function GraphFullscreenBtn() {
  const { isFullscreen, toggleFullscreen } = useFullscreen();

  function onClick() {
    if (!document.fullscreenElement) {
      toggleFullscreen(document.getElementById(PORTAL_ID));
    } else if (document.exitFullscreen) {
      document.exitFullscreen();
    }
  }

  const text = isFullscreen ? 'Exit Fullscreen' : 'Enter Fullscreen';

  return (
    <AdviserHoverWrapper adviserContent={text}>
      <Button className={styles.btn} onClick={onClick}>
        <img src={isFullscreen ? minimizeIcon : expandIcon} alt={text} />
      </Button>
    </AdviserHoverWrapper>
  );
}

export default GraphFullscreenBtn;
