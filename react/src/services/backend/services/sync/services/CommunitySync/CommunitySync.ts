import { combineLatest, distinctUntilChanged, filter, map, Observable, switchMap } from 'rxjs';
import BroadcastChannelSender from 'src/services/backend/channels/BroadcastChannelSender';
import { CommunityDto } from 'src/services/CozoDb/types/dto';
import { fetchStoredSyncCommunity$, SyncCommunityResult } from 'src/services/community/community';
import { ServiceDeps } from '../types';

// eslint-disable-next-line import/no-unused-modules
export default function createCommunitySync$(deps: ServiceDeps): Observable<CommunityDto[]> {
  const { dbInstance$, ipfsInstance$, params$ } = deps;
  const channel = new BroadcastChannelSender();

  return combineLatest([
    dbInstance$,
    params$!.pipe(
      map((params) => params.myAddress),
      distinctUntilChanged()
    ),
    ipfsInstance$,
  ]).pipe(
    filter(
      ([dbInstance, myAddress, ipfsInstance]) => !!dbInstance && !!ipfsInstance && !!myAddress
    ),
    switchMap(([dbApi, myAddress, _ipfsInstance]) => {
      const { waitForParticleResolve } = deps;
      let community: CommunityDto[] = []; // Fix: Add type declaration for community array
      return new Observable<CommunityDto[]>((observer) => {
        observer.next([]);

        fetchStoredSyncCommunity$(dbApi!, myAddress!, waitForParticleResolve!).subscribe(
          ({ action, items }: SyncCommunityResult) => {
            channel.post({ type: 'load_community', value: { action, items } });

            if (action === 'reset') {
              community = [];
            } else if (['add', 'complete'].some((s) => s === action)) {
              community.push(...items);
            }

            if (action === 'complete') {
              observer.next(community);
              observer.complete();
            }
          }
        );
      });
    })
  );
}
