import { extractTxData } from './mapping';

describe('extract memo from transaction Tx', () => {
  it('should decode the transaction data', () => {
    const data =
      'Cr4BCosBChwvY29zbW9zLmJhbmsudjFiZXRhMS5Nc2dTZW5kEmsKLmJvc3Ryb20xdWo4NWw5dWFyODBzMzQybnc1dXFqcm52bTN6bHpzZDAzOTJkcTMSLmJvc3Ryb20xcW44c3IyaHpta3RsZWN1c2R0eGo5aHdqMHVwbm0wamZ0OXNuYXIaCQoEYm9vdBIBMRIuUW1UemFzZzJxb2Fyb2JNaFI1WFFpSDNhQ0xoM1pmMWpUNDY4Q1B4ZUxpYWZicRJZClEKRgofL2Nvc21vcy5jcnlwdG8uc2VjcDI1NmsxLlB1YktleRIjCiED626Wdc0/oaCfIDqBmel56lmm/9RIlQ483B8CKQxeZ78SBAoCCAEYmAMSBBDb8QgaQHLhOB4XBMJxdhArSmWZ0Jw4HIECjkg/rV/CVsP/pXDkEXz17AjRAXN8cl2RKvFy5Xfy9eDM7/CzuF7M1ZMXy2U='; // Base64 encoded string
    const { memo, messages } = extractTxData(data);
    expect(memo).toEqual('QmTzasg2qoarobMhR5XQiH3aCLh3Zf1jT468CPxeLiafbq');
    expect(messages?.length).toEqual(1);
    console.log(messages);
    expect(messages[0].amount).toEqual([{ denom: 'boot', amount: '1' }]);
  });
});
