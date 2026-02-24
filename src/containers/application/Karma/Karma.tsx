import IconsNumber from 'src/components/IconsNumber/IconsNumber';
import styles from './Karma.module.scss';
import { useGetKarma } from './useGetKarma';

function Karma({ address }: { address: string }) {
  const { data } = useGetKarma(address);

  if (!data) {
    return null;
  }

  return (
    // to={routes.robot.routes.karma.path}
    <span className={styles.containerKarma}>
      {/* <Tooltip tooltip="Karma measure the brightness of cyberlinks and particles created by you"> */}
      <IconsNumber value={data} type="karma" />
      {/* </Tooltip> */}
    </span>
  );
}

export default Karma;
