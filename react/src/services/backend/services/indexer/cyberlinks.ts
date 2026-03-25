/* eslint-disable import/no-unused-modules */

import {
  CyberlinksByParticleDocument,
  CyberlinksByParticleQuery,
  CyberlinksByParticleQueryVariables,
  CyberlinksCountByNeuronDocument,
  CyberlinksCountByNeuronQuery,
  CyberlinksCountByNeuronQueryVariables,
  Order_By,
} from 'src/generated/graphql';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { fetchIterableByOffset } from 'src/utils/async/iterable';
import { numberToUtcDate } from 'src/utils/date';
import { CYBERLINKS_BATCH_LIMIT } from './consts';
import { createIndexerClient } from './utils/graphqlClient';

const fetchCyberlinks = async ({
  particleCid,
  timestampFrom,
  offset = 0,
  abortSignal,
}: {
  particleCid: ParticleCid;
  timestampFrom: number;
  offset?: number;
  abortSignal: AbortSignal;
}) => {
  try {
    const res = await createIndexerClient(abortSignal).request<
      CyberlinksByParticleQuery,
      CyberlinksByParticleQueryVariables
    >(CyberlinksByParticleDocument, {
      limit: CYBERLINKS_BATCH_LIMIT,
      offset,
      orderBy: [{ timestamp: Order_By.Asc }],
      where: {
        _or: [{ particle_to: { _eq: particleCid } }, { particle_from: { _eq: particleCid } }],
        timestamp: { _gt: numberToUtcDate(timestampFrom) },
      },
    });

    return res.cyberlinks;
  } catch (e) {
    console.error('fetchCyberlinks failed:', e);
    return [];
  }
};

const fetchCyberlinksCount = async (
  address: NeuronAddress,
  particlesFrom: ParticleCid[],
  timestampFrom: number,
  abortSignal: AbortSignal
) => {
  try {
    const res = await createIndexerClient(abortSignal).request<
      CyberlinksCountByNeuronQuery,
      CyberlinksCountByNeuronQueryVariables
    >(CyberlinksCountByNeuronDocument, {
      address,
      particles_from: particlesFrom,
      timestamp: numberToUtcDate(timestampFrom),
    });

    return res.cyberlinks_aggregate.aggregate?.count;
  } catch (e) {
    console.error('fetchCyberlinksCount failed:', e);
    return 0;
  }
};

const fetchCyberlinksByNeroun = async ({
  neuron,
  particlesFrom,
  timestampFrom,
  batchSize,
  offset = 0,
  abortSignal,
}: {
  neuron: NeuronAddress;
  particlesFrom: ParticleCid[];
  timestampFrom: number;
  batchSize: number;
  offset: number;
  abortSignal: AbortSignal;
}) => {
  const where = {
    _and: [
      {
        timestamp: {
          _gt: numberToUtcDate(timestampFrom),
        },
      },
      {
        neuron: {
          _eq: neuron,
        },
      },
      { particle_from: { _in: particlesFrom } },
    ],
  };

  try {
    const res = await createIndexerClient(abortSignal).request<
      CyberlinksByParticleQuery,
      CyberlinksByParticleQueryVariables
    >(CyberlinksByParticleDocument, {
      limit: batchSize,
      offset,
      orderBy: [
        {
          timestamp: Order_By.Asc,
        },
      ],
      where,
    });

    return res.cyberlinks;
  } catch (e) {
    console.error('fetchCyberlinksByNeroun failed:', e);
    return [];
  }
};

export const fetchCyberlinksByNerounIterable = async (
  neuron: NeuronAddress,
  particlesFrom: ParticleCid[],
  timestampFrom: number,
  batchSize: number,
  abortSignal: AbortSignal
) =>
  fetchIterableByOffset(fetchCyberlinksByNeroun, {
    neuron,
    particlesFrom,
    timestampFrom,
    batchSize,
    abortSignal,
  });

const fetchCyberlinksIterable = (
  particleCid: ParticleCid,
  timestampFrom: number,
  abortSignal: AbortSignal
) =>
  fetchIterableByOffset(fetchCyberlinks, {
    particleCid,
    timestampFrom,
    abortSignal,
  });

export { fetchCyberlinksIterable, fetchCyberlinksCount };
