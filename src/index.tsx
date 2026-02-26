/* eslint-disable import/prefer-default-export */
/* eslint-disable import/no-unused-modules */
// eslint-disable-next-line import/no-unused-modules
import 'core-js/stable';
import 'regenerator-runtime/runtime';

import { ApolloProvider } from '@apollo/client';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { createRoot } from 'react-dom/client';
import { Helmet } from 'react-helmet';
import { Provider } from 'react-redux';
import ErrorBoundary from './components/ErrorBoundary/ErrorBoundary';
import store from './redux/store';
import AppRouter from './router';

import './image/favicon.ico';
import './style/index.scss';
import './style/main.css';

// for boot loading
import './image/robot.svg';

import { localStorageKeys } from './constants/localStorageKeys';
import DataProvider from './contexts/appData';
import BackendProvider from './contexts/backend/backend';
import DeviceProvider from './contexts/device';
import HubProvider from './contexts/hub';
import IbcDenomProvider from './contexts/ibcDenom';
import NetworksProvider from './contexts/networks';
import SdkQueryClientProvider from './contexts/queryClient';
import CyberClientProvider from './contexts/queryCyberClient';
import ScriptingProvider from './contexts/scripting/scripting';
import SigningClientProvider from './contexts/signerClient';
import AdviserProvider from './features/adviser/context';
import apolloClient from './services/graphql';
import WebsocketsProvider from './websockets/context';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      // staleTime: 60 * 1000,
    },
  },
});

const container: HTMLElement | null = document.getElementById('root');

if (container === null) {
  throw new Error('Root container missing in index.html');
}

const root = createRoot(container);

import safeLocalStorage from './utils/safeLocalStorage';

safeLocalStorage.removeItem(localStorageKeys.settings.adviserAudio);

function Providers({ children }: { children: React.ReactNode }) {
  return (
    <Provider store={store}>
      <NetworksProvider>
        <QueryClientProvider client={queryClient}>
          <SdkQueryClientProvider>
            <CyberClientProvider>
              <SigningClientProvider>
                <HubProvider>
                  <IbcDenomProvider>
                    <WebsocketsProvider>
                      <DataProvider>
                        <ApolloProvider client={apolloClient}>
                          <BackendProvider>
                            <ScriptingProvider>
                              <DeviceProvider>
                                <AdviserProvider>
                                  <ErrorBoundary>{children}</ErrorBoundary>
                                </AdviserProvider>
                              </DeviceProvider>
                            </ScriptingProvider>
                          </BackendProvider>
                        </ApolloProvider>
                      </DataProvider>
                    </WebsocketsProvider>
                  </IbcDenomProvider>
                </HubProvider>
              </SigningClientProvider>
            </CyberClientProvider>
          </SdkQueryClientProvider>
        </QueryClientProvider>
      </NetworksProvider>
    </Provider>
  );
}

// Tauri debugging: F12/Cmd+Opt+I toggles native devtools, shake/4-finger-tap toggles eruda
if (process.env.IS_TAURI) {
  // Desktop: F12 or Cmd+Opt+I → native devtools via Tauri command
  window.addEventListener('keydown', (e) => {
    const isMac = navigator.platform.includes('Mac');
    const isF12 = e.key === 'F12';
    const isCmdOptI = e.key === 'i' && (isMac ? e.metaKey : e.ctrlKey) && e.altKey;
    if (isF12 || isCmdOptI) {
      e.preventDefault();
      import('@tauri-apps/api/core').then(({ invoke }) => invoke('toggle_devtools'));
    }
  });

  const toggleEruda = () => {
    import('eruda').then((eruda) => {
      if (!(window as any)._eruda) {
        eruda.default.init();
        (window as any)._eruda = true;
      } else {
        eruda.default.destroy();
        (window as any)._eruda = false;
      }
    });
  };

  // Mobile: shake → eruda in-app console
  if ('DeviceMotionEvent' in window) {
    let lastToggle = 0;
    window.addEventListener('devicemotion', (e) => {
      const acc = e.accelerationIncludingGravity;
      if (!acc) return;
      const force = Math.sqrt(acc.x! ** 2 + acc.y! ** 2 + acc.z! ** 2);
      if (force > 30 && Date.now() - lastToggle > 2000) {
        lastToggle = Date.now();
        toggleEruda();
      }
    });
  }

}

root.render(
  <Providers>
    <Helmet>
      <title>cyb: your immortal robot for the great web</title>
    </Helmet>
    <AppRouter />
    <ReactQueryDevtools position="bottom-right" />
  </Providers>
);
