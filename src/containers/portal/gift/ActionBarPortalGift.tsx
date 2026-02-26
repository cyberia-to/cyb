/* eslint-disable react-hooks/exhaustive-deps */
/* eslint-disable no-await-in-loop */
/* eslint-disable no-restricted-syntax */
/* eslint-disable jsx-a11y/control-has-associated-label */

import { toAscii, toBase64 } from '@cosmjs/encoding';
import { GasPrice } from '@cosmjs/launchpad';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { CHAIN_ID } from 'src/constants/config';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { useBackend } from 'src/contexts/backend/backend';
import { useSigningClient } from 'src/contexts/signerClient';
import useCurrentAddress from 'src/hooks/useCurrentAddress';
import useWaitForTransaction from 'src/hooks/useWaitForTransaction';
import { useAppDispatch } from 'src/redux/hooks';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { Nullable } from 'src/types';
import { Citizenship } from 'src/types/citizenship';
import { toHex } from 'src/utils/encoding';
import { getKeplr } from 'src/utils/keplrUtils';
import { ActionBar as ActionBarSteps, BtnGrd, ButtonIcon, Dots } from '../../../components';
import { addAddress, deleteAddress } from '../../../features/passport/passports.redux';
import imgCosmos from '../../../image/cosmos-2.svg';
import imgEth from '../../../image/Ethereum_logo_2014.svg';
import imgKeplr from '../../../image/keplr-icon.svg';
import imgMetaMask from '../../../image/mm-logo.svg';
import imgOsmosis from '../../../image/osmosis.svg';
import imgSpacePussy from '../../../image/space-pussy.svg';
import imgTerra from '../../../image/terra.svg';
import { groupMsg, trimString } from '../../../utils/utils';
import { TxHash } from '../hook/usePingTxs';
import {
  BOOT_ICON,
  CONSTITUTION_HASH,
  CONTRACT_ADDRESS_GIFT,
  CONTRACT_ADDRESS_PASSPORT,
} from '../utils';
import configTerraKeplr from './configTerraKeplr';
import { ClaimMsg } from './type';
import STEP_INFO from './utils';

const _gasPrice = GasPrice.fromString('0.001boot');

const proofAddressMsg = (address, nickname, signature) => {
  return {
    proof_address: {
      address,
      nickname,
      signature,
    },
  };
};

const deleteAddressMsg = (address, nickname) => {
  return {
    remove_address: {
      address,
      nickname,
    },
  };
};

const claimMsg = (
  nickname: string,
  giftClaimingAddress: string,
  giftAmount: number,
  proof: string[]
): ClaimMsg => {
  return {
    claim: {
      proof,
      gift_amount: giftAmount.toString(),
      gift_claiming_address: giftClaimingAddress,
      nickname,
    },
  };
};

type Props = {
  addressActive?: {
    bech32: string;
  };
  citizenship: Nullable<Citizenship>;
  updateTxHash?: (data: TxHash) => void;
  isClaimed: any;
  selectedAddress?: string;
  currentGift: any;
  activeStep: any;
  setStepApp: any;

  setLoadingGift: any;
  loadingGift: any;
  progressClaim: number;
  currentBonus: number;
};

function ActionBarPortalGift({
  addressActive,
  citizenship,
  updateTxHash = () => {},
  isClaimed,
  selectedAddress,
  currentGift,
  activeStep,
  setStepApp,
  setLoadingGift,
  loadingGift,
  progressClaim,
  currentBonus,
}: Props) {
  const { isIpfsInitialized, ipfsApi } = useBackend();

  const navigate = useNavigate();
  const location = useLocation();

  const { signer, signingClient, initSigner } = useSigningClient();
  const [selectMethod, setSelectMethod] = useState('');
  const [selectNetwork, setSelectNetwork] = useState('');
  const [signedMessageKeplr, setSignedMessageKeplr] = useState(null);

  const currentAddress = useCurrentAddress();

  const isGiftPage = location.pathname === '/gift';

  const [currentTx, setCurrentTx] = useState<{
    hash: string;
    onSuccess: () => void;
  }>();

  const dispatch = useAppDispatch();

  useWaitForTransaction({
    hash: currentTx?.hash,
    onSuccess: currentTx?.onSuccess,
  });

  useEffect(() => {
    const checkAddress = async () => {
      if (
        activeStep === STEP_INFO.STATE_PROVE_CHANGE_ACCOUNT ||
        activeStep === STEP_INFO.STATE_PROVE_CHECK_ACCOUNT
      ) {
        if (signer && addressActive !== null) {
          const [{ address }] = await signer.getAccounts();
          const { bech32 } = addressActive;
          if (address === bech32) {
            setStepApp(STEP_INFO.STATE_PROVE_SEND_SIGN);
          } else {
            setStepApp(STEP_INFO.STATE_PROVE_CHANGE_ACCOUNT);
          }
        }
      }
    };
    checkAddress();
  }, [signer, addressActive, activeStep, setStepApp]);

  const useAddressOwner = useMemo(() => {
    if (citizenship && addressActive !== null) {
      const { owner } = citizenship;
      const { name } = addressActive;
      if (name !== undefined && name !== null) {
        return (
          <>
            account
            <span style={{ color: '#36d6ae', padding: '0 5px' }}>{name}</span>
          </>
        );
      }
      return (
        <>
          address
          <span style={{ color: '#36d6ae', padding: '0 5px' }}>{trimString(owner, 10, 4)}</span>
        </>
      );
    }
    return '';
  }, [citizenship, addressActive]);

  const signMsgKeplr = useCallback(async () => {
    const keplrWindow = await getKeplr();
    if (keplrWindow && citizenship && selectNetwork !== '') {
      const { owner, extension } = citizenship;
      const { addresses } = extension;

      if (selectNetwork === 'columbus-5') {
        if (window.keplr?.experimentalSuggestChain) {
          await window.keplr.experimentalSuggestChain(configTerraKeplr());
        }
      }
      await keplrWindow.enable(selectNetwork);
      const signer = await keplrWindow.getOfflineSignerAuto(selectNetwork);

      const [{ address }] = await signer.getAccounts();

      if (addresses !== null && Object.keys(addresses).length > 0) {
        const result = Object.keys(addresses).filter((key) => addresses[key].address === address);

        if (result.length > 0) {
          setStepApp(STEP_INFO.STATE_PROVE_YOU_ADDED_ADDR);
          return null;
        }
      }

      const data = `${owner}:${CONSTITUTION_HASH}`;
      const res = await keplrWindow.signArbitrary(selectNetwork, address, data);

      const proveData = {
        pub_key: res.pub_key.value,
        signature: res.signature,
      };

      const signature = toBase64(toAscii(JSON.stringify(proveData)));
      setSignedMessageKeplr({ signature, address });
      setStepApp(STEP_INFO.STATE_PROVE_CHECK_ACCOUNT);
    }
    return null;
  }, [citizenship, selectNetwork, setStepApp]);

  const signMsgETH = useCallback(async () => {
    if (window.ethereum && citizenship) {
      const { owner, extension } = citizenship;
      const { addresses } = extension;

      const { ethereum } = window;

      const accounts = await ethereum.request({
        method: 'eth_requestAccounts',
      });

      const address = accounts[0];
      const message = `${owner}:${CONSTITUTION_HASH}`;
      const msg = `0x${toHex(new TextEncoder().encode(message))}`;
      const from = address;

      if (addresses !== null && Object.keys(addresses).length > 0) {
        const result = Object.keys(addresses).filter((key) => addresses[key].address === address);

        if (result.length > 0) {
          setStepApp(STEP_INFO.STATE_PROVE_YOU_ADDED_ADDR);
          return null;
        }
      }

      const signature = await ethereum.request({
        method: 'personal_sign',
        params: [msg, from, 'proveAddress'],
      });

      setSignedMessageKeplr({ signature, address });
      setStepApp(STEP_INFO.STATE_PROVE_CHECK_ACCOUNT);
    }
    return null;
  }, [citizenship, setStepApp]);

  const sendSignedMessage = useCallback(async () => {
    if (signer && signingClient && citizenship && signedMessageKeplr !== null) {
      const { nickname } = citizenship.extension;

      const msgObject = proofAddressMsg(
        signedMessageKeplr.address,
        nickname,
        signedMessageKeplr.signature
      );

      try {
        const [{ address }] = await signer.getAccounts();

        const executeResponseResult = await signingClient.execute(
          address,
          CONTRACT_ADDRESS_PASSPORT,
          msgObject,
          Soft3MessageFactory.fee(2),
          'cyber'
        );

        if (executeResponseResult.code === 0) {
          updateTxHash({
            status: 'pending',
            txHash: executeResponseResult.transactionHash,
          });

          setCurrentTx({
            hash: executeResponseResult.transactionHash,
            onSuccess: () => {
              dispatch(
                addAddress({
                  address: signedMessageKeplr.address,
                  currentAddress,
                })
              );
            },
          });

          setStepApp(STEP_INFO.STATE_PROVE_IN_PROCESS);
          if (setLoadingGift) {
            setLoadingGift(true);
          }
        }

        if (executeResponseResult.code) {
          updateTxHash({
            txHash: executeResponseResult?.transactionHash,
            status: 'error',
            rawLog: executeResponseResult?.rawLog.toString(),
          });
        }
        if (isIpfsInitialized) {
          ipfsApi?.addContent(signedMessageKeplr.address);
        }
      } catch (error) {
        console.log('error', error);
        setStepApp(STEP_INFO.STATE_PROVE);
      }
    }
  }, [
    signer,
    signingClient,
    citizenship,
    signedMessageKeplr,
    isIpfsInitialized,
    ipfsApi,
    currentAddress,
    dispatch,
    setLoadingGift,
    setStepApp,
    updateTxHash,
  ]);

  const claim = useCallback(async () => {
    try {
      if (!signer) {
        if (initSigner) {
          initSigner();
        }
      }

      if (
        signer &&
        signingClient &&
        selectedAddress !== null &&
        currentGift !== null &&
        citizenship
      ) {
        const { nickname } = citizenship.extension;
        if (Object.keys(currentGift).length > 0) {
          const msgs: ClaimMsg[] = [];
          Object.keys(currentGift).forEach((key) => {
            const { address, proof, amount } = currentGift[key];
            const msgObject = claimMsg(nickname, address, amount, proof);
            msgs.push(msgObject);
          });
          const { bech32Address, isNanoLedger } = await signer.keplr.getKey(CHAIN_ID);

          if (!msgs.length) {
            return;
          }

          let elementMsg = msgs;

          if (isNanoLedger) {
            elementMsg = groupMsg(msgs, 1)[0] as ClaimMsg[];
          }

          const multiplier = new BigNumber(2.5)
            .multipliedBy(Object.keys(elementMsg).length)
            .toNumber();

          const executeResponseResult = await signingClient.executeArray(
            bech32Address,
            CONTRACT_ADDRESS_GIFT,
            elementMsg,
            Soft3MessageFactory.fee(multiplier),
            'cyber'
          );

          console.log('executeResponseResult', executeResponseResult);
          if (executeResponseResult.code === 0) {
            updateTxHash({
              status: 'pending',
              txHash: executeResponseResult.transactionHash,
            });

            if (setLoadingGift) {
              setLoadingGift(true);
            }
            setStepApp(STEP_INFO.STATE_CLAIM_IN_PROCESS);
            // setStep(STEP_INIT);
          }

          if (executeResponseResult.code) {
            updateTxHash({
              txHash: executeResponseResult?.transactionHash,
              status: 'error',
              rawLog: executeResponseResult?.rawLog.toString(),
            });
          }
        }
      }
    } catch (error) {
      console.log('error', error);
      // setStep(STEP_INIT);
      setStepApp(STEP_INFO.STATE_CLAIME);
    }
  }, [
    signer,
    signingClient,
    selectedAddress,
    currentGift,
    citizenship,
    initSigner,
    setLoadingGift, // setStep(STEP_INIT);
    setStepApp,
    updateTxHash,
  ]);

  const isProve = useMemo(() => {
    if (citizenship && !citizenship.extension.addresses) {
      return false;
    }

    if (
      !!citizenship?.extension.addresses &&
      Object.keys(citizenship.extension.addresses).length <= 8
    ) {
      return false;
    }

    return true;
  }, [citizenship]);

  const isClaime = useMemo(() => {
    if (isClaimed !== undefined && isClaimed !== null && !isClaimed) {
      return false;
    }
    return true;
  }, [isClaimed]);

  const useDeleteAddress = useCallback(async () => {
    if (!signer || !signingClient || !selectedAddress || !citizenship) {
      return;
    }

    // not possible to delete cyber address
    if (selectedAddress.match(PATTERN_CYBER)) {
      return;
    }

    const { nickname } = citizenship.extension;
    const msgObject = deleteAddressMsg(selectedAddress, nickname);
    try {
      const [{ address }] = await signer.getAccounts();
      const executeResponseResult = await signingClient.execute(
        address,
        CONTRACT_ADDRESS_PASSPORT,
        msgObject,
        Soft3MessageFactory.fee(2),
        'cyber'
      );

      if (executeResponseResult.code === 0) {
        updateTxHash({
          status: 'pending',
          txHash: executeResponseResult.transactionHash,
        });
        setCurrentTx({
          hash: executeResponseResult.transactionHash,
          onSuccess: () => {
            dispatch(
              deleteAddress({
                address: selectedAddress,
                currentAddress,
              })
            );
            setStepApp(STEP_INFO.STATE_INIT);
          },
        });
        setStepApp(STEP_INFO.STATE_DELETE_IN_PROCESS);
      }

      if (executeResponseResult.code) {
        updateTxHash({
          txHash: executeResponseResult?.transactionHash,
          status: 'error',
          rawLog: executeResponseResult?.rawLog.toString(),
        });
      }
    } catch (error) {
      console.log('error', error);
      setStepApp(STEP_INFO.STATE_INIT);
    }
  }, [
    signer,
    signingClient,
    selectedAddress,
    citizenship,
    currentAddress,
    dispatch,
    setStepApp,
    updateTxHash,
  ]);

  const useGetSelectAddress = useMemo(() => {
    if (selectedAddress && selectedAddress !== null) {
      return (
        <span style={{ color: '#36d6ae', padding: '0 5px' }}>
          {trimString(selectedAddress, 10, 4)}
        </span>
      );
    }
    return '';
  }, [selectedAddress]);

  const onClickMMSigner = () => {
    setSelectNetwork('eth');
    setSelectMethod('MetaMask');
  };

  if (activeStep === STEP_INFO.STATE_INIT_NULL) {
    return (
      <ActionBarSteps>
        <BtnGrd onClick={() => navigate('/citizenship')} text="get citizenship" />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_INIT_PROVE || activeStep === STEP_INFO.STATE_PROVE) {
    return (
      <ActionBarSteps>
        <BtnGrd
          disabled={isProve}
          onClick={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}
          text="prove address"
        />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_INIT_CLAIM) {
    return (
      <ActionBarSteps>
        <BtnGrd onClick={() => setStepApp(STEP_INFO.STATE_CLAIME)} text="go to claim" />
      </ActionBarSteps>
    );
  }

  if (
    activeStep === STEP_INFO.STATE_CLAIME_TO_PROVE ||
    activeStep === STEP_INFO.STATE_GIFT_NULL_ALL
  ) {
    return (
      <ActionBarSteps gridGap="35px">
        <BtnGrd
          disabled={isProve}
          onClick={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}
          text="prove one more address"
        />
        <BtnGrd onClick={() => navigate('/teleport')} text={`buy ${BOOT_ICON}`} />
      </ActionBarSteps>
    );
  }

  if (
    activeStep === STEP_INFO.STATE_PROVE_CONNECT ||
    activeStep === STEP_INFO.STATE_PROVE_YOU_ADDED_ADDR
  ) {
    return (
      <ActionBarSteps
        onClickBack={() => setStepApp(STEP_INFO.STATE_INIT_PROVE)}
        button={{
          onClick: () =>
            setStepApp(
              selectMethod === 'keplr'
                ? STEP_INFO.STATE_PROVE_SIGN_KEPLR
                : STEP_INFO.STATE_PROVE_SIGN_MM
            ),
          text: 'connect',
          disabled: selectMethod === '',
        }}
      >
        {!process.env.IS_TAURI && (
          <ButtonIcon
            onClick={() => setSelectMethod('keplr')}
            active={selectMethod === 'keplr'}
            img={imgKeplr}
            text="keplr"
          />
        )}
        <ButtonIcon
          onClick={() => onClickMMSigner()}
          active={selectMethod === 'MetaMask'}
          img={imgMetaMask}
          text="metaMask"
        />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_PROVE_SIGN_KEPLR) {
    return (
      <ActionBarSteps
        onClickBack={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}
        button={{
          onClick: () => signMsgKeplr(),
          text: 'sign Moon Code in keplr',
          disabled: selectNetwork === '',
        }}
      >
        <ButtonIcon
          onClick={() => setSelectNetwork('cosmoshub')}
          active={selectNetwork === 'cosmoshub'}
          img={imgCosmos}
          text="cosmoshub"
        />
        <ButtonIcon
          onClick={() => setSelectNetwork('osmosis')}
          active={selectNetwork === 'osmosis'}
          img={imgOsmosis}
          text="osmosis"
        />

        {!isGiftPage && (
          <ButtonIcon
            onClick={() => setSelectNetwork('space-pussy')}
            active={selectNetwork === 'space-pussy'}
            img={imgSpacePussy}
            text="space pussy"
          />
        )}

        <ButtonIcon
          onClick={() => setSelectNetwork('columbus-5')}
          active={selectNetwork === 'columbus-5'}
          img={imgTerra}
          text="terra"
        />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_PROVE_SIGN_MM) {
    return (
      <ActionBarSteps
        onClickBack={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}
        button={{
          onClick: () => signMsgETH(),
          text: 'sign Moon Code in metamask',
        }}
      >
        <ButtonIcon
          onClick={() => setSelectNetwork('eth')}
          active={selectNetwork === 'eth'}
          img={imgEth}
          text="eth"
        />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_PROVE_CHECK_ACCOUNT) {
    return (
      <ActionBarSteps onClickBack={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}>
        address comparison <Dots />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_PROVE_CHANGE_ACCOUNT) {
    return (
      <ActionBarSteps onClickBack={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}>
        choose {useAddressOwner} in keplr
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_PROVE_SEND_SIGN) {
    return (
      <ActionBarSteps onClickBack={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}>
        <BtnGrd onClick={() => sendSignedMessage()} text="send signature of Moon Code" />
      </ActionBarSteps>
    );
  }

  if (loadingGift && activeStep === STEP_INFO.STATE_RELEASE) {
    return (
      <ActionBarSteps>
        <BtnGrd pending />
      </ActionBarSteps>
    );
  }

  if (
    (activeStep === STEP_INFO.STATE_INIT_RELEASE || activeStep === STEP_INFO.STATE_RELEASE) &&
    isClaimed
  ) {
    return (
      <ActionBarSteps>
        <BtnGrd onClick={() => setStepApp(STEP_INFO.STATE_RELEASE_INIT)} text="go to release" />
      </ActionBarSteps>
    );
  }

  if (activeStep === STEP_INFO.STATE_CLAIME) {
    return (
      <ActionBarSteps>
        <BtnGrd
          disabled={isProve}
          onClick={() => setStepApp(STEP_INFO.STATE_PROVE_CONNECT)}
          text="prove one more address"
        />
        <BtnGrd disabled={isClaime} onClick={() => claim()} text="claim" />
      </ActionBarSteps>
    );
  }
  if (activeStep === STEP_INFO.STATE_DELETE_ADDRESS) {
    return (
      <ActionBarSteps
        onClickBack={() => setStepApp(STEP_INFO.STATE_INIT)}
        button={{ onClick: useDeleteAddress, text: 'delete' }}
      >
        you want to delete {useGetSelectAddress} from your passport
      </ActionBarSteps>
    );
  }

  if (
    activeStep === STEP_INFO.STATE_CLAIM_IN_PROCESS ||
    activeStep === STEP_INFO.STATE_PROVE_IN_PROCESS ||
    activeStep === STEP_INFO.STATE_DELETE_IN_PROCESS
  ) {
    return (
      <ActionBarSteps>
        <BtnGrd pending />
      </ActionBarSteps>
    );
  }

  return null;
}

export default ActionBarPortalGift;
