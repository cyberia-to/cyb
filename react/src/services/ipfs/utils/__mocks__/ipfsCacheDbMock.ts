export const mockCacheDbGet = jest.fn();
export const mockCacheDbAdd = jest.fn();

const mock = jest.fn().mockImplementation(() => ({
  add: async (...args) => {
    return mockCacheDbAdd(args);
  },
  get: async (...args) => {
    return mockCacheDbGet(args);
  },
}));

export default mock;
