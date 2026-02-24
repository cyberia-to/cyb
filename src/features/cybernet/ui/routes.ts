const root = '/cyberver';

export const routes = {
  verses: {
    path: `${root}/verses`,
    getLink: () => `${root}/verses`,
  },
  verseNetwork: {
    path: `${root}/verses/:network`,
    getLink: (network: string) => `${root}/verse/${network}`,
  },
  verse: {
    path: `${root}/verses/:network/:nameOrAddress`,
    getLink: (network: string, nameOrAddress: string) =>
      `${root}/verses/${network}/${nameOrAddress}`,
  },
  subnets: {
    path: `${root}/verses/:network/:nameOrAddress/faculties`,
    getLink: (network: string, nameOrAddress: string) =>
      `${root}/verses/${network}/${nameOrAddress}/faculties`,
  },
  subnet: {
    path: `${root}/verses/:network/:nameOrAddress/faculties/:nameOrUid`,
    getLink: (network: string, nameOrAddress: string, nameOrUid: string | number) =>
      `${root}/verses/${network}/${nameOrAddress}/faculties/${nameOrUid}`,
  },
  delegators: {
    path: `${root}/verses/:network/:nameOrAddress/mentors`,
    getLink: (network: string, nameOrAddress: string) =>
      `${root}/verses/${network}/${nameOrAddress}/mentors`,
  },
  delegator: {
    path: `${root}/verses/:network/:nameOrAddress/faculties/:nameOrUid/mentors/:address`,
    getLink: (network: string, nameOrAddress: string, address: string) =>
      `${root}/verses/${network}/${nameOrAddress}/mentors/${address}`,
  },
  myMentor: {
    path: `${root}/verses/:network/:nameOrAddress/mentors/my`,
    getLink: (network: string, nameOrAddress: string) =>
      `${root}/verses/${network}/${nameOrAddress}/mentors/my`,
  },
  myLearner: {
    path: `${root}/verses/:network/:nameOrAddress/learners/my`,
    getLink: (network: string, nameOrAddress: string) =>
      `${root}/verses/${network}/${nameOrAddress}/learners/my`,
  },
  sigma: {
    path: `${root}/sigma`,
    getLink: () => `${root}/sigma`,
  },
};

routes.delegate = routes.delegator;

export const cybernetRoutes = routes;
