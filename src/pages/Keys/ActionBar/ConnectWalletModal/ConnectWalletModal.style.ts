import { CSSProperties } from 'react';

export const heading: CSSProperties = { padding: '15px' };

export const wrapper: CSSProperties = {
  display: 'flex',
  justifyContent: 'space-between',
};

export const dropdown: CSSProperties = { paddingTop: '15px' };

export const mnemonics: CSSProperties = {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fill, minmax(100px, 1fr))',
  gap: '10px',
};

export const buttons: CSSProperties = {
  paddingTop: '25px',
  display: 'flex',
  justifyContent: 'flex-end',
};
