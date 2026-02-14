import { Link } from 'react-router-dom';
import { Account, SearchItem } from 'src/components';
import {
  PATTERN_CYBER,
  PATTERN_CYBER_VALOPER,
  PATTERN_TX,
  PATTERN_BLOCK,
} from 'src/constants/patterns';
import { routes } from 'src/routes';
import { trimString, formatNumber } from 'src/utils/utils';
import { v4 as uuidv4 } from 'uuid';

const textPreviewSparkApp = (text, value) => (
  <div style={{ display: 'grid', gap: '10px' }}>
    <div>{text}</div>
    <div style={{ fontSize: '18px', color: '#36d6ae' }}>{value}</div>
  </div>
);

function Wrapper({ children }) {
  const key = uuidv4();
  return <div key={key}>{children}</div>;
}

function FirstItems({ query }: { query: string }) {
  const searchItems = [];

  if (query.match(PATTERN_CYBER)) {
    searchItems.push(
      <Wrapper>
        <Link to={`/network/bostrom/contract/${query}`}>
          <SearchItem hash={`${query}_PATTERN_CYBER`} status="sparkApp">
            {textPreviewSparkApp(
              'Explore details of contract',
              <Account avatar address={query} />
            )}
          </SearchItem>
        </Link>
      </Wrapper>
    );
  }

  if (query.match(PATTERN_CYBER_VALOPER)) {
    searchItems.push(
      <Wrapper>
        <Link to={routes.hero.getLink(query)}>
          <SearchItem hash={`${query}_PATTERN_CYBER_VALOPER`} status="sparkApp">
            {textPreviewSparkApp(
              'Explore details of hero',
              <Account address={query} />
            )}
          </SearchItem>
        </Link>
      </Wrapper>
    );
  }

  if (query.match(PATTERN_TX)) {
    searchItems.push(
      <Wrapper>
        <Link to={`/network/bostrom/tx/${query}`}>
          <SearchItem hash={`${query}_PATTERN_TX`} status="sparkApp">
            {textPreviewSparkApp(
              'Explore details of tx',
              trimString(query, 4, 4)
            )}
          </SearchItem>
        </Link>
      </Wrapper>
    );
  }

  if (query.match(PATTERN_BLOCK)) {
    searchItems.push(
      <Wrapper>
        <Link to={`/network/bostrom/blocks/${query}`}>
          <SearchItem hash={`${query}_PATTERN_BLOCK`} status="sparkApp">
            {textPreviewSparkApp(
              'Explore details of block ',
              formatNumber(parseFloat(query))
            )}
          </SearchItem>
        </Link>
      </Wrapper>
    );
  }

  return searchItems;
}

export default FirstItems;
