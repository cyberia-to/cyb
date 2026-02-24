import { Link } from 'react-router-dom';
import { Account, Tooltip } from 'src/components';
import { routes } from 'src/routes';
import styles from './KeywordButton.module.scss';

function KeywordButton({ keyword, author }: { keyword: string }) {
  return (
    <Tooltip tooltip={author && <Account avatar address={author} />}>
      <Link className={styles.keywordBtn} key={keyword} to={routes.search.getLink(keyword)}>
        {keyword}
      </Link>
    </Tooltip>
  );
}

export default KeywordButton;
