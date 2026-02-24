/* eslint-disable  */
require('./loader.css');

const Bootloader = () => {
  this.progressMap = {};
  this.tagMap = {};
  this.assets = {};
  this.totalSize = 0;
  this.networkSpeed = 0;

  this.attachAssets = (assets) => {
    this.assets = assets;
    this.totalSize = this.getTotalSize(assets);
    return this;
  };
  this.getTotalSize = (assets) => {
    const cssAssets = assets.css || [];
    const jsAssets = assets.js || [];
    const sum = (acc, asset) => {
      return acc + (asset.size || 0);
    };
    return cssAssets.reduce(sum, 0) + jsAssets.reduce(sum, 0);
    return this;
  };

  this.updateProgress = (src, size) => {
    if (size >= 0) {
      const currentValue = this.progressMap[src] || 0;
      if (size > currentValue) {
        this.progressMap[src] = size;
        return true;
      }
    }
    return false;
  };
  this.getProgress = () => {
    const _this = this;
    return Object.keys(this.progressMap).reduce((acc, src) => {
      return acc + _this.progressMap[src];
    }, 0);
  };
  this.createScriptTag = (src, id) => {
    const tag = document.createElement('script');
    tag.id = id;
    tag.type = 'text/javascript';
    tag.src = src;
    return tag;
  };
  this.createCssTag = (href, id) => {
    const tag = document.createElement('link');
    tag.id = id;
    tag.rel = 'stylesheet';
    tag.href = href;
    return tag;
  };
  this.getBlob = (asset, cb) => {
    const _this = this;
    return new Promise((resolve, reject) => {
      let endTime;
      let fileSize;

      const startTime = Date.now();

      const xhr = new XMLHttpRequest();
      xhr.onreadystatechange = () => {
        if (xhr.readyState === 4 && xhr.status === 200) {
          endTime = Date.now();
          fileSize = xhr.response.size;
          _this.networkSpeed = fileSize / ((endTime - startTime) / 1000) / 1024;
          // Use (fileSize * 8) instead of fileSize for kBps instead of kbps
          // Report the result, or have fries with it...
          // console.log(speed + " kbps\n");
        }
      };

      xhr.open('GET', asset.file, true);
      xhr.responseType = 'blob';
      // set listeners
      xhr.addEventListener(
        'error',
        (err) => {
          return reject(err);
        },
        false
      );
      xhr.addEventListener(
        'progress',
        (event) => {
          if (event.lengthComputable) {
            _this.updateProgress(asset.file, event.loaded) &&
              cb({
                totalSize: _this.totalSize,
                networkSpeed: _this.networkSpeed,
                loaded: _this.getProgress(),
              });
          }
        },
        false
      );
      xhr.addEventListener(
        'load',
        (_event) => {
          const { status } = xhr;
          if (status === 200 || (status === 0 && xhr.response)) {
            _this.updateProgress(asset.file, asset.size) &&
              cb({
                totalSize: _this.totalSize,
                networkSpeed: _this.networkSpeed,
                loaded: _this.getProgress(),
              });
            resolve(xhr.response);
          } else {
            reject('status: '.concat(xhr.status, ' - ').concat(xhr.statusText));
          }
        },
        false
      );
      xhr.send();
    });
  };
  this.loadAsset = (asset, js, cb) => {
    const _this = this;
    return this.getBlob(asset, cb).then((blob) => {
      let _a;
      const assetId = 'asset_'.concat(asset.file);
      // remove asset if it exists
      const oldAsset = document.getElementById(assetId);
      oldAsset &&
        ((_a = document.head) === null || _a === void 0 ? void 0 : _a.removeChild(oldAsset));
      // create new asset
      const objectURL = URL.createObjectURL(blob);
      const tag = js
        ? _this.createScriptTag(objectURL, assetId)
        : _this.createCssTag(objectURL, assetId);
      tag.onload = tag.onerror = () => {
        // remove listeners
        tag.onload = tag.onerror = null;
        // note: if you want the file to be accessible after loading
        // then comment out bellow line
        URL.revokeObjectURL(objectURL);
      };
      _this.tagMap[asset.file] = tag;
      return asset;
    });
  };
  this.loadAssets = (assets, js, cb) => {
    const _this = this;
    const report = {
      succeeded: [],
      failed: [],
    };
    const tasks = assets.map((asset) => {
      return _this
        .loadAsset(asset, js, cb)
        .then((done) => {
          report.succeeded.push(done);
        })
        .catch((error) => {
          report.failed.push({ asset, error });
        });
    });
    return Promise.all(tasks).then(() => {
      return report;
    });
  };
  this.mergeReport = (lr1, lr2) => {
    lr2.succeeded.forEach((i) => {
      return lr1.succeeded.push(i);
    });
    lr2.failed.forEach((i) => {
      return lr1.failed.push(i);
    });
  };
  this.appendHtmlElements = (assets) => {
    let _a;
    for (let i = 0, assets_1 = assets; i < assets_1.length; i++) {
      const asset = assets_1[i];
      const tag = this.tagMap[asset.file];
      if (tag) {
        (_a = document.head) === null || _a === void 0 ? void 0 : _a.appendChild(tag);
      }
    }
  };
  this.load = (cb) => {
    const _this = this;
    const cssAssets = this.assets.css || [];
    const jsAssets = this.assets.js || [];
    const fullReport = {
      succeeded: [],
      failed: [],
    };
    return this.loadAssets(cssAssets, false, cb)
      .then((report) => {
        _this.mergeReport(fullReport, report);
        _this.appendHtmlElements(cssAssets);
        return _this.loadAssets(jsAssets, true, cb);
      })
      .then((report) => {
        _this.mergeReport(fullReport, report);
        _this.appendHtmlElements(jsAssets);
        return fullReport;
      });
  };
  return this;
};

function bootstrap() {
  if ('serviceWorker' in navigator && !process.env.IS_TAURI) {
    console.log('Going to install service worker');

    window.addEventListener('load', () => {
      console.log('Starting to load service worker');
      navigator.serviceWorker
        .register('/service-worker.js')
        .then((registration) => {
          console.log('service worker registered: ', registration);
        })
        .catch((registrationError) => {
          console.log(
            'service worker registration failed: ',
            registrationError
          );
        });
    });
  } else {
    console.log('No service worker is available');
  }

  let _a;
  const assets =
    ((_a = window === null || window === void 0 ? void 0 : window.$bootloader) === null ||
    _a === void 0
      ? void 0
      : _a.assets) || {};
  Bootloader()
    .attachAssets(assets)
    .load((e) => {
      const progress = e.loaded / e.totalSize;
      const progressBar = document.getElementById('progressbar');
      const progressData = document.getElementById('progress-data');

      progressBar === null || progressBar === void 0
        ? void 0
        : progressBar.setAttribute('value', progress.toString());

      progressData.innerHTML = `Loading: <span>${Math.round(
        progress * 100
      )}%</span>. <br/> Network speed: <span>${Math.round(e.networkSpeed * 100) / 100} kbps</span>`;

      // console.log(e.loaded, e.loaded / e.totalSize); // @TODO
    })
    .then((_report) => {
      if (window !== null) {
        let pageloader = null;
        pageloader = setInterval(() => {
          if (typeof window.onload === 'function') {
            clearInterval(pageloader);
            window.onload();
          }
        }, 300);
      } else {
        console.error('No window');
      }

      // selfdestroy @TODO
    });
}
exports.bootstrap = bootstrap;
document.addEventListener('DOMContentLoaded', bootstrap);
