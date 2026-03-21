import { ClipboardEventHandler, useEffect, useMemo, useRef } from 'react';
import Display from '../containerGradient/Display/Display';
import * as styles from './Modal.style';

interface ModalProps {
  children: React.ReactNode;
  isOpen: boolean;
  style?: React.CSSProperties;

  onPaste?: ClipboardEventHandler<HTMLDivElement>;
}

export default function Modal({
  isOpen,
  style,
  onPaste,
  children,
}: ModalProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) {
      ref.current?.focus();
    }
  }, [isOpen]);

  const wrapperStyles = useMemo(
    () => ({ ...styles.wrapper, ...style }),
    [style]
  );

  if (!isOpen) {
    return null;
  }

  return (
    <>
      <div style={styles.backdrop} />
      <div ref={ref} style={wrapperStyles} onPaste={onPaste}>
        <Display>{children}</Display>
      </div>
    </>
  );
}
