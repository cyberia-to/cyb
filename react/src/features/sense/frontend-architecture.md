


<img width="544" alt="image" src="https://github.com/cybercongress/cyb-ts/assets/18665326/cbce6213-8f88-45ab-a054-5c705ae8f9f0">


frontend formats backend data to items similar to blockchain `Tx.message type`
<br> `value` is additional information of item

```
// store primary type of item
// similar to Tx.message
type SenseItem = {
  id: SenseItemId;
  transactionHash: string;
  timestamp: string;
  memo: string;
  from: string;
  type: string; // ex. cosmos.bank.v1beta1.MsgSend

  // additional information of tx, link
  value: {};

  // for optimistic update
  status?: 'pending' | 'error';
};
```

<img width="475" alt="image" src="https://github.com/cybercongress/cyb-ts/assets/18665326/54e51c93-50db-4d4f-8f49-8dae9dd26dbf">

---

TODO: 
- optimistic update
- ui format
- list

