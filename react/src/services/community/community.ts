import { Observable } from 'rxjs';
import { PATTERN_CYBER } from 'src/constants/patterns';
import { NeuronAddress, ParticleCid } from 'src/types/base';
import { getIpfsHash } from 'src/utils/ipfs/helpers';
import { createCyblogChannel } from 'src/utils/logging/cyblog';
import DbApiWrapper from '../backend/services/DbApi/DbApi';
import { FetchIpfsFunc } from '../backend/services/sync/types';
import { CommunityDto } from '../CozoDb/types/dto';
import { FetchParticleAsync, QueuePriority } from '../QueueManager/types';
import { getFollowers, getFollowsAsCid } from './lcd';

export type SyncCommunityResult = {
  action: 'reset' | 'add' | 'complete';
  items: CommunityDto[];
};

const cyblogCh = createCyblogChannel({
  thread: 'bckd',
  unit: 'fetchStoredSyncCommunity',
});

// eslint-disable-next-line import/prefer-default-export, import/no-unused-modules
export const fetchStoredSyncCommunity$ = (
  dbApi: DbApiWrapper,
  address: NeuronAddress,
  fetchParticleAsync?: FetchIpfsFunc,
  signal?: AbortSignal
): Observable<SyncCommunityResult> => {
  return new Observable<SyncCommunityResult>((subscriber) => {
    subscriber.next({ action: 'reset', items: [] });

    (async () => {
      const storedCommunity = await dbApi.getCommunity(address);

      subscriber.next({ action: 'add', items: storedCommunity });

      const communityUpdatesMap = new Map<ParticleCid, CommunityDto>(
        storedCommunity.map((c) => [c.particle, c])
      );

      const getExistingOrDefault = (cid: ParticleCid): Partial<CommunityDto> =>
        communityUpdatesMap.get(cid) || {
          ownerId: address,
          name: '',
          following: false,
          follower: false,
        };

      const followsCids = await getFollowsAsCid(address, signal);
      const followers = await getFollowers(address, signal);

      const newFollowerCids = followsCids.filter(
        (cid) => !storedCommunity.some((i) => i.particle === cid && i.following)
      );

      const newFollowingNeurons = followers.filter(
        (addr) => !storedCommunity.some((i) => i.neuron === addr && i.follower)
      );

      cyblogCh.info(
        `>>>$ sync community ${address} processing, stored ${storedCommunity.length} new followers: ${newFollowerCids.length} new following: ${newFollowingNeurons.length}`
      );

      const followersCommunity = await Promise.all(
        newFollowingNeurons.map(async (neuron) => {
          const cid = await getIpfsHash(neuron);

          const communityItem = {
            ...getExistingOrDefault(cid),
            particle: cid,
            neuron,
            follower: true,
          } as CommunityDto;

          await dbApi.putCommunity(communityItem);
          communityUpdatesMap.set(cid, communityItem);
          return communityItem;
        })
      );

      subscriber.next({ action: 'add', items: followersCommunity });

      await Promise.all(
        newFollowerCids.map(async (cid: ParticleCid) => {
          const neuron = (await fetchParticleAsync!(cid, QueuePriority.URGENT))?.result
            ?.textPreview;
          if (neuron?.match(PATTERN_CYBER)) {
            const communityItem = {
              ...getExistingOrDefault(cid),
              neuron,
              particle: cid,
              following: true,
            } as CommunityDto;

            await dbApi.putCommunity(communityItem);
            communityUpdatesMap.set(cid, communityItem);
            subscriber.next({ action: 'add', items: [communityItem] });
          }
        })
      );

      cyblogCh.info(`>>>$ sync community ${address}, done`);
      // const communityUpdates = [...communityUpdatesMap.values()];

      // if (communityUpdates.length > 0) {
      //   subscriber.next(communityUpdates);
      // }
      subscriber.next({ action: 'complete', items: [] });

      subscriber.complete();
    })().catch((err) => {
      cyblogCh.error(`>>>$ sync community ${address}, error`, { error: err });
      subscriber.error(err);
    });
  });
};

// eslint-disable-next-line import/no-unused-modules
export const fetchCommunity = async (
  address: NeuronAddress,
  fetchParticleAsync?: FetchParticleAsync,
  onResolve?: (community: CommunityDto[]) => void,
  signal?: AbortSignal
) => {
  const communityUpdatesMap = new Map<ParticleCid, CommunityDto>();

  const getExistingOrDefault = (cid: ParticleCid): Partial<CommunityDto> =>
    communityUpdatesMap.get(cid) || {
      ownerId: address,
      name: '',
      following: false,
      follower: false,
    };

  const followsCids = await getFollowsAsCid(address, signal);
  const followers = await getFollowers(address, signal);

  console.log(`>>> sync community ${address} processing without store`);

  const followsPromise = Promise.all(
    followsCids.map(async (cid) => {
      const neuron = (await fetchParticleAsync!(cid))?.result?.textPreview;
      if (neuron?.match(PATTERN_CYBER)) {
        const communityItem = {
          ...getExistingOrDefault(cid),
          neuron,
          particle: cid,
          following: true,
        } as CommunityDto;
        communityUpdatesMap.set(cid, communityItem);
        onResolve && !signal?.aborted && onResolve([communityItem]);
      }
    })
  );

  const followersPromise = Promise.all(
    followers.map(async (neuron) => {
      const cid = await getIpfsHash(neuron);

      const communityItem = {
        ...getExistingOrDefault(cid),
        particle: cid,
        neuron,
        follower: true,
      } as CommunityDto;

      communityUpdatesMap.set(cid, communityItem);
      onResolve && !signal?.aborted && onResolve([communityItem]);
    })
  );

  await Promise.all([followersPromise, followsPromise]);
};
