import { useEffect, useState } from 'react';
import Iframe from 'react-iframe';
import { useNavigate } from 'react-router-dom';
import { Loading } from '../../components';
import styles from './Movie.module.scss';

// const linkMovie =
//   'http://127.0.0.1:8080/ipfs/QmY37mCc1FuSMzpKaHoz5aDtJsz4gnJWG13Vrih8ifxXjS/';

const linkMovie =
  'https://gateway.ipfs.cybernode.ai/ipfs/QmRqJEd8H2MaDCNY8toHD9wTub7JkHzbWZqEZdyzZiqGY5';

function Movie() {
  const [loading, setLoading] = useState(true);
  const navigate = useNavigate();

  const redirect = () => {
    navigate(`/oracle`);
  };

  useEffect(() => {
    const handlerEventListener = (e) => {
      if (e.data === 'endStatusMovie') {
        redirect();
      }
    };
    window.addEventListener('message', handlerEventListener);
    return () => {
      window.removeEventListener('message', handlerEventListener);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [redirect]);

  useEffect(() => {
    const iframeTag = document.querySelector('iframe');

    if (iframeTag) {
      iframeTag.addEventListener('load', () => {
        setLoading(false);
      });
    } else {
      setLoading(false);
    }
  }, []);

  return (
    <div className={styles.container}>
      {loading && (
        <div className={styles.containerLoading}>
          <Loading />
          <div style={{ color: '#fff', marginTop: 20, fontSize: 20 }}>rendering story</div>
        </div>
      )}
      <Iframe
        width="100%"
        height="100%"
        id="iframeCid"
        className={styles.iframeCid}
        src={linkMovie}
        sandbox="allow-scripts"
      />
    </div>
  );
}

export default Movie;
