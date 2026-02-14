import { useEffect, useMemo, useState } from 'react';
import axios from 'axios';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { useSearchParams } from 'react-router-dom';
import { isDevEnv } from 'src/utils/dev';
import { Endpoints } from '@octokit/types';

type CommitsResponse =
  Endpoints['GET /repos/{owner}/{repo}/commits']['response'];
type Commit = CommitsResponse['data'][0];

const currentCommitSHA = document.head
  .querySelector('meta[name="commit-version"]')
  ?.getAttribute('content');
const currentBranch = document.head
  .querySelector('meta[name="branch"]')
  ?.getAttribute('content');

async function getLastCommit() {
  const response = await axios.get<unknown, CommitsResponse>(
    `https://api.github.com/repos/cyberia-to/cyb-ts/commits`,
    {
      params: {
        sha: currentBranch,
        per_page: 1,
      },
    }
  );

  return response.data[0];
}

const cacheBustParamName = 'cache_bust';

function NewVersionChecker() {
  const [lastCommit, setLastCommit] = useState<Commit>();

  const [searchParams, setSearchParams] = useSearchParams();
  const hasCacheBust = searchParams.has(cacheBustParamName);

  useEffect(() => {
    if (hasCacheBust) {
      setSearchParams((prev) => {
        prev.delete(cacheBustParamName);
        return prev;
      });
    }
  }, [hasCacheBust, setSearchParams]);

  useEffect(() => {
    if (isDevEnv()) {
      return undefined;
    }

    function request() {
      getLastCommit().then(setLastCommit);
    }

    request();

    // check every 3 minutes
    const interval = setInterval(() => {
      request();
    }, 3 * 60 * 1000);

    return () => {
      clearInterval(interval);
    };
  }, []);

  const newVersionAvailable =
    !isDevEnv() &&
    lastCommit &&
    currentCommitSHA &&
    lastCommit.sha !== currentCommitSHA;

  const text = useMemo(() => {
    if (!newVersionAvailable) {
      return null;
    }

    return (
      <div>
        <a href={lastCommit.html_url} target="_blank" rel="noreferrer">
          New version
        </a>{' '}
        available 👨‍💻🚀 <br />{' '}
        <a
          href={window.location.href}
          onClick={(e) => {
            e.preventDefault();

            setSearchParams((prev) => {
              prev.set(cacheBustParamName, Date.now().toString());
              return prev;
            });

            window.location.reload();
          }}
        >
          reload app
        </a>
      </div>
    );
  }, [newVersionAvailable, lastCommit, setSearchParams]);

  useAdviserTexts({
    defaultText: text,
    priority: true,
  });

  return null;
}

export default NewVersionChecker;
