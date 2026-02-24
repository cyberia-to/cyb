import { toAscii, toBase64 } from '@cosmjs/encoding';
import { coins, GasPrice } from '@cosmjs/launchpad';
import BigNumber from 'bignumber.js';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { connect } from 'react-redux';
import { BASE_DENOM, CHAIN_ID } from 'src/constants/config';
import { useBackend } from 'src/contexts/backend/backend';
import { useDevice } from 'src/contexts/device';
import { useQueryClient } from 'src/contexts/queryClient';
import { useSigningClient } from 'src/contexts/signerClient';

import { useAdviser } from 'src/features/adviser/context';
import { getPassport } from 'src/features/passport/passports.redux';
import { useAppDispatch } from 'src/redux/hooks';
import Soft3MessageFactory from 'src/services/soft.js/api/msgs';
import { Nullable } from 'src/types';
import { getKeplr } from 'src/utils/keplrUtils';
import Carousel from '../../../components/Tabs/Carousel/CarouselOld/CarouselOld';
import useSetActiveAddress from '../../../hooks/useSetActiveAddress';
import { getCredit } from '../../../utils/search/utils';
import { MainContainer, MoonAnimation, Stars } from '../components';
import { AvataImgIpfs } from '../components/avataIpfs';
import { Avatar, InitKeplr, InputNickname, Passport, Rules, SetupKeplr } from '../stateComponent';
import {
  CONSTITUTION_HASH,
  CONTRACT_ADDRESS_PASSPORT,
  getNumTokens,
  getPassportByNickname,
} from '../utils';
import ActionBar from './ActionBar';
import Info from './Info';
import { steps } from './utils';

const portalConfirmed = require('../../../sounds/portalConfirmed112.mp3');
const portalAmbient = require('../../../sounds/portalAmbient112.mp3');

const portalConfirmedObg = new Audio(portalConfirmed);
const portalAmbientObg = new Audio(portalAmbient);

const playPortalConfirmed = () => {
  portalConfirmedObg.play();
};

const playPortalAmbient = () => {
  portalAmbientObg.loop = true;
  portalAmbientObg.play();
};

const stopPortalAmbient = () => {
  portalAmbientObg.loop = false;
  portalAmbientObg.pause();
  portalAmbientObg.currentTime = 0;
};

const {
  STEP_INIT,
  STEP_NICKNAME_CHOSE,
  STEP_NICKNAME_INVALID,
  STEP_NICKNAME_APROVE,
  STEP_RULES,
  STEP_AVATAR_UPLOAD,
  STEP_KEPLR_INIT,
  STEP_KEPLR_INIT_INSTALLED,
  STEP_KEPLR_INIT_CHECK_FNC,
  STEP_KEPLR_SETUP,
  STEP_KEPLR_CONNECT,
  STEP_CHECK_ADDRESS,
  STEP_CHECK_ADDRESS_CHECK_FNC,
  STEP_ACTIVE_ADD,
  STEP_KEPLR_REGISTER,
  STEP_DONE,
  STEP_CHECK_GIFT,
} = steps;

// const items1 = {
//   [STEP_NICKNAME_CHOSE]: 'nickname',
//   [STEP_RULES]: 'rules',
//   [STEP_AVATAR_UPLOAD]: 'avatar',
//   [STEP_KEPLR_INIT]: 'install keplr',
//   [STEP_KEPLR_SETUP]: 'setup keplr',
//   [STEP_KEPLR_CONNECT]: 'connect keplr',
//   [STEP_CHECK_ADDRESS]: 'check address',
//   [STEP_KEPLR_REGISTER]: 'register',
//   [STEP_DONE]: 'passport look',
// };

const items = [
  {
    title: 'nickname',
    step: STEP_NICKNAME_CHOSE,
  },
  {
    title: 'avatar',
    step: STEP_AVATAR_UPLOAD,
  },
  {
    title: 'install keplr',
    step: STEP_KEPLR_INIT,
  },
  {
    title: 'setup keplr',
    step: STEP_KEPLR_SETUP,
  },
  {
    title: 'active address',
    step: STEP_CHECK_ADDRESS,
  },
  {
    title: 'code',
    step: STEP_RULES,
  },
  {
    title: 'passport',
    step: STEP_KEPLR_REGISTER,
  },
];

const _gasPrice = GasPrice.fromString('0.001boot');

const calculatePriceNicname = (valueNickname) => {
  let funds = [];
  if (valueNickname.length < 8) {
    const exponent = 8 - valueNickname.length;
    const base = new BigNumber(10).pow(exponent);
    const priceName = new BigNumber(1000000).multipliedBy(base).toNumber();
    funds = coins(priceName, BASE_DENOM);
  }

  return funds;
};

function GetCitizenship({ defaultAccount }) {
  const { isMobile: mobile } = useDevice();
  const { isIpfsInitialized, ipfsApi } = useBackend();
  const dispatch = useAppDispatch();

  const queryClient = useQueryClient();
  const { signer, signingClient } = useSigningClient();
  const { addressActive } = useSetActiveAddress(defaultAccount);
  const [step, setStep] = useState(STEP_INIT);
  const [valueNickname, setValueNickname] = useState('');
  // const [validNickname, setValidNickname] = useState(true);
  const [avatarIpfs, setAvatarIpfs] = useState<Nullable<string>>(null);
  const [avatarImg, setAvatarImg] = useState(null);
  const [txHash, setTxHash] = useState(null);
  const [registerDisabled, setRegisterDisabled] = useState(false);
  const [signedMessage, setSignedMessage] = useState(null);
  const [counCitizenshipst, setCounCitizenshipst] = useState(0);
  const inputOpenFileRef = useRef();

  useEffect(() => {
    playPortalAmbient();

    return () => {
      stopPortalAmbient();
    };
  }, []);

  useEffect(() => {
    const localStorageNickname = localStorage.getItem('nickname');
    if (localStorageNickname !== null) {
      const localStorageNicknameData = JSON.parse(localStorageNickname);
      setValueNickname(localStorageNicknameData);

      const localStorageAvatarCid = localStorage.getItem('avatarCid');
      if (localStorageAvatarCid !== null) {
        const localStorageAvatarCidData = JSON.parse(localStorageAvatarCid);
        setAvatarIpfs(localStorageAvatarCidData);
        setStep(STEP_KEPLR_INIT_CHECK_FNC);
      } else {
        setStep(STEP_NICKNAME_CHOSE);
      }
    } else {
      setStep(STEP_INIT);
    }
  }, []);

  useEffect(() => {
    const getCounCitizenshipst = async () => {
      if (queryClient) {
        const respnseNumTokens = await getNumTokens(queryClient);
        if (respnseNumTokens?.count) {
          setCounCitizenshipst(parseFloat(respnseNumTokens.count));
        }
      }
    };
    getCounCitizenshipst();
  }, [queryClient]);

  useEffect(() => {
    if (!avatarImg || !ipfsApi || !isIpfsInitialized) {
      return;
    }

    (async () => {
      try {
        const cid = await ipfsApi.addContent(avatarImg);
        setAvatarIpfs(cid);
      } catch (error) {
        console.error(error);
      }
    })();
  }, [isIpfsInitialized, ipfsApi, avatarImg]);

  useEffect(() => {
    const getKeplrSetup = async () => {
      if (step === STEP_KEPLR_INIT_CHECK_FNC) {
        if (window.keplr === undefined) {
          setStep(STEP_KEPLR_INIT_CHECK_FNC);
        } else {
          const offlineSigner = window.getOfflineSigner(CHAIN_ID);
          try {
            const [{ address }] = await offlineSigner.getAccounts();
            if (addressActive !== null) {
              const { bech32 } = addressActive;
              if (bech32 === address) {
                setStep(STEP_CHECK_ADDRESS_CHECK_FNC);
              } else {
                setStep(STEP_KEPLR_CONNECT);
              }
            } else {
              setStep(STEP_KEPLR_CONNECT);
            }
          } catch (_error) {
            setStep(STEP_KEPLR_SETUP);
          }
        }
      }
    };
    getKeplrSetup();
  }, [step, addressActive]);

  const dispatchGetPassport = useCallback(() => {
    if (step === STEP_DONE) {
      queryClient &&
        addressActive?.bech32 &&
        dispatch(getPassport({ address: addressActive.bech32, queryClient }));
    }
  }, [step, queryClient, dispatch, addressActive]);

  useEffect(() => {
    const confirmTx = async () => {
      if (queryClient && txHash !== null && txHash.status === 'pending') {
        const response = await queryClient.getTx(txHash.txHash);
        console.log('response :>> ', response);
        if (response && response !== null) {
          if (response.code === 0) {
            setTxHash((item) => ({
              ...item,
              status: 'confirmed',
            }));

            dispatchGetPassport();

            try {
              playPortalConfirmed();
            } catch (error) {
              console.log('error', error);
            }
            localStorage.removeItem('nickname');
            localStorage.removeItem('avatarCid');
            setStep(STEP_CHECK_GIFT);

            return;
          }
          if (response.code) {
            setTxHash((item) => ({
              ...item,
              status: 'error',
              rawLog: response.rawLog.toString(),
            }));
            setStep(STEP_KEPLR_REGISTER);
            return;
          }
        }
        setTimeout(confirmTx, 1500);
      }
    };
    confirmTx();
  }, [queryClient, txHash, dispatchGetPassport]);

  const usePriceNickname = useMemo(() => {
    if (valueNickname.length < 8) {
      const priceNickname = calculatePriceNicname(valueNickname);
      if (priceNickname.length > 0) {
        const [{ amount: amountPrice, denom: denomPrice }] = priceNickname;

        return { amountPrice, denomPrice };
      }
    }
    return null;
  }, [valueNickname]);

  const getBalanceAndNickname = useCallback(
    async (bech32) => {
      if (usePriceNickname !== null) {
        const { amountPrice } = usePriceNickname;
        if (queryClient) {
          const getBalance = await queryClient.getBalance(bech32, BASE_DENOM);
          const { amount } = getBalance;
          if (parseFloat(amount) === 0) {
            setRegisterDisabled(false);
          } else if (parseFloat(amountPrice) > parseFloat(amount)) {
            setRegisterDisabled(false);
          } else {
            setRegisterDisabled(true);
          }
        }
      } else {
        setRegisterDisabled(true);
      }
    },
    [queryClient, usePriceNickname]
  );

  useEffect(() => {
    const checkAddress = async () => {
      if (step === STEP_CHECK_ADDRESS_CHECK_FNC && queryClient && addressActive !== null) {
        const { bech32 } = addressActive;
        const response = await queryClient.getAccount(bech32);
        await getBalanceAndNickname(bech32);
        if (response && Object.hasOwn(response, 'accountNumber')) {
          setStep(STEP_RULES);
        } else {
          setStep(STEP_ACTIVE_ADD);
        }
      }
    };
    checkAddress();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [queryClient, addressActive, step, getBalanceAndNickname]);

  const checkAddressNetwork = useCallback(async () => {
    if (step === STEP_ACTIVE_ADD && queryClient && addressActive !== null) {
      const { bech32 } = addressActive;
      const response = await queryClient.getAccount(bech32);
      console.log('response', response);
      if (response && Object.hasOwn(response, 'accountNumber')) {
        setStep(STEP_RULES);
      } else {
        const responseCredit = await getCredit(bech32);
        if (responseCredit !== null && responseCredit.data === 'ok') {
          checkAddressNetwork();
        }
      }
    }
  }, [queryClient, addressActive, step]);

  const onClickSignMoonCode = async () => {
    const keplrWindow = await getKeplr();

    if (keplrWindow) {
      await keplrWindow.enable(CHAIN_ID);
      const signer = await keplrWindow.getOfflineSignerAuto(CHAIN_ID);
      const [{ address }] = await signer.getAccounts();
      const data = `${address}:${CONSTITUTION_HASH}`;

      const res = await keplrWindow.signArbitrary(CHAIN_ID, address, data);
      const proveData = {
        pub_key: res.pub_key.value,
        signature: res.signature,
      };
      const signature = toBase64(toAscii(JSON.stringify(proveData)));
      setSignedMessage(signature);
      setStep(STEP_KEPLR_REGISTER);
    }
  };

  const fncClearAvatar = () => {
    localStorage.removeItem('avatarCid');
    setAvatarIpfs(null);
    setAvatarImg(null);
  };

  const checkNickname = useCallback(async () => {
    try {
      if (queryClient) {
        const responsData = await getPassportByNickname(queryClient, valueNickname);
        if (responsData === null) {
          setStep(STEP_NICKNAME_APROVE);
        } else {
          setStep(STEP_NICKNAME_INVALID);
        }
      }
    } catch (error) {
      console.log('error', error);
      setStep(STEP_NICKNAME_INVALID);
    }
  }, [queryClient, valueNickname]);

  const setupNickname = useCallback(() => {
    localStorage.setItem('nickname', JSON.stringify(valueNickname));
    setStep(STEP_AVATAR_UPLOAD);
  }, [valueNickname]);

  const uploadAvatarImg = useCallback(() => {
    if (avatarIpfs !== null) {
      localStorage.setItem('avatarCid', JSON.stringify(avatarIpfs));
      setStep(STEP_KEPLR_INIT_CHECK_FNC);
    }
  }, [avatarIpfs]);

  const onClickRegister = useCallback(async () => {
    if (signer && signingClient) {
      try {
        const [{ address }] = await signer.getAccounts();
        console.log('create_passport', {
          signedMessage,
          valueNickname,
          avatarIpfs,
        });
        const msgObject = {
          create_passport: {
            avatar: avatarIpfs,
            nickname: valueNickname,
            signature: signedMessage,
          },
        };
        const funds = calculatePriceNicname(valueNickname);
        console.log('funds', funds);
        const executeResponseResult = await signingClient.execute(
          address,
          CONTRACT_ADDRESS_PASSPORT,
          msgObject,
          Soft3MessageFactory.fee(2.5),
          'cyber',
          funds
        );
        console.log('executeResponseResult', executeResponseResult);
        if (executeResponseResult.code === 0) {
          setTxHash({
            status: 'pending',
            txHash: executeResponseResult.transactionHash,
          });
          setStep(STEP_DONE);
        }

        if (executeResponseResult.code) {
          setTxHash({
            txHash: executeResponseResult?.transactionHash,
            status: 'error',
            rawLog: executeResponseResult?.rawLog.toString(),
          });
        }

        if (isIpfsInitialized) {
          ipfsApi?.addContent(valueNickname).then((cid) => {
            console.log('pin cid nickname', cid);
          });
          ipfsApi?.addContent(address).then((cid) => {
            console.log('pin cid address', cid);
          });
        }
      } catch (error) {
        console.log('error', error);
      }
    }
  }, [
    valueNickname,
    avatarIpfs,
    signer,
    signingClient,
    signedMessage,
    isIpfsInitialized,
    ipfsApi?.addContent,
  ]);

  const onChangeNickname = useCallback((e) => {
    const { value } = e.target;
    if (value.match(/^([a-z0-9-]*)$/g)) {
      setValueNickname(value);
    }
  }, []);

  const showOpenFileDlg = () => {
    inputOpenFileRef.current.click();
  };

  const onFilePickerChange = (files) => {
    const file = files.current.files[0];
    setAvatarImg(file);
  };

  let content;

  if (
    step === STEP_NICKNAME_CHOSE ||
    step === STEP_NICKNAME_APROVE ||
    step === STEP_NICKNAME_INVALID
  ) {
    content = (
      <InputNickname
        step={step}
        valueNickname={valueNickname}
        onChangeNickname={onChangeNickname}
      />
    );
  }

  if (step === STEP_AVATAR_UPLOAD) {
    content = (
      <Avatar
        valueNickname={valueNickname}
        upload={avatarIpfs === null}
        setAvatarImg={setAvatarImg}
        fncClearAvatar={fncClearAvatar}
        inputOpenFileRef={inputOpenFileRef}
        onFilePickerChange={onFilePickerChange}
        showOpenFileDlg={showOpenFileDlg}
        avatar={avatarIpfs === null ? 'upload avatar' : <AvataImgIpfs cidAvatar={avatarIpfs} />}
      />
    );
  }

  if (
    step === STEP_KEPLR_INIT ||
    step === STEP_KEPLR_INIT_CHECK_FNC ||
    step === STEP_KEPLR_INIT_INSTALLED
  ) {
    content = <InitKeplr />;
  }

  if (step === STEP_KEPLR_SETUP || step === STEP_KEPLR_CONNECT) {
    content = <SetupKeplr />;
  }

  // if (step === STEP_KEPLR_CONNECT) {
  //   content = <ConnectKeplr />;
  // }

  if (step === STEP_RULES) {
    content = <Rules />;
  }

  if (
    step === STEP_CHECK_ADDRESS ||
    step === STEP_CHECK_ADDRESS_CHECK_FNC ||
    step === STEP_ACTIVE_ADD ||
    step === STEP_KEPLR_REGISTER
  ) {
    content = (
      <Passport
        valueNickname={valueNickname}
        txs={txHash}
        avatar={<AvataImgIpfs cidAvatar={avatarIpfs} />}
        addressActive={addressActive}
      />
    );
  }

  if (step === STEP_DONE || step === STEP_CHECK_GIFT) {
    content = (
      <Passport
        valueNickname={valueNickname}
        avatar={<AvataImgIpfs cidAvatar={avatarIpfs} />}
        txs={txHash}
        addressActive={addressActive}
      />
    );
  }

  const useDisableNext = useMemo(() => {
    return true;
  }, []);

  const { setAdviser } = useAdviser();

  useEffect(() => {
    setAdviser(
      <Info
        nickname={valueNickname}
        stepCurrent={step}
        valuePriceNickname={usePriceNickname}
        registerDisabled={registerDisabled}
        setStep={setStep}
        counCitizenshipst={counCitizenshipst}
        mobile={mobile}
      />
    );
  }, [
    setAdviser,
    valueNickname,
    step,
    usePriceNickname,
    registerDisabled,
    counCitizenshipst,
    mobile,
  ]);

  return (
    <>
      <MainContainer>
        <Stars />

        {(step === STEP_INIT || !mobile) && <MoonAnimation stepCurrent={step} />}

        {step !== STEP_INIT && step !== STEP_CHECK_GIFT && (
          <Carousel
            slides={items}
            activeStep={Math.floor(step)}
            setStep={setStep}
            disableNext={useDisableNext}
          />
        )}
        {/* {step !== STEP_INIT && step !== STEP_CHECK_GIFT && (
            <ScrollableTabs items={items} active={step} setStep={setStep} />
          )} */}
        {content}
        {/* <button type="button" onClick={() => checkKeplr()}>
          keplr
        </button> */}
      </MainContainer>

      {!mobile && (
        <ActionBar
          step={step}
          valueNickname={valueNickname}
          signedMessage={signedMessage}
          setStep={setStep}
          setupNickname={setupNickname}
          checkNickname={checkNickname}
          uploadAvatarImg={uploadAvatarImg}
          avatarImg={avatarImg}
          setAvatarImg={setAvatarImg}
          avatarIpfs={avatarIpfs}
          onClickRegister={onClickRegister}
          checkAddressNetwork={checkAddressNetwork}
          registerDisabled={registerDisabled}
          showOpenFileDlg={showOpenFileDlg}
          onClickSignMoonCode={onClickSignMoonCode}
        />
      )}
    </>
  );
}

const mapStateToProps = (store) => {
  return {
    defaultAccount: store.pocket.defaultAccount,
  };
};

export default connect(mapStateToProps)(GetCitizenship);
