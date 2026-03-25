const initialState = {
  takeoff: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  relevance: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  load: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  delegation: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  lifetime: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  euler4Rewards: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
  communityPool: {
    cybAbsolute: 0,
    currentPrize: 0,
  },
};

const golReducer = (state = initialState, action) => {
  switch (action.type) {
    case 'CHANGE_GOL_LOAD': {
      return {
        ...state,
        load: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_GOL_DELEGATION': {
      return {
        ...state,
        delegation: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_GOL_LIFETIME': {
      return {
        ...state,
        lifetime: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_GOL_TAKEOFF': {
      return {
        ...state,
        takeoff: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_GOL_RELEVANCE': {
      return {
        ...state,
        relevance: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_GOL_EULER4REWARDS': {
      return {
        ...state,
        euler4Rewards: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }

    case 'CHANGE_COMMUNITY_POOL': {
      return {
        ...state,
        communityPool: {
          cybAbsolute: action.cybAbsolute,
          currentPrize: action.currentPrize,
        },
      };
    }
    default:
      return state;
  }
};

export default golReducer;
