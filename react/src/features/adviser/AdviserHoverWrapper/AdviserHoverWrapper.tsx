import { useRef, useState } from 'react';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import styles from './AdviserHoverWrapper.module.scss';

type Props = {
  adviserContent: string | Element;
  children: React.ReactNode;
};

function AdviserHoverWrapper({ children, adviserContent }: Props) {
  const [isHovering, setIsHovering] = useState(false);

  const ref = useRef(null);

  useAdviserTexts({ defaultText: isHovering ? adviserContent : '' });

  return (
    <div
      className={styles.wrapper}
      ref={ref}
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      {children}
    </div>
  );
}

export default AdviserHoverWrapper;
