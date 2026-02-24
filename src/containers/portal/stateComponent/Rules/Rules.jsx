import ReactMarkdown from 'react-markdown';
import rehypeSanitize from 'rehype-sanitize';
import remarkGfm from 'remark-gfm';
import { ContainerGradient, LinkWindow } from '../../../../components';
import MoonCode from '../MoonCode';
import styles from './Rules.module.scss';

const linkMovie =
  'https://gateway.ipfs.cybernode.ai/ipfs/QmZKL7toTbohUtrd57LaUgcFB8Z47PfAEU1MdLFNP66tXP';

function Rules() {
  return (
    <ContainerGradient
      userStyleContent={{ height: '2350px' }}
      styleLampContent="red"
      title="Moon Code"
      closedTitle="Moon Code"
    >
      <div
        style={{
          width: '100%',
          background: 'transparent',
          position: 'relative',
          height: '300px',
          marginBottom: '10px',
        }}
      >
        {/* eslint-disable-next-line jsx-a11y/media-has-caption */}
        <video width="100%" height="100%" controls>
          <source src={linkMovie} type="video/mp4" />
        </video>
      </div>
      <div
        className={styles.markdown}
        style={{ paddingRight: '15px', height: '100%', width: '100%' }}
      >
        <ReactMarkdown
          // eslint-disable-next-line react/no-children-prop
          children={MoonCode}
          rehypePlugins={[rehypeSanitize]}
          remarkPlugins={[remarkGfm]}
          components={{
            // eslint-disable-next-line react/no-unstable-nested-components
            a: ({ node, ...props }) => {
              if (node.properties?.href) {
                const { href } = node.properties;
                return <LinkWindow to={href} {...props} />;
              }
              return null;
            },
          }}
        />
      </div>
    </ContainerGradient>
  );
}

export default Rules;
