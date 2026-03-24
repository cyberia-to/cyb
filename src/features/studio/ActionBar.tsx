import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ActionBar } from 'src/components';
import { PATTERN_IPFS_HASH } from 'src/constants/patterns';
import { useBackend } from 'src/contexts/backend/backend';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import { InputMemo } from 'src/pages/teleport/components/Inputs';
import { routes } from 'src/routes';
import { sendCyberlinkArray } from 'src/services/neuron/neuronApi';
import { addIfpsMessageOrCid } from 'src/utils/ipfs/helpers';
import useAdviserTexts from '../adviser/useAdviserTexts';
import { KeywordsItem, useStudioContext } from './studio.context';
import { friendlyErrorMessage } from 'src/utils/errorMessages';
import { checkLoopLinks, mapLinks, reduceLoopKeywords } from './utils/utils';

// function execute (content: any[]) {

//   // const addToIPFS = useAddToIPFS([content])
//   const {isLoading, error} = useExecutionWithWaitAndAdviser(execute);

//   // src/features/cyberlinks/hooks/useCyberlink.ts
//   // useCyberlinks(cyberlinks[])

//   const {setAdviser} = useAdviserTexts({});

//   function execute2 () {

//     // check existing links
//     // setAdviser()

//     await addToIPFS.execute();

//     // useCyberlinks

//   }

//   return {
//     isReady,
//     iaLoading,
//     error
//   }

// }

function ActionBarContainer() {
  const { signer, signingClient } = useSigningClient();
  const { isIpfsInitialized, ipfsApi, senseApi, isDbInitialized } = useBackend();
  const queryClient = useQueryClient();
  const navigate = useNavigate();

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string>();

  const [newKeywords, setNewKeywords] = useState<
    | {
        value: string;
        fileName?: string;
      }
    | undefined
  >();

  const {
    currentMarkdown,
    keywordsFrom,
    keywordsTo,
    setStateActionBar,
    stateActionBar,
    addKeywords,
  } = useStudioContext();

  const [tx, setTx] = useState({
    hash: '',
    onSuccess: () => {},
  });

  useWaitForTransaction({
    hash: tx.hash,
    onSuccess: tx.onSuccess,
  });

  const { setAdviser } = useAdviserTexts({
    isLoading: loading,
    loadingText: 'transaction pending...',
    error,
  });

  const addNewKeywords = useCallback(async () => {
    if (!newKeywords?.value || !ipfsApi) {
      return;
    }

    const { value, fileName } = newKeywords;

    const newItem: KeywordsItem[] = [];

    if (value.match(PATTERN_IPFS_HASH)) {
      newItem.push({ text: fileName || value, cid: value });
    } else {
      const arrValue = value.split(',');

      for (let index = 0; index < arrValue.length; index++) {
        const item = arrValue[index];
        // eslint-disable-next-line no-await-in-loop
        const itemCid = await addIfpsMessageOrCid(item, { ipfsApi });
        newItem.push({ text: item, cid: itemCid });
      }
    }

    addKeywords(stateActionBar === 'keywords-from' ? 'from' : 'to', newItem);

    setNewKeywords(undefined);
    setStateActionBar('link');
  }, [newKeywords, ipfsApi, addKeywords, stateActionBar, setStateActionBar]);

  useEffect(() => {
    setError(undefined);
  }, []);

  const createCyberlinkTx = async () => {
    if (!signer || !signingClient || !senseApi || !isDbInitialized) {
      return;
    }

    const [{ address }] = await signer.getAccounts();

    // Check V (millivolt) balance — required for cyberlinks
    if (queryClient) {
      try {
        const balance = await queryClient.getBalance(address, 'millivolt');
        if (!balance || BigInt(balance.amount) === 0n) {
          setError('You need V (volts) to create cyberlinks. Go to Energy to get some.');
          return;
        }
      } catch {
        // If balance check fails, proceed anyway — tx will fail with a clear error
      }
    }

    setAdviser('preparing content...');
    setLoading(true);

    const currentMarkdownCid = await addIfpsMessageOrCid(currentMarkdown, {
      ipfsApi,
    });

    const links = mapLinks(currentMarkdownCid, {
      from: keywordsFrom,
      to: keywordsTo,
    });

    const { uniqueLinks, loopLink } = await checkLoopLinks(links);

    if (loopLink.length) {
      setAdviser(
        <>
          Links with these keywords have already been created: <br />
          {reduceLoopKeywords(loopLink, [...keywordsFrom, ...keywordsTo]).join(', ')}
        </>,
        'yellow'
      );
    }

    if (!uniqueLinks.length) {
      setTimeout(() => {
        setError('try adding more unique keywords');
      }, 5000);
      return;
    }

    setLoading(true);
    setAdviser('signing and broadcasting transaction...');

    await sendCyberlinkArray(address, uniqueLinks, { signingClient, senseApi })
      .then((txHash) => {
        setAdviser('waiting for transaction confirmation...');
        setTx({
          hash: txHash,
          onSuccess: () => {
            setLoading(false);
            setAdviser('link created successfully!', 'green');
            setTimeout(() => navigate(routes.ipfs.getLink(currentMarkdownCid)), 2000);
          },
        });
      })
      .catch((e) => {
        setError(friendlyErrorMessage(e?.message || e));
        console.error(e);
        setLoading(false);
      });
  };

  const isDisabledLink = useMemo(() => {
    const isKeywords = ![...keywordsFrom, ...keywordsTo].length;

    return isKeywords || loading || !currentMarkdown.length;
  }, [currentMarkdown, keywordsFrom, keywordsTo, loading]);

  if (!isIpfsInitialized) {
    return <ActionBar>node is loading...</ActionBar>;
  }

  if (stateActionBar === 'link') {
    return (
      <ActionBar
        button={{
          text: 'publish',
          disabled: isDisabledLink,
          onClick: createCyberlinkTx,
          pending: loading,
        }}
      />
    );
  }

  if (stateActionBar === 'keywords-from' || stateActionBar === 'keywords-to') {
    const textBtn = `add ${stateActionBar === 'keywords-from' ? 'incoming' : 'outcoming'} link(s)`;
    return (
      <ActionBar
        button={{
          text: textBtn,
          onClick: addNewKeywords,
          disabled: !newKeywords?.value.length,
        }}
        onClickBack={() => setStateActionBar('link')}
      >
        <InputMemo
          title="type keywords"
          value={newKeywords?.value || ''}
          onChangeValue={(value, fileName) => setNewKeywords({ value, fileName })}
        />
      </ActionBar>
    );
  }

  return null;
}

export default ActionBarContainer;
