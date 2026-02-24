import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { createSearchParams, useSearchParams } from 'react-router-dom';
import { CID_TWEET } from 'src/constants/app';
import { useBackend } from 'src/contexts/backend/backend';
import useDebounce from 'src/hooks/useDebounce';
import useParticle from 'src/hooks/useParticle';
import { addIfpsMessageOrCid } from 'src/utils/ipfs/helpers';
import useAdviserTexts from '../adviser/useAdviserTexts';

type StateActionBar = 'link' | 'keywords-from' | 'keywords-to';

export type KeywordsItem = {
  text: string;
  cid: string;
};

const defaultKeywordsFrom: KeywordsItem = {
  text: 'tweet',
  cid: CID_TWEET,
};

const StudioContext = React.createContext<{
  stateActionBar: StateActionBar;
  keywordsFrom: KeywordsItem[];
  keywordsTo: KeywordsItem[];
  currentMarkdown: string;
  loadedMarkdown: string;
  lastCid: string | undefined;
  setStateActionBar: React.Dispatch<React.SetStateAction<StateActionBar>>;
  addKeywords: (type: 'from' | 'to', item: KeywordsItem[]) => void;
  removeKeywords: (type: 'from' | 'to', item: string) => void;
  saveMarkdown: (md: string) => void;
}>({
  stateActionBar: 'link',
  currentMarkdown: '',
  loadedMarkdown: '',
  lastCid: '',
  keywordsFrom: [],
  keywordsTo: [],
  setStateActionBar: () => {},
  addKeywords: () => {},
  removeKeywords: () => {},
  saveMarkdown: () => {},
});

export const useStudioContext = () => React.useContext(StudioContext);

function StudioContextProvider({ children }: { children: React.ReactNode }) {
  const { isIpfsInitialized, ipfsApi } = useBackend();
  const [cidSearchParams, setCidSearchParams] = useState<string | undefined>();
  const [lastCid, setLastCid] = useState<string | undefined>();

  const firstEffectOccured = useRef(false);
  const [searchParams, setSearchParams] = useSearchParams();
  const updateSearchParams = useCallback(setSearchParams, [setSearchParams]);

  const { status, details } = useParticle(cidSearchParams!);
  const content = details && details.type === 'text' && details.content ? details.content : '';

  const [stateActionBar, setStateActionBar] = useState<StateActionBar>('link');
  const [keywordsFrom, setKeywordsFrom] = useState<KeywordsItem[]>([defaultKeywordsFrom]);
  const [keywordsTo, setKeywordsTo] = useState<KeywordsItem[]>([]);
  const [currentMarkdown, setCurrentMarkdown] = useState<string>('');

  const { debounce } = useDebounce();

  const { setAdviser } = useAdviserTexts({
    isLoading: (!cidSearchParams && status === 'pending') || !isIpfsInitialized,
    loadingText: !isIpfsInitialized ? 'node is loading' : 'cid is loading',
    defaultText: 'you can create content',
    error:
      details && details.type !== 'text'
        ? 'invalid content type, content must be text type'
        : undefined,
  });

  useEffect(() => {
    if (firstEffectOccured.current) {
      return;
    }

    firstEffectOccured.current = true;
    const param = Object.fromEntries(searchParams.entries());

    if (Object.keys(param).length > 0) {
      const { cid } = param;
      setCidSearchParams(cid);
      setLastCid(cid);
    }
  }, [searchParams]);

  useEffect(() => {
    if (content.length && !currentMarkdown.length) {
      setCurrentMarkdown(content);
    }
  }, [content, currentMarkdown]);

  const handleSaveMarkdown = useCallback(
    debounce((markdown: string) => {
      addIfpsMessageOrCid(markdown, { ipfsApi }).then((cid) => {
        updateSearchParams(createSearchParams({ cid }), { replace: true });
        setLastCid(cid);

        setAdviser('📝 Particle saved to ipfs', 'yellow');

        setTimeout(() => {
          setAdviser('you can create content');
        }, 5000);
      });
    }, 5000),
    []
  );

  const saveMarkdown = useCallback(
    (markdown: string) => {
      setCurrentMarkdown(markdown);

      if (!firstEffectOccured.current || !ipfsApi) {
        return;
      }

      if (markdown.length === 0) {
        updateSearchParams(createSearchParams({}), { replace: true });
        setLastCid(undefined);
        return;
      }

      handleSaveMarkdown(markdown);
    },
    [handleSaveMarkdown, ipfsApi, updateSearchParams]
  );

  const addKeywords = useCallback(
    (type: 'from' | 'to', newItem: KeywordsItem[]) => {
      if (!isIpfsInitialized) {
        return;
      }

      const stateKeywords = type === 'from' ? keywordsFrom : keywordsTo;
      const setStateKeywords = type === 'from' ? setKeywordsFrom : setKeywordsTo;

      const uniqueArray = [
        ...new Map([...stateKeywords, ...newItem].map((item) => [item.cid, item])).values(),
      ];

      if (uniqueArray.length) {
        setStateKeywords(uniqueArray);
      }
    },
    [keywordsFrom, keywordsTo, isIpfsInitialized]
  );

  const removeKeywords = useCallback(
    (type: 'from' | 'to', itemCid: string) => {
      const stateKeywords = type === 'from' ? keywordsFrom : keywordsTo;
      const setStateKeywords = type === 'from' ? setKeywordsFrom : setKeywordsTo;
      const newState = stateKeywords.filter((item) => item.cid !== itemCid);
      setStateKeywords(newState);
    },
    [keywordsFrom, keywordsTo]
  );

  const contextValue = useMemo(
    () => ({
      currentMarkdown,
      loadedMarkdown: content,
      stateActionBar,
      keywordsFrom,
      keywordsTo,
      lastCid,
      setStateActionBar,
      addKeywords,
      removeKeywords,
      saveMarkdown,
    }),
    [
      stateActionBar,
      keywordsFrom,
      keywordsTo,
      lastCid,
      addKeywords,
      currentMarkdown,
      content,
      removeKeywords,
      saveMarkdown,
    ]
  );

  return <StudioContext.Provider value={contextValue}>{children}</StudioContext.Provider>;
}

export default StudioContextProvider;
