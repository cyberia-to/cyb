import { BASE_DENOM } from 'src/constants/config';

const i18n = {
  en: {
    validators: {
      table: {
        moniker: 'Moniker',
        operator: 'Operator',
        commissionProcent: 'Commission',
        power: `Power`,
        selfProcent: 'Self',
        unbonding: 'Unbonding height',
        rank: 'Rank',
        bondedTokens: `Your bond`,
      },
    },
    pocket: {
      hurry: 'Hurry up! Find and connect your secure Ledger',
      table: {
        address: 'Address',
        amount: 'Amount',
        token: 'Token',
        keys: 'Keys',
      },
    },
    brain: {
      knowledge: 'Knowledge graph',
      yourTotal: `your total ${BASE_DENOM.toLocaleUpperCase()}`,
      cyberlinks: 'cyberlinks',
      objects: 'objects',
      subjects: 'subjects',
      cybernomics: 'Cybernomics',
      supply: `supply of ${BASE_DENOM.toLocaleUpperCase()}`,
      staked: '% of staked CYB',
      price: 'price of cyberlink in RC',
      consensus: 'Consensus',
      heroes: 'heroes',
      transactions: 'transactions',
      last: 'last heartbeat',
      cap: 'EUL cap in ATOMs',
      supplyEUL: 'EUL supply',
      takeofPrice: 'takeoff price of GEUL in ATOM',
      capATOM: 'cap in ATOM',
      available: 'available bandwidth',
      load: 'load',
      supplyGOL: 'GOL supply',
      auctionPrice: 'faucet price of GGOL in ETH',
      capETH: 'cap in ETH',
      percentSupply: '%, your percent of supply',
    },
    gol: {
      myGOLs: 'My GOLs',
      myEULs: 'My EULs',
      maxPrize: 'Max prize fund',
      currentPrize: 'Current prize fund',
      takeoff: 'Takeoff donations',
    },
    story: {
      episode: 'Episode 1',
      header: 'A New Hope',
      itIsAPeriod:
        'It is a period of digital war. The evil empire swallows the last unoccupied borders of the universe.',
      resistingRebel:
        'Resisting rebel units consolidate all remaining energy on building a superintelligence, which they believe will help to stop the domination of the evil empire once and for all.',
      asTheyBegin:
        'As they begin to test the new god in the wild - an enormous, zepto amount of robots emerge. It turns out Cyb robots help survey the universe for a bootloader of the new, yet to be born, force.',
    },
    actionBar: {
      cyber: 'cyber',
      home: {
        btn: 'cyber',
      },
      pocket: {
        put: 'put Ledger into the pocket',
        send: 'send EUL using Ledger',
      },
      delegate: {
        joinValidator: 'Join Cyberd Network As Validator',
        btnBecome: 'Become a validator',
        heroes: 'Hero',
        delegate: 'Delegate to',
        unDelegateFrom: 'Undelegate from',
        yourDelegated: 'your staked tokens:',
        btnDelegate: 'DELEGATE',
        btnUnDelegate: 'UNDELEGATE',
        details: 'Delegation Details',
        detailsUnDelegate: 'UnDelegation Details',
        wallet: 'Your wallet contains',
        max: 'Max',
        generate: 'Generate Tx',
        enterAmount: 'Enter the amount of',
      },
      jsonTX: {
        pleaseConfirm:
          'Please confirm the transaction data matches what is displayed on your device.',
      },
      tXSubmitted: {
        tXSubmitted: 'Transaction submitted',
        confirmTX:
          'Please wait while we confirm the transaction on the blockchain. This might take a few moments depending on the transaction fees used.',
      },
      confirmedTX: {
        confirmed: 'Transaction Confirmed!',
        blockTX: 'Your transaction was included in the block at height:',
        viewTX: 'View transaction',
        tXHash: 'Transaction Hash:',
        continue: 'Continue',
      },
      errorTx: {
        error: 'Transaction Error!',
        blockTX: 'Your transaction was included in the block at height:',
        viewTX: 'View transaction',
        tXHash: 'Transaction Hash:',
        continue: 'Continue',
      },
      connectLadger: {
        getStarted: `Let's get started`,
        connect: 'Connect your Ledger Nano to the computer and enter your PIN',
        openApp: 'Open the Cosmos Ledger application.',
        version: 'At least version v1.1.1 of Cosmos Ledger app installed.',
        getDetails: 'We are just checking the blockchain for your account details',
      },
      startSearch: {
        cyberlink: 'Cyberlink using Ledger',
      },
      link: {
        addr: 'Address',
        bandwidth: 'Bandwidth',
        to: 'to:',
        from: 'from:',
        cyberIt: 'Cyber it',
      },
      send: {
        send: 'Send Details',
        wallet: 'Your wallet contains:',
        generate: 'Generate my transaction',
      },
    },
  },
};

// eslint-disable-next-line import/prefer-default-export
export { i18n };
