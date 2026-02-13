import { useNavigate } from 'react-router-dom';
import { routes } from 'src/routes';
import useCyberlinksCount from 'src/features/cyberlinks/hooks/useCyberlinksCount';
import ParticleSize from 'src/features/particle/ParticleSize/ParticleSize';
import Links from './Links/Links';
import styles from './Meta.module.scss';

type Props = {
  cid: string;
};

function Meta({ cid }: Props) {
  const { data: count } = useCyberlinksCount(cid);

  const navigate = useNavigate();

  return (
    <div className={styles.meta}>
      <Links
        to={count.to}
        from={count.from}
        onChange={() => {
          navigate(routes.ipfs.getLink(cid));
        }}
      />

      <ParticleSize cid={cid} />
    </div>
  );
}

export default Meta;
