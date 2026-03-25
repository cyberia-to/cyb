// __mocks__/BroadcastChannelSenderMock.ts

export const mockPostServiceStatus = jest.fn();
// export const mockPostSyncStatus = jest.fn();
export const mockPostSyncEntryProgress = jest.fn();
export const mockPost = jest.fn();

const mock = jest.fn().mockImplementation(() => ({
  postServiceStatus: async (...args) => {
    return mockPostServiceStatus(args);
  },
  // postSyncStatus: async (...args) => {
  //   return mockPostSyncStatus(args);
  // },
  postSyncEntryProgress: async (...args) => {
    return mockPostSyncEntryProgress(args);
  },
  post: async (...args) => {
    return mockPost(args);
  },
}));

export default mock;
