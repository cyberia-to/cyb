import styles from './styles.scss';

type Props = {
  children: React.ReactNode;
  minHeight?: string;
  width?: string;
  [key: string]: any;
};

function MainContainer({ children, minHeight, width = '62%', ...props }: Props) {
  return (
    <main
      style={{
        minHeight: minHeight || 'calc(100vh - 162px)',
        // overflow: 'hidden',
      }}
      className="block-body"
      {...props}
    >
      <div style={{ width }} className={styles.containerContent}>
        {children}
      </div>
    </main>
  );
}

export default MainContainer;
