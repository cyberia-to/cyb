import React from 'react';
import { Helmet } from 'react-helmet';
import styles from './styles.module.scss';

interface MainContainerProps {
  children: React.ReactNode;
  width?: string;
  title?: string;
}

function MainContainer({ children, width = '62%', title }: MainContainerProps) {
  return (
    <div style={{ width }} className={styles.wrapper}>
      {title && (
        <Helmet>
          <title>{title}</title>
        </Helmet>
      )}

      <main>{children}</main>
    </div>
  );
}

export default MainContainer;
