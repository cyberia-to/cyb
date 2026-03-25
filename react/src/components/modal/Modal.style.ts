import { CSSProperties } from 'react';

export const backdrop: CSSProperties = {
  position: 'fixed',
  top: 0,
  left: 0,
  width: '100vw',
  height: '100vh',
  background: 'rgba(0, 0, 0, 0.85)',
  backdropFilter: 'blur(4px)',
  zIndex: 9999,
};

export const wrapper: CSSProperties = {
  top: '50%',
  left: '50%',
  transform: 'translate(-50%, -50%)',
  position: 'fixed',
  minWidth: 'min(420px, calc(100vw - 32px))',
  maxWidth: 'calc(100vw - 64px)',
  maxHeight: 'calc(100vh - 64px)',
  overflow: 'auto',
  zIndex: 10000,
};
