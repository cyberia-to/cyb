import React, { useCallback, useMemo, useState } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { CHAIN_ID } from 'src/constants/config';
import usePassportByAddress from 'src/features/passport/hooks/usePassportByAddress';
import usePassportContract from 'src/features/passport/usePassportContract';
import { selectCurrentAddress } from 'src/redux/features/pocket';
import { useAppSelector } from 'src/redux/hooks';
import { routes } from 'src/routes';
import { Citizenship } from 'src/types/citizenship';
import { Networks } from 'src/types/networks';
import { fromBech32 } from 'src/utils/utils';

const RobotContext = React.createContext<{
  address: string | null;
  passport: Citizenship | undefined | null;
  isOwner: boolean;
  nickname: string | undefined;
  isLoading: boolean;
  addRefetch: (func: () => void) => void;
  refetchData: () => void;
  isFetched: boolean;
}>({
  address: null,
  passport: undefined,
  isOwner: false,
  isLoading: false,
  nickname: undefined,
  addRefetch: () => {},
  refetchData: () => {},
  isFetched: false,
});

export const useRobotContext = () => React.useContext(RobotContext);

/**
 * Complex logic change carefully
 *
 * TODO: refactor to use 1 passport hook
 */
function RobotContextProvider({ children }: { children: React.ReactNode }) {
  const params = useParams();
  const location = useLocation();
  const navigate = useNavigate();

  const [refetchFuncs, setRefetch] = useState<(() => void)[]>([]);

  const currentAddress = useAppSelector(selectCurrentAddress);
  const currentUserPassport = usePassportByAddress(currentAddress);

  const { accounts, isInitialized: isPocketInitialized } = useAppSelector(({ pocket }) => pocket);

  const { username } = params;
  const nickname = username?.includes('@') && username.substring(1, username.length);

  let { address } = params;

  const robotUrl = location.pathname.includes(routes.robot.path);
  let isOwner: boolean | undefined = false;

  if (robotUrl) {
    address = currentAddress || currentUserPassport.data?.owner;
    isOwner = true;
  } else if (nickname && currentUserPassport.data?.extension.nickname === nickname) {
    address = currentUserPassport.data.owner;
  }

  const checkIsOwner = useCallback(
    (address: string | undefined) => {
      if (!address) {
        return false;
      }

      const ownedAddresses: string[] = [];

      // if (currentUserPassport.data) {
      //   const { owner, extension } = currentUserPassport.data;

      //   if (owner === address || extension.nickname === nickname) {
      //     return true;
      //   }
      //   extension.addresses?.forEach(({ address: addr }) => {
      //     if (addr === address) {
      //       ownedAddresses.push(addr);
      //     }
      //   });
      // }

      if (accounts) {
        Object.keys(accounts).forEach((account) => {
          const { cyber } = accounts[account];

          // only bostrom for now
          if (!cyber) {
            return false;
          }

          const { bech32, keys } = cyber;

          if (bech32 === address && keys !== 'read-only') {
            ownedAddresses.push(bech32);
          }
        });
      }

      if (ownedAddresses.includes(address)) {
        return true;
      }

      return false;
    },
    [accounts]
  );

  if (!isOwner) {
    isOwner = checkIsOwner(address);
  }
  isOwner = isOwner as boolean;

  let query = {};
  if (address) {
    query = {
      active_passport: {
        address,
      },
    };
  } else if (nickname) {
    query = {
      passport_by_nickname: {
        nickname,
      },
    };
  }

  const passportContract = usePassportContract<Citizenship>({
    query,
    skip: isOwner,
  });

  const currentPassport = isOwner ? currentUserPassport : passportContract;
  let currentRobotAddress = address || currentPassport.data?.owner || null;

  if (robotUrl && currentUserPassport.data && currentAddress === currentUserPassport.data.owner) {
    navigate(
      location.pathname.replace(
        routes.robot.path,
        routes.robotPassport.getLink(currentUserPassport.data.extension.nickname)
      ),
      {
        replace: true,
      }
    );
  }

  if (CHAIN_ID === Networks.SPACE_PUSSY && currentRobotAddress) {
    currentRobotAddress = fromBech32(currentRobotAddress, 'pussy');
  }

  const addRefetch = useCallback((func: () => void) => {
    setRefetch((items) => [...items, func]);
  }, []);

  const refetchData = useCallback(() => {
    refetchFuncs.forEach((func) => func());
  }, [refetchFuncs]);

  const isLoading = currentPassport.loading || false;

  const isFetched =
    currentPassport.data !== undefined ||
    currentPassport.error ||
    // zero user
    (robotUrl && isPocketInitialized && !currentAddress);

  const contextValue = useMemo(
    () => ({
      address: currentRobotAddress,
      passport: currentPassport.data,
      isOwner,
      addRefetch,
      nickname: nickname || currentPassport.data?.extension.nickname,
      refetchData,
      isLoading,
      isFetched,
    }),
    [
      currentPassport.data,
      nickname,
      addRefetch,
      refetchData,
      isOwner,
      isFetched,
      isLoading,
      currentRobotAddress,
    ]
  );
  return <RobotContext.Provider value={contextValue}>{children}</RobotContext.Provider>;
}

export default RobotContextProvider;
