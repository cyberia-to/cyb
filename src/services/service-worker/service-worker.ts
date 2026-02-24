/* eslint-disable import/no-unused-modules */
import { clientsClaim, setCacheNameDetails } from 'workbox-core';
import { ExpirationPlugin } from 'workbox-expiration';
import { matchPrecache, precacheAndRoute } from 'workbox-precaching';
import { registerRoute } from 'workbox-routing';
import { CacheFirst, NetworkFirst } from 'workbox-strategies';
import { CustomHeaders, XCybSourceValues } from '../QueueManager/constants';

declare const self: ServiceWorkerGlobalScope;

const prefix = 'cyb.ai';
const suffix = 'v1';
const precache = 'app-precache';

setCacheNameDetails({
  prefix,
  suffix,
  precache,
});

self.skipWaiting();
clientsClaim();
precacheAndRoute(self.__WB_MANIFEST);

registerRoute(
  ({ request }) =>
    request.destination === 'image' ||
    request.destination === 'style' ||
    request.destination === 'audio' ||
    request.destination === 'video' ||
    request.destination === 'font',
  new CacheFirst({
    cacheName: 'assets',
    plugins: [
      new ExpirationPlugin({
        maxAgeSeconds: 24 * 60 * 60,
      }),
    ],
  })
);

registerRoute(
  ({ request }) => !navigator.onLine && request.mode === 'navigate',
  async ({ event }) => {
    const request = (event as any).request as Request;
    console.log('[Service worker] fecth document', request);

    const cachedDocument = await matchPrecache('/index.html');

    if (cachedDocument) {
      return cachedDocument;
    }

    return new Response(
      `<body style="color: white; background-color: black; display: flex; justify-content: center; width: 100wv; height: 100hv; align-items: center">
        <div style="display: flex; flex-direction: column;">
          <div>We're sorry</div>
          <div>Cyb.ai is down</div>
        </div>
      </body>`,
      {
        headers: { 'Content-Type': 'text/html' },
      }
    );
  }
);

registerRoute(
  ({ request }) =>
    request.method === 'GET' &&
    request.destination !== 'document' &&
    request.headers.get(CustomHeaders.XCybSource) !== XCybSourceValues.sharedWorker &&
    !(
      request.destination === 'image' ||
      request.destination === 'style' ||
      request.destination === 'audio' ||
      request.destination === 'video' ||
      request.destination === 'font'
    ),
  new NetworkFirst({ cacheName: 'api-responses' })
);

function generateCacheKey(request: Request) {
  return request.url + JSON.stringify(request.body);
}
registerRoute(
  ({ request }) => request.method === 'POST',
  async ({ event }) => {
    const request = (event as any).request as Request;
    const cacheKey = generateCacheKey(request);
    const cachedResponse = await caches.match(cacheKey);

    if (cachedResponse) {
      return cachedResponse;
    }

    const response = await fetch(request);

    const cache = await caches.open('api-responses-post');
    await cache.put(cacheKey, response.clone());

    return response;
  }
);
