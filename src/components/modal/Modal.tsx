import { ClipboardEventHandler, useCallback, useEffect, useMemo, useRef } from 'react';
import { createPortal } from 'react-dom';
import Display from '../containerGradient/Display/Display';
import * as styles from './Modal.style';

interface ModalProps {
  children: React.ReactNode;
  isOpen: boolean;
  style?: React.CSSProperties;
  onClose?: () => void;
  onPaste?: ClipboardEventHandler<HTMLDivElement>;
}

export default function Modal({
  isOpen,
  style,
  onClose,
  onPaste,
  children,
}: ModalProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (isOpen) {
      ref.current?.focus();
    }
  }, [isOpen]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape' && onClose) {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown);
      return () => document.removeEventListener('keydown', handleKeyDown);
    }
  }, [isOpen, handleKeyDown]);

  const wrapperStyles = useMemo(
    () => ({ ...styles.wrapper, ...style }),
    [style]
  );

  if (!isOpen) {
    return null;
  }

  return createPortal(
    <>
      <div style={styles.backdrop} onClick={onClose} />
      <div
        ref={ref}
        tabIndex={-1}
        style={wrapperStyles}
        onPaste={onPaste}
        role="dialog"
        aria-modal="true"
      >
        <Display>{children}</Display>
      </div>
    </>,
    document.body
  );
}
