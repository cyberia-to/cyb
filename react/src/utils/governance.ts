import axios from 'axios';
import { OrderBy } from 'cosmjs-types/cosmos/tx/v1beta1/service';
import { LCD_URL } from 'src/constants/config';
import { getTransactions } from 'src/services/transactions/lcd';

export const getProposals = async () => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/cosmos/gov/v1/proposals`,
    });

    return response.data.proposals;
  } catch (error) {
    console.log('getProposals error', error);
    return [];
  }
};

export const getProposalsDetail = async (id) => {
  return axios({
    method: 'get',
    url: `${LCD_URL}/cosmos/gov/v1/proposals/${id}`,
  })
    .then((response) => response.data.proposal)
    .catch((e) => {
      console.error('getProposalsDetail: ', e);
      throw new Error(`Not found: ${id}`);
    });
};

export const getStakingPool = () =>
  new Promise((resolve) => {
    axios({
      method: 'get',
      url: `${LCD_URL}/staking/pool`,
    })
      .then((response) => {
        resolve(response.data.result);
      })
      .catch((e) => {
        console.error(e);
      });
  });

export const getTallying = () =>
  new Promise((resolve) => {
    axios({
      method: 'get',
      url: `${LCD_URL}/gov/parameters/tallying`,
    })
      .then((response) => {
        resolve(response.data.result);
      })
      .catch((e) => {
        console.error(e);
      });
  });

export const getProposer = async (id) => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/gov/proposals/${id}/proposer`,
    });
    return response.data.result;
  } catch (error) {
    console.log(error);
    return null;
  }
};

export const getTableVoters = async (id, offset = 0, limit = 20) => {
  try {
    const response = await getTransactions({
      events: [
        {
          key: 'proposal_vote.proposal_id',
          value: id,
        },
      ],
      pagination: { limit, offset: offset * limit },
      orderBy: OrderBy.ORDER_BY_DESC,
    });

    return response;
  } catch (_error) {
    return null;
  }
};

export const getTallyingProposals = async (id) => {
  try {
    const response = await axios({
      method: 'get',
      url: `${LCD_URL}/gov/proposals/${id}/tally`,
    });
    return response.data.result;
  } catch (e) {
    console.log(e);
    return null;
  }
};

export const reduceTxsVoters = (txs) => {
  return txs.reduce((prevObj, item) => {
    const { txhash, timestamp } = item;
    const messagesItems = item.tx.body.messages.reduce((mPrevObj, value) => {
      let v = value;

      if (value['@type'] === '/cosmos.authz.v1.MsgExec') {
        v = value.msgs?.[0];
      }

      return { ...mPrevObj, ...v };
    }, {});
    return [...prevObj, { txhash, timestamp, ...messagesItems }];
  }, []);
};
