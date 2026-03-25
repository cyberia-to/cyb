import { coin } from '@cosmjs/launchpad';
import { useEffect, useState } from 'react';
import { createSearchParams, Link } from 'react-router-dom';
import { ActionBar, Button, DenomArr, Input } from 'src/components';
import { BASE_DENOM } from 'src/constants/config';
import { useBackend } from 'src/contexts/backend/backend';
import { useSigningClient } from 'src/contexts/signerClient';
import { isParticle } from 'src/features/particle/utils';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import { ibcDenomAtom } from 'src/pages/teleport/bridge/bridge';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppDispatch, useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { sendCyberlink, sendTokensWithMessage } from 'src/services/neuron/neuronApi';
import { addIfpsMessageOrCid } from 'src/utils/ipfs/helpers';
import { addSenseItem, updateSenseItem } from '../../redux/sense.redux';
import { AdviserProps } from '../Sense';
import styles from './ActionBar.module.scss';

type Props = {
  id: string | undefined;
} & AdviserProps;

enum STEPS {
  // INITIAL,
  MESSAGE,
  AMOUNT,
}

function ActionBarWrapper({ id, adviser }: Props) {
  const [step, setStep] = useState(STEPS.MESSAGE);

  const [message, setMessage] = useState<string>('');
  const [amount, setAmount] = useState<number>(0);

  const address = useAppSelector(selectCurrentAddress);
  const dispatch = useAppDispatch();

  const { isIpfsInitialized, ipfsApi, senseApi } = useBackend();

  const [tx, setTx] = useState({
    hash: '',
    onSuccess: null,
  });

  const waitForTransaction = useWaitForTransaction(tx);

  const particle = id && isParticle(id);

  useEffect(() => {
    const { error, isLoading } = waitForTransaction;

    adviser.setLoading(isLoading);

    if (error) {
      adviser.setError(error);

      if (!id || !tx.hash) {
        return;
      }

      dispatch(
        updateSenseItem({
          chatId: id!,
          txHash: tx.hash,
          isSuccess: false,
        })
      );
    }
  }, [waitForTransaction, dispatch, adviser, id, tx.hash]);

  const { signingClient, signer } = useSigningClient();

  const signerIsReady = Boolean(signer && signingClient);

  useEffect(() => {
    setStep(STEPS.MESSAGE);
    setMessage('');
    setAmount(0);
  }, []);

  async function send() {
    if (!signerIsReady || !address || !ipfsApi || !senseApi) {
      return;
    }

    if (!(message || amount)) {
      adviser.setError('Message or amount is empty');
      adviser.setAdviserText('Message or amount is empty');
      return;
    }

    try {
      adviser.setLoading(true);

      adviser.setAdviserText('Preparing transaction...');

      const formattedAmount = [coin(amount || 1, BASE_DENOM)];

      const messageCid = await addIfpsMessageOrCid(message, { ipfsApi });

      const deps = {
        senseApi,
        signingClient: signingClient!,
      };

      const txHash = await (particle
        ? sendCyberlink(address, messageCid, id, deps)
        : sendTokensWithMessage(address, id!, formattedAmount, messageCid, deps));

      const optimisticMessage = {
        id: id!,

        item: {
          from: address,
          type: undefined,
          memo: messageCid,
          meta: {},
          id: address,
          address,
          transactionHash: txHash,
          timestamp: Date.now(),
        },
      };

      if (particle) {
        optimisticMessage.item.meta = {
          from: messageCid,
        };

        optimisticMessage.item.type = 'cyber.graph.v1beta1.MsgCyberlink';
      } else {
        optimisticMessage.item.meta = {
          amount: formattedAmount,
        };
        optimisticMessage.item.type = 'cyber.graph.v1beta1.MsgSend';
      }

      dispatch(addSenseItem(optimisticMessage));

      adviser.setAdviserText('Confirming transaction...');
      setTx({
        hash: txHash,
        onSuccess: () => {
          // update();

          dispatch(
            updateSenseItem({
              chatId: id!,
              txHash,
              isSuccess: true,
            })
          );

          setStep(STEPS.MESSAGE);
          setMessage('');
          setAmount(0);

          adviser.setLoading(false);
          adviser.setAdviserText('Message sent!');

          setTimeout(() => {
            adviser.setAdviserText('');
          }, 5000);
        },
      });
    } catch (error) {
      adviser.setLoading(false);

      if (error.message === 'Request rejected') {
        adviser.setAdviserText('');
        return;
      }

      console.dir(error);

      let { message } = error;

      if (message.includes('insufficient funds')) {
        message = (
          <div className={styles.error}>
            sending message needs at least 1 <DenomArr denomValue="boot" onlyImg /> <br />
            <Link
              to={{
                pathname: routes.teleport.swap.path,
                search: createSearchParams({
                  from: ibcDenomAtom,
                  to: 'boot',
                  amount: '1',
                }).toString(),
              }}
            >
              get BOOT
            </Link>
          </div>
        );
      }

      adviser.setError(message);
      adviser.setAdviserText(message);
    }
  }

  if (!id) {
    return null;
  }

  if (step === STEPS.MESSAGE) {
    return (
      <ActionBar>
        <Input
          classNameTextbox={styles.messageInput}
          isTextarea
          autoFocus
          onChange={(e) => setMessage(e.target.value)}
          value={message}
          placeholder="Send message"
        />
        <Button
          className={styles.sendBtn}
          onClick={send}
          disabled={!isIpfsInitialized || !signerIsReady || !senseApi}
        >
          ▲
        </Button>
        {/* <span>or</span> */}
        {/* <Button onClick={() => setStep(STEPS.AMOUNT)}>Send tokens</Button> */}
      </ActionBar>
    );
  }

  // if (step === STEPS.AMOUNT) {
  //   return (
  //     <ActionBar
  //       onClickBack={() => {
  //         setStep(STEPS.MESSAGE);
  //       }}
  //       button={{
  //         text: 'Confirm',
  //         onClick: send,
  //         disabled: !signerIsReady || !address || !(message || amount),
  //       }}
  //     >
  //       <InputNumber
  //         min={1}
  //         onChange={(value) => setAmount(+value)}
  //         value={amount}
  //         placeholder="Amount"
  //       />
  //       <span>🟢</span>
  //     </ActionBar>
  //   );
  // }

  return null;
}

export default ActionBarWrapper;
