import { useCallback, useEffect, useState } from 'react';
import { Link } from 'react-router-dom';
import { Dots } from 'src/components';
import { Props as AdviserProps } from 'src/features/adviser/Adviser/Adviser';
import { useSetAdviser } from 'src/features/adviser/context';
import useId from 'src/hooks/useId';
import { routes } from 'src/routes';

type Props =
  | {
      isLoading?: boolean;
      loadingText?: string;
      error?: string | undefined;
      defaultText?: string | Element;
      successText?: string;
      txHash?: string;

      priority?: boolean;
    }
  | undefined;

function useAdviserTexts(
  { isLoading, error, defaultText, txHash, loadingText, successText, priority } = {} as Props
) {
  const { setAdviser } = useSetAdviser();

  const [messageShowed, setMessageShowed] = useState(false);

  const key = useId();

  const setAdviserFunc = useCallback(
    (content: AdviserProps['children'], color?: AdviserProps['color'], priority) => {
      setAdviser(key, content, color, priority);
    },
    [setAdviser, key]
  );

  const set2 = useCallback(() => {
    setTimeout(() => {
      setMessageShowed(true);
    }, 4 * 1000);
  }, []);

  useEffect(() => {
    let adviserText = '';
    let color;

    if (error && !messageShowed) {
      adviserText = (
        <p>
          {error} {txHash && <Link to={routes.txExplorer.getLink(txHash)}>check tx</Link>}
        </p>
      );
      color = 'red';
    } else if (isLoading) {
      adviserText = loadingText ? (
        <>
          {loadingText}
          <Dots />
        </>
      ) : (
        'Loading...'
      );
      color = 'yellow';
    } else if (successText && !messageShowed) {
      adviserText = successText;
      color = 'green';
    } else {
      adviserText = defaultText || '';
    }

    setAdviserFunc(adviserText, color, priority);

    if (!messageShowed && (error || successText)) {
      set2();
    }
  }, [
    setAdviserFunc,
    set2,
    priority,
    isLoading,
    error,
    defaultText,
    messageShowed,
    txHash,
    loadingText,
    successText,
  ]);

  useEffect(() => {
    return () => {
      setAdviserFunc(null);
    };
  }, [setAdviserFunc]);

  return {
    setAdviser: setAdviserFunc,
  };
}

export default useAdviserTexts;
