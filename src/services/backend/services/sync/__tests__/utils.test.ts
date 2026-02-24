import { CID_TWEET } from 'src/constants/app';
import { CYBER_LINK_TRANSACTION_TYPE } from '../../indexer/types';
import { extractCybelinksFromTransaction } from '../services/utils/links';
import { changeParticleSyncStatus, extractLinkData } from '../utils';

jest.mock('../../dataSource/blockchain/requests', () => ({
  fetchCyberlinksIterable: jest.fn(),
}));

test('extractParticlesResults should return correct results', () => {
  // Mock input batch
  const batch = [
    {
      type: CYBER_LINK_TRANSACTION_TYPE,
      value: {
        links: [
          { from: CID_TWEET, to: 'to1' },
          { from: CID_TWEET, to: 'to2' },
        ],
      },
      transaction: {
        block: { timestamp: new Date() },
      },
    },
    {
      type: 'ANOTHER_TRANSACTION_TYPE',
      value: {
        // Add relevant data for the new transaction type
      },
      transaction: {
        block: { timestamp: new Date() },
      },
    },
    {
      type: CYBER_LINK_TRANSACTION_TYPE,
      value: {
        links: [{ from: CID_TWEET, to: 'to3' }],
      },
      transaction: {
        block: { timestamp: new Date() },
      },
    },
    {
      type: CYBER_LINK_TRANSACTION_TYPE,
      value: {
        links: [{ from: 'to4', to: 'to5' }],
      },
      transaction: {
        block: { timestamp: new Date() },
      },
    },
    // Add more mock transactions as needed
  ];

  // Call the function
  const result = extractCybelinksFromTransaction(batch);
  console.log(result);
  // Assert the expected output
  expect(result.tweets).toEqual({
    to1: {
      timestamp: expect.any(Number),
      direction: 'from',
      from: CID_TWEET,
      to: 'to1',
    },
    to2: {
      timestamp: expect.any(Number),
      direction: 'from',
      from: CID_TWEET,
      to: 'to2',
    },
    to3: {
      timestamp: expect.any(Number),
      direction: 'from',
      from: CID_TWEET,
      to: 'to3',
    },
  });
  expect(result.tweets).toEqual(
    expect.objectContaining({
      to1: expect.any(Object),
      to2: expect.any(Object),
      to3: expect.any(Object),
    })
  );
  expect(result.particlesFound).toEqual(
    expect.arrayContaining([CID_TWEET, 'to1', 'to2', 'to3', 'to5', 'to4'])
  );
  expect(result.links.length).toBe(4); // Update the expected links count
});

const mockTransaction = [
  {
    transaction_hash: 'hash1',
    type: CYBER_LINK_TRANSACTION_TYPE,
    value: {
      links: [
        { from: 'from_cid_1', to: 'to_cid_1' },
        { from: 'from_cid_2', to: 'to_cid_2' },
      ],
      neuron: 'neuron1',
    },
    transaction: {
      memo: 'M33M0',
      block: { timestamp: new Date() },
    },
  },
  {
    transaction_hash: 'hash2',
    value: {
      msg: 'msg1==',
      funds: [],
      sender: 'sender-addr',
      contract: 'contact-addr',
    },
    transaction: {
      success: true,
      block: {
        timestamp: '2023-12-18T10:28:13.942406',
        height: 11324342,
      },
      memo: '[bostrom] cyb.ai, using keplr',
    },
    type: 'cosmwasm.wasm.v1.MsgExecuteContract',
  },
  // Add more mock transactions as needed
];

const mockCyberlinks = {
  cyberlinks: [
    { from: 'from_cid_1', to: 'to_cid_1', timestamp: '2022-01-01' },
    { from: 'from_cid_2', to: 'to_cid_2', timestamp: '2022-01-10' },
  ],
  // Add more mock cyberlinks as needed
};

// Test for extractParticlesResults function
test('extractParticlesResults should return the expected result', () => {
  const result = extractCybelinksFromTransaction(mockTransaction);
  expect(result.tweets).toBeDefined();
  expect(result.particlesFound).toBeDefined();
  expect(result.links).toBeDefined();
  // Add more specific assertions as needed
});

// Test for extractLinkData function
test('extractLinkData should return the expected result', () => {
  const result = extractLinkData('from_cid_1', mockCyberlinks.cyberlinks);
  expect(result.direction).toEqual('from');
  expect(result.lastLinkCid).toEqual('to_cid_1');
  expect(result.count).toEqual(2);
  expect(result.lastTimestamp).toEqual(1640995200000);
  expect(result.firstTimestamp).toEqual(1641772800000);
  // Add more specific assertions as needed
});

// Test for updateSyncState function
test('updateSyncState should return the expected result', () => {
  const mockStatusEntity = {
    id: 'from_cid_xx',
    unreadCount: 5,
    timestampRead: 1641753000000,
  };
  const result = changeParticleSyncStatus(mockStatusEntity, mockCyberlinks.cyberlinks);
  console.log(result);

  expect(result.lastId).toEqual('from_cid_1');
  expect(result.unreadCount).toEqual(7);
  expect(result.meta).toBeDefined();
  expect(result.timestampUpdate).toEqual(1640995200000);
  expect(result.timestampRead).toEqual(1641753000000);
  // Add more specific assertions as needed
});
