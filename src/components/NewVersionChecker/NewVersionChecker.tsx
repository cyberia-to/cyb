import { Endpoints } from '@octokit/types';
import axios from 'axios';
import { useEffect, useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import useAdviserTexts from 'src/features/adviser/useAdviserTexts';
import { isDevEnv } from 'src/utils/dev';

type CommitsResponse = Endpoints['GET /repos/{owner}/{repo}/commits']['response'];
type Commit = CommitsResponse['data'][0];

const currentCommitSHA = document.head
  .querySelector('meta[name="commit-version"]')
  ?.getAttribute('content');
const currentBranch = document.head.querySelector('meta[name="branch"]')?.getAttribute('content');

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

  // If user already clicked reload but version still doesn't match,
  // suppress the banner to avoid infinite reload loop
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (hasCacheBust) {
      setDismissed(true);
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
    const interval = setInterval(
      () => {
        request();
      },
      3 * 60 * 1000
    );

    return () => {
      clearInterval(interval);
    };
  }, []);

  const newVersionAvailable =
    !dismissed && !isDevEnv() && lastCommit && currentCommitSHA && lastCommit.sha !== currentCommitSHA;

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
          onClick={async (e) => {
            e.preventDefault();

            // Unregister service workers and clear caches so reload fetches fresh index.html
            if ('serviceWorker' in navigator) {
              const registrations = await navigator.serviceWorker.getRegistrations();
              await Promise.all(registrations.map((r) => r.unregister()));
            }
            const cacheNames = await caches.keys();
            await Promise.all(cacheNames.map((name) => caches.delete(name)));

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
