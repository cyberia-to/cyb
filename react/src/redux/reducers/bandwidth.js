const initialState = {
  bandwidth: {
    remained: 0,
    maxValue: 0,
  },
};

const bandwidthReducer = (state = initialState, action) => {
  switch (action.type) {
    case 'SET_BANDWIDTH': {
      return {
        ...state,
        bandwidth: {
          remained: action.remained,
          maxValue: action.maxValue,
        },
      };
    }

    default:
      return state;
  }
};

export default bandwidthReducer;
