import cx from 'classnames';
import { ReactMarkdown } from 'react-markdown/lib/react-markdown';
import rehypeSanitize from 'rehype-sanitize';
import rehypeStringify from 'rehype-stringify';
import remarkBreaks from 'remark-breaks';
import remarkGfm from 'remark-gfm';
import { shortenString } from 'src/utils/string';
import { LinkWindow } from '../link/link';
import styles from './styles.module.scss';

function TextMarkdown({ children, preview }: { children: string; preview?: boolean }) {
  const { length } = children;

  return (
    <div
      className={cx(styles.wrapper, {
        [styles.markdownContainerPreview]: preview,
        [styles.markdownContainer]: !preview,
        [styles.center]: !preview && length <= 64,
        [styles.title]: !preview && length <= 16,
      })}
    >
      <ReactMarkdown
        rehypePlugins={[rehypeStringify, rehypeSanitize]}
        remarkPlugins={[remarkGfm, remarkBreaks]}
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
      >
        {preview ? shortenString(children) : children}
      </ReactMarkdown>
    </div>
  );
}

export default TextMarkdown;
