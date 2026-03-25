import React from 'react';
import styles from './Containers.module.scss';

function GridContainer({ children }: { children: React.ReactNode }) {
  return <div className={styles.containerGridContainer}>{children}</div>;
}

function Col({ children }: { children: React.ReactNode }) {
  return <div className={styles.containerCol}>{children}</div>;
}

function TeleportContainer({ children }: { children: React.ReactNode }) {
  return <div className={styles.teleportContainer}>{children}</div>;
}

export { GridContainer, Col, TeleportContainer };
