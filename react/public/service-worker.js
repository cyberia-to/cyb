// Self-destructing service worker: unregisters itself and clears caches
// Deployed to replace the old Workbox SW that was caching stale index.html
self.addEventListener('install', function() {
  self.skipWaiting();
});

self.addEventListener('activate', function(event) {
  event.waitUntil(
    caches.keys().then(function(names) {
      return Promise.all(names.map(function(name) { return caches.delete(name); }));
    }).then(function() {
      return self.registration.unregister();
    })
  );
});
