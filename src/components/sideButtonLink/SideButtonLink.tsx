import cx from 'classnames';
import { Link } from 'react-router-dom';
import styles from './SideButtonLink.module.scss';

type ContainerLinkProps = {
  to: string;
  buttonType: 'sense' | 'hydrogen';
  children: React.ReactNode;
};

function SideButtonLink({ to, buttonType, children }: ContainerLinkProps) {
  return (
    <Link to={to} className={cx(styles.main, styles[buttonType])}>
      {children}
    </Link>
  );
}

export default SideButtonLink;
