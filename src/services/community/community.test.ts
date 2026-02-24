import DbApiWrapper from '../backend/services/DbApi/DbApi';
import { featchStoredSyncCommunity } from './community';
import { getFollowers, getFollowsAsCid } from './lcd';

jest.mock('../backend/services/dataSource/indexedDb/dbApiWrapper');
jest.mock('../backend/services/lcd/lcd');

const neuronMap = {
  neuron1: 'cid1',
  neuron2: 'cid2',
  neuron3: 'cid3',
};
const reversedNeuronMap = Object.fromEntries(
  Object.entries(neuronMap).map(([key, value]) => [value, key])
);

jest.mock('src/utils/search/utils', () => ({
  getIpfsHash: jest.fn().mockImplementation(async (addr) => neuronMap[addr]),
}));

// Creating mock implementations
const mockGetFollowsAsCid = getFollowsAsCid as jest.Mock;
const mockGetFollowers = getFollowers as jest.Mock;
const mockDbApi = new DbApiWrapper() as jest.Mocked<DbApiWrapper>;

describe('getCommunity function', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should update stored community with new followers and following neurons', async () => {
    const storedCommunity = [
      { particle: 'cid1', neuron: 'neuron1', follower: true }, // My follower
      { particle: 'cid2', neuron: 'neuron2', following: true }, // I follow
    ];

    const newFollowsCids = ['cid3', 'cid1']; // I will follow
    const newFollowers = ['neuron2']; // They follows me

    // Mocking the return values of the functions
    mockDbApi.getCommunity.mockResolvedValue(storedCommunity);
    mockGetFollowsAsCid.mockResolvedValue(newFollowsCids);
    mockGetFollowers.mockResolvedValue(newFollowers);

    // Mock fetchParticleAsync function
    const fetchParticleAsync = jest.fn().mockImplementation(async (cid) => ({
      result: { textPreview: reversedNeuronMap[cid] },
    }));

    // Call the function under test
    await featchStoredSyncCommunity(mockDbApi, 'ownerId', fetchParticleAsync);

    const updatedCommunity = mockDbApi.putCommunity.mock.calls[0][0];

    console.log('updatedCommunity:', updatedCommunity, storedCommunity);
    expect(updatedCommunity).toEqual(
      expect.arrayContaining([
        expect.objectContaining({
          particle: 'cid3',
          following: true,
          neuron: 'neuron3',
        }),
        expect.objectContaining({
          particle: 'cid2',
          follower: true,
          following: true,
        }),
        expect.objectContaining({
          particle: 'cid1',
          follower: true,
          following: true,
        }),
      ])
    );
  });
});
