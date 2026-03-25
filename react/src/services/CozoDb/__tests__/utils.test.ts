import { dtoToEntity, entityToDto } from 'src/utils/dto';

describe('DTO<->DB Entity', () => {
  it('transforms DTO to DB entity and vice versa', () => {
    const dtoEntity = {
      hash: 'txhash',
      index: 0,
      type: 'cosmos.bank.v1beta1.MsgMultiSend',
      timestamp: 1700115771831,
      memo: 'txmemo',
      value: {
        inputs: [
          {
            coins: [{ denom: 'hydrogen', amount: '11646000000000' }],
            address: 'adress1',
          },
        ],
        outputs: [
          {
            coins: [{ denom: 'hydrogen', amount: '1092000000000' }],
            address: 'adress2',
          },
          {
            coins: [{ denom: 'hydrogen', amount: '972000000000' }],
            address: 'adress3',
          },
        ],
      },
      success: true,
      neuron: 'neuronXYZ',
      blockHeight: 10863958,
    };

    const dbEntity = {
      hash: 'txhash',
      index: 0,
      type: 'cosmos.bank.v1beta1.MsgMultiSend',
      timestamp: 1700115771831,
      memo: 'txmemo',
      value: {
        inputs: [
          {
            coins: [{ denom: 'hydrogen', amount: '11646000000000' }],
            address: 'adress1',
          },
        ],
        outputs: [
          {
            coins: [{ denom: 'hydrogen', amount: '1092000000000' }],
            address: 'adress2',
          },
          {
            coins: [{ denom: 'hydrogen', amount: '972000000000' }],
            address: 'adress3',
          },
        ],
      },
      success: true,
      neuron: 'neuronXYZ',
      block_height: 10863958,
    };

    const entityResult = dtoToEntity(dtoEntity);
    expect(entityResult).toEqual(dbEntity);
    const dtoResult = entityToDto(dbEntity);
    expect(dtoResult).toEqual(dtoEntity);
  });
});
