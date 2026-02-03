// eslint-disable-next-line import/prefer-default-export
export const routes = {
  home: {
    path: '/',
  },
  temple: {
    path: '/temple',
  },
  senate: {
    path: '/senate',
    routes: {
      new: {
        path: '/senate/new',
      },
      proposal: {
        path: '/senate/:proposalId/*',
        getLink: (proposalId: number) => `/senate/${proposalId}`,
      },
    },
  },
  sphere: {
    path: '/sphere',
  },
  sphereJailed: {
    path: '/sphere/jailed',
  },
  hfr: {
    path: '/hfr',
  },
  gift: {
    path: '/gift',
  },
  buy: {
    path: '/buy',
  },
  ipfs: {
    path: '/ipfs',
    getLink: (param: string) => `/ipfs/${param}`,
  },
  citizenship: {
    path: '/citizenship',
  },
  robot: {
    path: '/robot',
    routes: {
      // TODO: reuse parent
      karma: {
        path: '/robot/karma',
      },
      drive: {
        path: '/robot/drive',
      },
      sense: {
        path: '/robot/sense',
      },
      energy: {
        path: '/robot/energy',
      },
      brain: {
        path: '/robot/brain',
      },
      soul: {
        path: '/robot/soul',
      },
      sigma: {
        path: '/robot/sigma',
      },
    },
  },
  robotPassport: {
    path: '/:username',
    getLink: (username: string) => `/@${username}`,
  },
  portal: {
    path: '/portal',
    routes: {
      map: {
        path: '/portal/map',
      },
    },
  },
  search: {
    path: '/search',
    getLink: (search: string) => `/oracle/ask/${search}`,
  },
  hero: {
    path: '/sphere/hero/:address',
    getLink: (address: string) => `/sphere/hero/${address}`,
    getLinkToTab: (address: string, tab: string) =>
      `/sphere/hero/${address}/${tab}`,
  },
  teleport: {
    path: '/teleport',
    send: {
      path: '/teleport/send',
    },
    bridge: {
      path: '/teleport/bridge',
    },
    swap: {
      path: '/teleport/swap',
    },
  },
  keys: {
    path: '/settings/keys',
  },
  sigma: {
    path: '/sigma',
  },
  neuron: {
    path: '/neuron/:address/*',
    getLink: (address: string) => `/neuron/${address}`,
  },
  oracle: {
    path: '/',
    learn: {
      path: '/oracle/learn',
    },
    ask: {
      path: '/oracle/ask/:query',
      getLink: (query: string) => `/oracle/ask/${query}`,
    },
    routes: {
      stats: {
        path: '/oracle/stats',
      },
      blocks: {
        path: '/oracle/blocks',
      },
      txs: {
        path: '/oracle/txs',
      },
      particles: {
        path: '/oracle/particles',
      },
    },
  },
  nebula: {
    path: '/nebula',
  },
  warp: {
    path: '/warp',
  },
  social: {
    path: '/social',
  },
  brain: {
    path: '/brain',
  },
  txExplorer: {
    path: '/tx/:hash',
    getLink: (hash: string) => `/network/bostrom/tx/${hash}`,
  },
  contracts: {
    path: '/contracts',
    byAddress: {
      path: '/contracts/:contractAddress',
      getLink: (contractAddress: string) => `/contracts/${contractAddress}`,
    },
  },
  blocks: {
    path: '/blocks',
    getLink: () => `/network/bostrom/blocks`,
    idBlock: {
      path: '/blocks/:idBlock',
      getLink: (idBlock: string) => `/network/bostrom/blocks/${idBlock}`,
    },
  },
  settings: {
    path: '/settings',
  },
  studio: {
    path: '/studio',
    getLink: (hash: string) => `/studio/${hash}`,
  },
};
