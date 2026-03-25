export const mockGetSyncStatus = jest.fn();
export const mockPutSyncStatus = jest.fn();
export const mockUpdateSyncStatus = jest.fn();
export const mockPutTransactions = jest.fn();
export const mockFindSyncStatus = jest.fn();
export const mockPutPins = jest.fn();
export const mockGetPins = jest.fn();
export const mockDeletePins = jest.fn();
export const mockPutParticles = jest.fn();
export const mockGetParticles = jest.fn();
export const mockGetSenseList = jest.fn();
// export const mockGetSenseSummary = jest.fn();
export const mockSenseMarkAsRead = jest.fn();
export const mockGetTransactions = jest.fn();
export const mockPutCyberlinks = jest.fn();
export const mockPutSyncQueue = jest.fn();
export const mockUpdateSyncQueue = jest.fn();
export const mockRemoveSyncQueue = jest.fn();
export const mockGetSyncQueue = jest.fn();
export const mockGetLinks = jest.fn();

// Inside dbApiWrapperMock.ts, update the 'mock' function to include all methods from DbApiWrapper as follows:

const mock = jest.fn().mockImplementation(() => ({
  putSyncStatus: async (syncStatus) => {
    console.log('---------mockDbApi putSyncStatus!!!!', syncStatus);
    await mockPutSyncStatus(syncStatus);
    return Promise.resolve();
  },
  updateSyncStatus: async (syncStatus) => {
    console.log('---------mockDbApi updateSyncStatus!!!!', syncStatus);
    await mockUpdateSyncStatus(syncStatus);
    return Promise.resolve();
  },
  getSyncQueue: async (...args) => {
    console.log('---------mockDbApi getSyncQueue', args);
    return [];
  },
  // getSyncQueue: async (...args) => {
  //   console.log('---------mockDbApi getSyncQueue', args);
  //   return mockGetSyncQueue(args);
  // },
  findSyncStatus: async (args) => {
    const result = await mockFindSyncStatus(args);
    console.log('---------mockDbApi findSyncStatus', args, result);
    return result;
  },
  getSyncStatus: async (...args) => {
    console.log('---------mockDbApi getSyncStatus', args);
    return mockGetSyncStatus(args);
  },
  putTransactions: async (...args) => {
    console.log('---------mockDbApi putTransactions', args);
    return mockPutTransactions(args);
  },
  putCyberlinks: async (...args) => {
    console.log('---------mockDbApi putCyberlinks', args);
    return mockPutCyberlinks(args);
  },
  putPins: async (args) => {
    console.log('---------mockDbApi putPins', args);
    return mockPutPins(args);
  },
  getPins: async (...args) => {
    console.log('---------mockDbApi getPins', args);
    return mockGetPins(args);
  },
  deletePins: async (args) => {
    console.log('---------mockDbApi deletePins', args);
    return mockDeletePins(args);
  },
  putParticles: async (args) => {
    console.log('---------mockDbApi putParticles', args);
    return mockPutParticles(args);
  },
  getParticles: async (...args) => {
    console.log('---------mockDbApi getParticles', args);
    return mockGetParticles(args);
  },
  putSyncQueue: async (args) => {
    console.log('---------mockDbApi putSyncQueue', args);
    return mockPutSyncQueue(args);
  },
  updateSyncQueue: async (...args) => {
    console.log('---------mockDbApi updateSyncQueue', args);
    return mockUpdateSyncQueue(args);
  },
  removeSyncQueue: async (...args) => {
    console.log('---------mockDbApi removeSyncQueue', args);
    return mockRemoveSyncQueue(args);
  },

  getLinks: async (...args) => {
    console.log('---------mockDbApi getLinks', args);
    return mockGetLinks(args);
  },
  getSenseList: async (...args) => {
    console.log('---------mockDbApi getSenseList', args);
    return mockGetSenseList(args);
  },
  // getSenseSummary: async (...args) => {
  //   console.log('---------mockDbApi getSenseSummary', args);
  //   return mockGetSenseSummary(args);
  // },
  senseMarkAsRead: async (...args) => {
    console.log('---------mockDbApi senseMarkAsRead', args);
    return mockSenseMarkAsRead(args);
  },
  getTransactions: async (...args) => {
    console.log('---------mockDbApi getTransactions', args);
    return mockGetTransactions(args);
  },
}));

export default mock;
