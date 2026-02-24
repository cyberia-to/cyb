// import { combineReducers } from 'redux';

import commanderReducer from 'src/containers/application/Header/Commander/commander.redux';
import scriptingReducer from 'src/redux/reducers/scripting';
import passportsReducer from '../../features/passport/passports.redux';
import senseReducer from '../../features/sense/redux/sense.redux';
import signerReducer from '../features/signer';
import actionBarReducer from '../features/action-bar';
import timeHistoryReducer from '../../features/TimeHistory/redux/TimeHistory.redux';
import hubReducer from '../../pages/Hub/redux/hub';
import currentAccountReducer from '../features/currentAccount';
import ibcDenomReducer from '../features/ibcDenom';
import pocketReducer from '../features/pocket';
import warpReducer from '../features/warp';
import backendReducer from './backend';
import bandwidthReducer from './bandwidth';
import golReducer from './gol';

const rootReducer = {
  gol: golReducer,
  bandwidth: bandwidthReducer,
  pocket: pocketReducer,
  passports: passportsReducer,
  currentAccount: currentAccountReducer,
  backend: backendReducer,
  commander: commanderReducer,
  sense: senseReducer,
  warp: warpReducer,
  ibcDenom: ibcDenomReducer,
  scripting: scriptingReducer,
  hub: hubReducer,
  signer: signerReducer,
  actionBar: actionBarReducer,
  timeHistory: timeHistoryReducer,
};

export default rootReducer;
