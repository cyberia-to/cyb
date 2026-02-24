var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) =>
  key in obj
    ? __defProp(obj, key, {
        enumerable: true,
        configurable: true,
        writable: true,
        value,
      })
    : (obj[key] = value);
var __publicField = (obj, key, value) => {
  __defNormalProp(obj, typeof key !== 'symbol' ? `${key}` : key, value);
  return value;
};
var __accessCheck = (obj, member, msg) => {
  if (!member.has(obj)) throw TypeError(`Cannot ${msg}`);
};
var __privateGet = (obj, member, getter) => {
  __accessCheck(obj, member, 'read from private field');
  return getter ? getter.call(obj) : member.get(obj);
};
var __privateAdd = (obj, member, value) => {
  if (member.has(obj)) throw TypeError('Cannot add the same private member more than once');
  member instanceof WeakSet ? member.add(obj) : member.set(obj, value);
};
var __privateMethod = (obj, member, method) => {
  __accessCheck(obj, member, 'access private method');
  return method;
};
var _empty,
  empty_fn,
  _fire,
  fire_fn,
  _move,
  move_fn,
  _prepLoop,
  prepLoop_fn,
  _fireItemWithContext,
  fireItemWithContext_fn,
  _wait,
  wait_fn,
  _attachCursor,
  attachCursor_fn,
  _elementIsInput,
  elementIsInput_fn,
  _queueAndReturn,
  queueAndReturn_fn,
  _maybeAppendPause,
  maybeAppendPause_fn,
  _generateTemporaryOptionQueueItems,
  generateTemporaryOptionQueueItems_fn,
  _updateOptions,
  updateOptions_fn,
  _generateQueue,
  generateQueue_fn,
  _buildOptions,
  _prependHardcodedStrings,
  prependHardcodedStrings_fn,
  _setUpCursor,
  setUpCursor_fn,
  _addSplitPause,
  addSplitPause_fn,
  _type,
  type_fn,
  _delete,
  delete_fn,
  _removeNode,
  removeNode_fn,
  _getPace,
  getPace_fn,
  _derivedCursorPosition,
  derivedCursorPosition_get,
  _isInput,
  isInput_get,
  _shouldRenderCursor,
  shouldRenderCursor_get,
  _allChars,
  allChars_get,
  _a;

import React, { forwardRef, useEffect, useRef, useState } from 'react';

const isArray = (thing) => Array.isArray(thing);
const asArray = (value) => (isArray(value) ? value : [value]);
const Queue = (initialItems) => {
  const add = function (steps) {
    asArray(steps).forEach((step) => {
      var _a2;
      return _q.set(
        Symbol((_a2 = step.char) == null ? void 0 : _a2.innerText),
        buildQueueItem({ ...step })
      );
    });
    return this;
  };
  const getTypeable = () => rawValues().filter((value) => value.typeable);
  const set = (index, item) => {
    const keys = [..._q.keys()];
    _q.set(keys[index], buildQueueItem(item));
  };
  const buildQueueItem = (queueItem) => {
    queueItem.shouldPauseCursor = function () {
      return Boolean(this.typeable || this.cursorable || this.deletable);
    };
    return queueItem;
  };
  const reset = () => {
    _q.forEach((item) => delete item.done);
  };
  const wipe = () => {
    _q = /* @__PURE__ */ new Map();
    add(initialItems);
  };
  const getQueue = () => _q;
  const rawValues = () => Array.from(_q.values());
  const destroy = (key) => _q.delete(key);
  const getItems = (all = false) => (all ? rawValues() : rawValues().filter((i) => !i.done));
  const done = (key, shouldDestroy = false) =>
    shouldDestroy ? _q.delete(key) : (_q.get(key).done = true);
  let _q = /* @__PURE__ */ new Map();
  add(initialItems);
  return {
    add,
    set,
    wipe,
    done,
    reset,
    destroy,
    getItems,
    getQueue,
    getTypeable,
  };
};
const DATA_ATTRIBUTE = 'data-typeit-id';
const CURSOR_CLASS = 'ti-cursor';
const END = 'END';
const DEFAULT_STATUSES = {
  started: false,
  completed: false,
  frozen: false,
  destroyed: false,
};
const DEFAULT_OPTIONS = {
  breakLines: true,
  cursor: {
    autoPause: true,
    autoPauseDelay: 500,
    animation: {
      frames: [0, 0, 1].map((n) => {
        return { opacity: n };
      }),
      options: {
        iterations: Infinity,
        easing: 'steps(2, start)',
        fill: 'forwards',
      },
    },
  },
  cursorChar: '|',
  cursorSpeed: 1e3,
  deleteSpeed: null,
  html: true,
  lifeLike: true,
  loop: false,
  loopDelay: 750,
  nextStringDelay: 750,
  speed: 100,
  startDelay: 250,
  startDelete: false,
  strings: [],
  waitUntilVisible: false,
  beforeString: () => {},
  afterString: () => {},
  beforeStep: () => {},
  afterStep: () => {},
  afterComplete: () => {},
};
const PLACEHOLDER_CSS = `[${DATA_ATTRIBUTE}]:before {content: '.'; display: inline-block; width: 0; visibility: hidden;}`;
const createElement = (el) => document.createElement(el);
const createTextNode = (content) => document.createTextNode(content);
const appendStyleBlock = (styles, id = '') => {
  const styleBlock = createElement('style');
  styleBlock.id = id;
  styleBlock.appendChild(createTextNode(styles));
  document.head.appendChild(styleBlock);
};
const calculateDelay = (delayArg) => {
  if (!isArray(delayArg)) {
    delayArg = [delayArg / 2, delayArg / 2];
  }
  return delayArg;
};
const randomInRange = (value, range2) => {
  return Math.abs(Math.random() * (value + range2 - (value - range2)) + (value - range2));
};
const range = (val) => val / 2;
function calculatePace(options) {
  let { speed, deleteSpeed, lifeLike } = options;
  deleteSpeed = deleteSpeed !== null ? deleteSpeed : speed / 3;
  return lifeLike
    ? [randomInRange(speed, range(speed)), randomInRange(deleteSpeed, range(deleteSpeed))]
    : [speed, deleteSpeed];
}
const toArray = (val) => Array.from(val);
const expandTextNodes = (element) => {
  [...element.childNodes].forEach((child) => {
    if (child.nodeValue) {
      [...child.nodeValue].forEach((c) => {
        child.parentNode.insertBefore(createTextNode(c), child);
      });
      child.remove();
      return;
    }
    expandTextNodes(child);
  });
  return element;
};
const getParsedBody = (content) => {
  const doc = document.implementation.createHTMLDocument();
  doc.body.innerHTML = content;
  return expandTextNodes(doc.body);
};
function walkElementNodes(element, shouldReverse = false, shouldIncludeCursor = false) {
  const cursor = element.querySelector(`.${CURSOR_CLASS}`);
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_ALL, {
    acceptNode: (node) => {
      var _a2, _b;
      if (cursor && shouldIncludeCursor) {
        if ((_a2 = node.classList) == null ? void 0 : _a2.contains(CURSOR_CLASS)) {
          return NodeFilter.FILTER_ACCEPT;
        }
        if (cursor.contains(node)) {
          return NodeFilter.FILTER_REJECT;
        }
      }
      return ((_b = node.classList) == null ? void 0 : _b.contains(CURSOR_CLASS))
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT;
    },
  });
  let nextNode;
  const nodes = [];
  while ((nextNode = walker.nextNode())) {
    if (!nextNode.originalParent) {
      nextNode.originalParent = nextNode.parentNode;
    }
    nodes.push(nextNode);
  }
  return shouldReverse ? nodes.reverse() : nodes;
}
function chunkStringAsHtml(string) {
  return walkElementNodes(getParsedBody(string));
}
function maybeChunkStringAsHtml(str, asHtml = true) {
  return asHtml ? chunkStringAsHtml(str) : toArray(str).map(createTextNode);
}
const isNumber = (value) => Number.isInteger(value);
const countStepsToSelector = ({ queueItems, selector, cursorPosition, to }) => {
  if (isNumber(selector)) {
    return selector * -1;
  }
  const isMovingToEnd = new RegExp(END, 'i').test(to);
  let selectorIndex = selector
    ? [...queueItems].reverse().findIndex(({ char }) => {
        const parentElement = char.parentElement;
        const parentMatches = parentElement.matches(selector);
        if (isMovingToEnd && parentMatches) {
          return true;
        }
        return parentMatches && parentElement.firstChild.isSameNode(char);
      })
    : -1;
  if (selectorIndex < 0) {
    selectorIndex = isMovingToEnd ? 0 : queueItems.length - 1;
  }
  const offset = isMovingToEnd ? 0 : 1;
  return selectorIndex - cursorPosition + offset;
};
const destroyTimeouts = (timeouts) => {
  timeouts.forEach(clearTimeout);
  return [];
};
const duplicate = (value, times) => new Array(times).fill(value);
const beforePaint = (cb) => {
  return new Promise((resolve) => {
    requestAnimationFrame(async () => {
      resolve(await cb());
    });
  });
};
const getAnimationFromElement = (element) => {
  return element == null
    ? void 0
    : element.getAnimations().find((animation) => {
        return animation.id === element.dataset.tiAnimationId;
      });
};
const setCursorAnimation = ({ cursor, frames, options }) => {
  const animation = cursor.animate(frames, options);
  animation.pause();
  animation.id = cursor.dataset.tiAnimationId;
  beforePaint(() => {
    beforePaint(() => {
      animation.play();
    });
  });
  return animation;
};
const rebuildCursorAnimation = ({ cursor, options, cursorOptions }) => {
  if (!cursor || !cursorOptions) return;
  const animation = getAnimationFromElement(cursor);
  let oldCurrentTime;
  if (animation) {
    options.delay = animation.effect.getComputedTiming().delay;
    oldCurrentTime = animation.currentTime;
    animation.cancel();
  }
  const newAnimation = setCursorAnimation({
    cursor,
    frames: cursorOptions.animation.frames,
    options,
  });
  if (oldCurrentTime) {
    newAnimation.currentTime = oldCurrentTime;
  }
  return newAnimation;
};
const execute = (queueItem) => {
  var _a2;
  return (_a2 = queueItem.func) == null ? void 0 : _a2.call(null);
};
const fireItem = async ({ index, queueItems, wait: wait2, cursor, cursorOptions }) => {
  const queueItem = queueItems[index][1];
  const instantQueue = [];
  let tempIndex = index;
  let futureItem = queueItem;
  const shouldBeGrouped = () => futureItem && !futureItem.delay;
  const shouldPauseCursor = queueItem.shouldPauseCursor() && cursorOptions.autoPause;
  while (shouldBeGrouped()) {
    instantQueue.push(futureItem);
    shouldBeGrouped() && tempIndex++;
    futureItem = queueItems[tempIndex] ? queueItems[tempIndex][1] : null;
  }
  if (instantQueue.length) {
    await beforePaint(async () => {
      for (const q of instantQueue) {
        await execute(q);
      }
    });
    return tempIndex - 1;
  }
  const animation = getAnimationFromElement(cursor);
  let options;
  if (animation) {
    options = {
      ...animation.effect.getComputedTiming(),
      delay: shouldPauseCursor ? cursorOptions.autoPauseDelay : 0,
    };
  }
  await wait2(async () => {
    if (animation && shouldPauseCursor) {
      animation.cancel();
    }
    await beforePaint(() => {
      execute(queueItem);
    });
  }, queueItem.delay);
  await rebuildCursorAnimation({
    cursor,
    options,
    cursorOptions,
  });
  return index;
};
const fireWhenVisible = (element, func) => {
  const observer = new IntersectionObserver(
    (entries, observer2) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          func();
          observer2.unobserve(element);
        }
      });
    },
    { threshold: 1 }
  );
  observer.observe(element);
};
const generateHash = () => Math.random().toString().substring(2, 9);
const isInput = (el) => {
  return 'value' in el;
};
const getAllChars = (element) => {
  if (isInput(element)) {
    return toArray(element.value);
  }
  return walkElementNodes(element, true).filter((c) => !(c.childNodes.length > 0));
};
const handleFunctionalArg = (arg) => {
  return typeof arg === 'function' ? arg() : arg;
};
const select = (selector, element = document, all = false) => {
  return element[`querySelector${all ? 'All' : ''}`](selector);
};
const isBodyElement = (node) => /body/i.test(node == null ? void 0 : node.tagName);
const insertIntoElement = (originalTarget, character) => {
  if (isInput(originalTarget)) {
    originalTarget.value = `${originalTarget.value}${character.textContent}`;
    return;
  }
  character.innerHTML = '';
  const target = isBodyElement(character.originalParent)
    ? originalTarget
    : // If we add one-off fresh elements, there will be no
      // "originalParent", so always fall back to the default target.
      character.originalParent || originalTarget;
  target.insertBefore(character, select(`.${CURSOR_CLASS}`, target) || null);
};
const isNonVoidElement = (el) => /<(.+)>(.*?)<\/(.+)>/.test(el.outerHTML);
const merge = (originalObj, newObj) => Object.assign({}, originalObj, newObj);
const processCursorOptions = (cursorOptions) => {
  var _a2, _b;
  if (typeof cursorOptions === 'object') {
    const newOptions = {};
    const { frames: defaultFrames, options: defaultOptions } = DEFAULT_OPTIONS.cursor.animation;
    newOptions.animation = cursorOptions.animation || {};
    newOptions.animation.frames =
      ((_a2 = cursorOptions.animation) == null ? void 0 : _a2.frames) || defaultFrames;
    newOptions.animation.options = merge(
      defaultOptions,
      ((_b = cursorOptions.animation) == null ? void 0 : _b.options) || {}
    );
    newOptions.autoPause = cursorOptions.autoPause ?? DEFAULT_OPTIONS.cursor.autoPause;
    newOptions.autoPauseDelay =
      cursorOptions.autoPauseDelay || DEFAULT_OPTIONS.cursor.autoPauseDelay;
    return newOptions;
  }
  if (cursorOptions === true) {
    return DEFAULT_OPTIONS.cursor;
  }
  return cursorOptions;
};
const removeNode = (node, rootElement) => {
  if (!node) return;
  const nodeParent = node.parentNode;
  const nodeToRemove =
    nodeParent.childNodes.length > 1 || nodeParent.isSameNode(rootElement)
      ? // This parent still needs to exist.
        node
      : // There's nothing else in there, so just delete the entire thing.
        // By doing this, we clean up markup as we go along.
        nodeParent;
  nodeToRemove.remove();
};
const repositionCursor = (element, allChars, newCursorPosition) => {
  const nodeToInsertBefore = allChars[newCursorPosition - 1];
  const cursor = select(`.${CURSOR_CLASS}`, element);
  element = (nodeToInsertBefore == null ? void 0 : nodeToInsertBefore.parentNode) || element;
  element.insertBefore(cursor, nodeToInsertBefore || null);
};
function selectorToElement(thing) {
  return typeof thing === 'string' ? select(thing) : thing;
}
const cursorFontStyles = {
  'font-family': '',
  'font-weight': '',
  'font-size': '',
  'font-style': '',
  'line-height': '',
  color: '',
  transform: 'translateX(-.125em)',
};
const setCursorStyles = (id, element) => {
  const rootSelector = `[${DATA_ATTRIBUTE}='${id}']`;
  const cursorSelector = `${rootSelector} .${CURSOR_CLASS}`;
  const computedStyles = getComputedStyle(element);
  const customProperties = Object.entries(cursorFontStyles).reduce((accumulator, [item, value]) => {
    return `${accumulator} ${item}: var(--ti-cursor-${item}, ${value || computedStyles[item]});`;
  }, '');
  appendStyleBlock(
    `${cursorSelector} { display: inline-block; width: 0; ${customProperties} }`,
    id
  );
};
function splitOnBreak(str) {
  return str
    .replace(/<!--(.+?)-->/g, '')
    .trim()
    .split(/<br(?:\s*?)(?:\/)?>/);
}
const updateCursorPosition = (steps, cursorPosition, printedCharacters) => {
  return Math.min(Math.max(cursorPosition + steps, 0), printedCharacters.length);
};
const wait = (callback, delay, timeouts) => {
  return new Promise((resolve) => {
    const cb = async () => {
      await callback();
      resolve();
    };
    timeouts.push(setTimeout(cb, delay || 0));
  });
};
const TypeIt$1 =
  ((_a = class {
    constructor(element, options = {}) {
      __privateAdd(this, _empty);
      /**
       * Execute items in the queue.
       *
       * @param remember If false, each queue item will be destroyed once executed.
       * @returns
       */
      __privateAdd(this, _fire);
      __privateAdd(this, _move);
      /**
       * 1. Reset queue.
       * 2. Reset initial pause.
       */
      __privateAdd(this, _prepLoop);
      __privateAdd(this, _fireItemWithContext);
      __privateAdd(this, _wait);
      /**
       * Attach it to the DOM so, along with the required CSS transition.
       */
      __privateAdd(this, _attachCursor);
      __privateAdd(this, _elementIsInput);
      __privateAdd(this, _queueAndReturn);
      __privateAdd(this, _maybeAppendPause);
      __privateAdd(this, _generateTemporaryOptionQueueItems);
      __privateAdd(this, _updateOptions);
      /**
       * Based on provided strings, generate a TypeIt queue
       * to be fired for each character in the string.
       */
      __privateAdd(this, _generateQueue);
      __privateAdd(this, _prependHardcodedStrings);
      /**
       * Provided it's a non-form element and the options is provided,
       * set up the cursor element for the animation.
       */
      __privateAdd(this, _setUpCursor);
      __privateAdd(this, _addSplitPause);
      __privateAdd(this, _type);
      __privateAdd(this, _delete);
      __privateAdd(this, _removeNode);
      __privateAdd(this, _getPace);
      __privateAdd(this, _derivedCursorPosition);
      __privateAdd(this, _isInput);
      __privateAdd(this, _shouldRenderCursor);
      __privateAdd(this, _allChars);
      __publicField(this, 'element');
      __publicField(this, 'timeouts');
      __publicField(this, 'cursorPosition');
      __publicField(this, 'predictedCursorPosition');
      __publicField(this, 'statuses', {
        started: false,
        completed: false,
        frozen: false,
        destroyed: false,
      });
      __publicField(this, 'opts');
      __publicField(this, 'id');
      __publicField(this, 'queue');
      __publicField(this, 'cursor');
      __publicField(this, 'unfreeze', () => {});
      __publicField(this, 'is', function (key) {
        return this.statuses[key];
      });
      __privateAdd(this, _buildOptions, (options) => {
        options.cursor = processCursorOptions(options.cursor ?? DEFAULT_OPTIONS.cursor);
        this.opts.strings = __privateMethod(
          this,
          _prependHardcodedStrings,
          prependHardcodedStrings_fn
        ).call(this, asArray(this.opts.strings));
        this.opts = merge(this.opts, {
          html: !__privateGet(this, _isInput, isInput_get) && this.opts.html,
          nextStringDelay: calculateDelay(this.opts.nextStringDelay),
          loopDelay: calculateDelay(this.opts.loopDelay),
        });
      });
      this.opts = merge(DEFAULT_OPTIONS, options);
      this.element = selectorToElement(element);
      this.timeouts = [];
      this.cursorPosition = 0;
      this.unfreeze = () => {};
      this.predictedCursorPosition = null;
      this.statuses = merge({}, DEFAULT_STATUSES);
      this.id = generateHash();
      this.queue = Queue([{ delay: this.opts.startDelay }]);
      __privateGet(this, _buildOptions).call(this, options);
      this.cursor = __privateMethod(this, _setUpCursor, setUpCursor_fn).call(this);
      this.element.dataset.typeitId = this.id;
      appendStyleBlock(PLACEHOLDER_CSS);
      if (this.opts.strings.length) {
        __privateMethod(this, _generateQueue, generateQueue_fn).call(this);
      }
    }
    /**
     * Can only be called once.
     */
    go() {
      if (this.statuses.started) {
        return this;
      }
      __privateMethod(this, _attachCursor, attachCursor_fn).call(this);
      if (!this.opts.waitUntilVisible) {
        __privateMethod(this, _fire, fire_fn).call(this);
        return this;
      }
      fireWhenVisible(this.element, __privateMethod(this, _fire, fire_fn).bind(this));
      return this;
    }
    destroy(shouldRemoveCursor = true) {
      this.timeouts = destroyTimeouts(this.timeouts);
      handleFunctionalArg(shouldRemoveCursor) &&
        this.cursor &&
        __privateMethod(this, _removeNode, removeNode_fn).call(this, this.cursor);
      this.statuses.destroyed = true;
    }
    reset(rebuild) {
      !this.is('destroyed') && this.destroy();
      if (rebuild) {
        this.queue.wipe();
        rebuild(this);
      } else {
        this.queue.reset();
      }
      this.cursorPosition = 0;
      for (const property in this.statuses) {
        this.statuses[property] = false;
      }
      this.element[
        __privateMethod(this, _elementIsInput, elementIsInput_fn).call(this) ? 'value' : 'innerHTML'
      ] = '';
      return this;
    }
    type(string, actionOpts = {}) {
      string = handleFunctionalArg(string);
      const { instant } = actionOpts;
      const bookEndQueueItems = __privateMethod(
        this,
        _generateTemporaryOptionQueueItems,
        generateTemporaryOptionQueueItems_fn
      ).call(this, actionOpts);
      const chars = maybeChunkStringAsHtml(string, this.opts.html);
      const charsAsQueueItems = chars.map((char) => {
        return {
          func: () => __privateMethod(this, _type, type_fn).call(this, char),
          char,
          delay:
            instant || isNonVoidElement(char)
              ? 0
              : __privateMethod(this, _getPace, getPace_fn).call(this),
          typeable: char.nodeType === Node.TEXT_NODE,
        };
      });
      const itemsToQueue = [
        bookEndQueueItems[0],
        { func: async () => await this.opts.beforeString(string, this) },
        ...charsAsQueueItems,
        { func: async () => await this.opts.afterString(string, this) },
        bookEndQueueItems[1],
      ];
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        itemsToQueue,
        actionOpts
      );
    }
    break(actionOpts = {}) {
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        {
          func: () => __privateMethod(this, _type, type_fn).call(this, createElement('BR')),
          typeable: true,
        },
        actionOpts
      );
    }
    move(movementArg, actionOpts = {}) {
      movementArg = handleFunctionalArg(movementArg);
      const bookEndQueueItems = __privateMethod(
        this,
        _generateTemporaryOptionQueueItems,
        generateTemporaryOptionQueueItems_fn
      ).call(this, actionOpts);
      const { instant, to } = actionOpts;
      const numberOfSteps = countStepsToSelector({
        queueItems: this.queue.getTypeable(),
        selector: movementArg === null ? '' : movementArg,
        to,
        cursorPosition: __privateGet(this, _derivedCursorPosition, derivedCursorPosition_get),
      });
      const directionalStep = numberOfSteps < 0 ? -1 : 1;
      this.predictedCursorPosition =
        __privateGet(this, _derivedCursorPosition, derivedCursorPosition_get) + numberOfSteps;
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        [
          bookEndQueueItems[0],
          ...duplicate(
            {
              func: () => __privateMethod(this, _move, move_fn).call(this, directionalStep),
              delay: instant ? 0 : __privateMethod(this, _getPace, getPace_fn).call(this),
              cursorable: true,
            },
            Math.abs(numberOfSteps)
          ),
          bookEndQueueItems[1],
        ],
        actionOpts
      );
    }
    exec(func, actionOpts = {}) {
      const bookEndQueueItems = __privateMethod(
        this,
        _generateTemporaryOptionQueueItems,
        generateTemporaryOptionQueueItems_fn
      ).call(this, actionOpts);
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        [bookEndQueueItems[0], { func: () => func(this) }, bookEndQueueItems[1]],
        actionOpts
      );
    }
    options(opts, actionOpts = {}) {
      opts = handleFunctionalArg(opts);
      __privateMethod(this, _updateOptions, updateOptions_fn).call(this, opts);
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(this, {}, actionOpts);
    }
    pause(milliseconds, actionOpts = {}) {
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        { delay: handleFunctionalArg(milliseconds) },
        actionOpts
      );
    }
    delete(numCharacters = null, actionOpts = {}) {
      numCharacters = handleFunctionalArg(numCharacters);
      const bookEndQueueItems = __privateMethod(
        this,
        _generateTemporaryOptionQueueItems,
        generateTemporaryOptionQueueItems_fn
      ).call(this, actionOpts);
      const num = numCharacters;
      const { instant, to } = actionOpts;
      const typeableQueueItems = this.queue.getTypeable();
      const rounds = (() => {
        if (num === null) {
          return typeableQueueItems.length;
        }
        if (isNumber(num)) {
          return num;
        }
        return countStepsToSelector({
          queueItems: typeableQueueItems,
          selector: num,
          cursorPosition: __privateGet(this, _derivedCursorPosition, derivedCursorPosition_get),
          to,
        });
      })();
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        [
          bookEndQueueItems[0],
          ...duplicate(
            {
              func: __privateMethod(this, _delete, delete_fn).bind(this),
              delay: instant ? 0 : __privateMethod(this, _getPace, getPace_fn).call(this, 1),
              deletable: true,
            },
            rounds
          ),
          bookEndQueueItems[1],
        ],
        actionOpts
      );
    }
    freeze() {
      this.statuses.frozen = true;
    }
    /**
     * Like `.go()`, but more... "off the grid."
     *
     * - won't trigger `afterComplete` callback
     * - items won't be replayed after `.reset()`
     *
     * When called, all non-done items will be "flushed" --
     * that is, executed, but not remembered.
     */
    flush(cb = () => {}) {
      __privateMethod(this, _attachCursor, attachCursor_fn).call(this);
      __privateMethod(this, _fire, fire_fn).call(this, false).then(cb);
      return this;
    }
    getQueue() {
      return this.queue;
    }
    getOptions() {
      return this.opts;
    }
    updateOptions(options) {
      return __privateMethod(this, _updateOptions, updateOptions_fn).call(this, options);
    }
    getElement() {
      return this.element;
    }
    empty(actionOpts = {}) {
      return __privateMethod(this, _queueAndReturn, queueAndReturn_fn).call(
        this,
        { func: __privateMethod(this, _empty, empty_fn).bind(this) },
        actionOpts
      );
    }
  }),
  (_empty = new WeakSet()),
  (empty_fn = async function () {
    if (__privateMethod(this, _elementIsInput, elementIsInput_fn).call(this)) {
      this.element.value = '';
      return;
    }
    __privateGet(this, _allChars, allChars_get).forEach(
      __privateMethod(this, _removeNode, removeNode_fn).bind(this)
    );
    return;
  }),
  (_fire = new WeakSet()),
  (fire_fn = async function (remember = true) {
    this.statuses.started = true;
    const cleanUp = (qKey) => {
      this.queue.done(qKey, !remember);
    };
    try {
      const queueItems = [...this.queue.getQueue()];
      for (let index = 0; index < queueItems.length; index++) {
        const [queueKey, queueItem] = queueItems[index];
        if (queueItem.done) continue;
        if (
          !queueItem.deletable ||
          (queueItem.deletable && __privateGet(this, _allChars, allChars_get).length)
        ) {
          const newIndex = await __privateMethod(
            this,
            _fireItemWithContext,
            fireItemWithContext_fn
          ).call(this, index, queueItems);
          Array(newIndex - index)
            .fill(index + 1)
            .map((x, y) => x + y)
            .forEach((i) => {
              const [key] = queueItems[i];
              cleanUp(key);
            });
          index = newIndex;
        }
        cleanUp(queueKey);
      }
      if (!remember) {
        return this;
      }
      this.statuses.completed = true;
      await this.opts.afterComplete(this);
      if (!this.opts.loop) {
        throw '';
      }
      const delay = this.opts.loopDelay;
      __privateMethod(this, _wait, wait_fn).call(
        this,
        async () => {
          await __privateMethod(this, _prepLoop, prepLoop_fn).call(this, delay[0]);
          __privateMethod(this, _fire, fire_fn).call(this);
        },
        delay[1]
      );
    } catch (_e) {}
    return this;
  }),
  (_move = new WeakSet()),
  (move_fn = async function (step) {
    this.cursorPosition = updateCursorPosition(
      step,
      this.cursorPosition,
      __privateGet(this, _allChars, allChars_get)
    );
    repositionCursor(
      this.element,
      __privateGet(this, _allChars, allChars_get),
      this.cursorPosition
    );
  }),
  (_prepLoop = new WeakSet()),
  (prepLoop_fn = async function (delay) {
    const derivedCursorPosition = __privateGet(
      this,
      _derivedCursorPosition,
      derivedCursorPosition_get
    );
    derivedCursorPosition &&
      (await __privateMethod(this, _move, move_fn).call(this, {
        value: derivedCursorPosition,
      }));
    const queueItems = __privateGet(this, _allChars, allChars_get).map((_c) => {
      return [
        Symbol(),
        {
          func: __privateMethod(this, _delete, delete_fn).bind(this),
          delay: __privateMethod(this, _getPace, getPace_fn).call(this, 1),
          deletable: true,
          shouldPauseCursor: () => true,
        },
      ];
    });
    for (let index = 0; index < queueItems.length; index++) {
      await __privateMethod(this, _fireItemWithContext, fireItemWithContext_fn).call(
        this,
        index,
        queueItems
      );
    }
    this.queue.reset();
    this.queue.set(0, { delay });
  }),
  (_fireItemWithContext = new WeakSet()),
  (fireItemWithContext_fn = function (index, queueItems) {
    return fireItem({
      index,
      queueItems,
      wait: __privateMethod(this, _wait, wait_fn).bind(this),
      cursor: this.cursor,
      cursorOptions: this.opts.cursor,
    });
  }),
  (_wait = new WeakSet()),
  (wait_fn = async function (callback, delay, silent = false) {
    if (this.statuses.frozen) {
      await new Promise((resolve) => {
        this.unfreeze = () => {
          this.statuses.frozen = false;
          resolve();
        };
      });
    }
    silent || (await this.opts.beforeStep(this));
    await wait(callback, delay, this.timeouts);
    silent || (await this.opts.afterStep(this));
  }),
  (_attachCursor = new WeakSet()),
  (attachCursor_fn = async function () {
    !__privateMethod(this, _elementIsInput, elementIsInput_fn).call(this) &&
      this.cursor &&
      this.element.appendChild(this.cursor);
    if (__privateGet(this, _shouldRenderCursor, shouldRenderCursor_get)) {
      setCursorStyles(this.id, this.element);
      this.cursor.dataset.tiAnimationId = this.id;
      const { animation } = this.opts.cursor;
      const { frames, options } = animation;
      setCursorAnimation({
        frames,
        cursor: this.cursor,
        options: {
          duration: this.opts.cursorSpeed,
          ...options,
        },
      });
    }
  }),
  (_elementIsInput = new WeakSet()),
  (elementIsInput_fn = function () {
    return isInput(this.element);
  }),
  (_queueAndReturn = new WeakSet()),
  (queueAndReturn_fn = function (steps, opts) {
    this.queue.add(steps);
    __privateMethod(this, _maybeAppendPause, maybeAppendPause_fn).call(this, opts);
    return this;
  }),
  (_maybeAppendPause = new WeakSet()),
  (maybeAppendPause_fn = function (opts = {}) {
    const delay = opts.delay;
    delay && this.queue.add({ delay });
  }),
  (_generateTemporaryOptionQueueItems = new WeakSet()),
  (generateTemporaryOptionQueueItems_fn = function (newOptions = {}) {
    return [
      {
        func: () => __privateMethod(this, _updateOptions, updateOptions_fn).call(this, newOptions),
      },
      {
        func: () => __privateMethod(this, _updateOptions, updateOptions_fn).call(this, this.opts),
      },
    ];
  }),
  (_updateOptions = new WeakSet()),
  (updateOptions_fn = async function (opts) {
    this.opts = merge(this.opts, opts);
  }),
  (_generateQueue = new WeakSet()),
  (generateQueue_fn = function () {
    const strings = this.opts.strings.filter((string) => !!string);
    strings.forEach((string, index) => {
      this.type(string);
      if (index + 1 === strings.length) {
        return;
      }
      const splitItems = this.opts.breakLines
        ? [
            {
              func: () => __privateMethod(this, _type, type_fn).call(this, createElement('BR')),
              typeable: true,
            },
          ]
        : duplicate(
            {
              func: __privateMethod(this, _delete, delete_fn).bind(this),
              delay: __privateMethod(this, _getPace, getPace_fn).call(this, 1),
            },
            this.queue.getTypeable().length
          );
      __privateMethod(this, _addSplitPause, addSplitPause_fn).call(this, splitItems);
    });
  }),
  (_buildOptions = new WeakMap()),
  (_prependHardcodedStrings = new WeakSet()),
  (prependHardcodedStrings_fn = function (strings) {
    const existingMarkup = this.element.innerHTML;
    if (!existingMarkup) {
      return strings;
    }
    this.element.innerHTML = '';
    if (this.opts.startDelete) {
      this.element.innerHTML = existingMarkup;
      expandTextNodes(this.element);
      __privateMethod(this, _addSplitPause, addSplitPause_fn).call(
        this,
        duplicate(
          {
            func: __privateMethod(this, _delete, delete_fn).bind(this),
            delay: __privateMethod(this, _getPace, getPace_fn).call(this, 1),
            deletable: true,
          },
          __privateGet(this, _allChars, allChars_get).length
        )
      );
      return strings;
    }
    return splitOnBreak(existingMarkup).concat(strings);
  }),
  (_setUpCursor = new WeakSet()),
  (setUpCursor_fn = function () {
    if (__privateGet(this, _isInput, isInput_get)) {
      return null;
    }
    const cursor = createElement('span');
    cursor.className = CURSOR_CLASS;
    if (!__privateGet(this, _shouldRenderCursor, shouldRenderCursor_get)) {
      cursor.style.visibility = 'hidden';
      return cursor;
    }
    cursor.innerHTML = getParsedBody(this.opts.cursorChar).innerHTML;
    return cursor;
  }),
  (_addSplitPause = new WeakSet()),
  (addSplitPause_fn = function (items) {
    const delay = this.opts.nextStringDelay;
    this.queue.add([{ delay: delay[0] }, ...items, { delay: delay[1] }]);
  }),
  (_type = new WeakSet()),
  (type_fn = function (char) {
    insertIntoElement(this.element, char);
  }),
  (_delete = new WeakSet()),
  (delete_fn = function () {
    if (!__privateGet(this, _allChars, allChars_get).length) return;
    if (__privateGet(this, _isInput, isInput_get)) {
      this.element.value = this.element.value.slice(0, -1);
    } else {
      __privateMethod(this, _removeNode, removeNode_fn).call(
        this,
        __privateGet(this, _allChars, allChars_get)[this.cursorPosition]
      );
    }
  }),
  (_removeNode = new WeakSet()),
  (removeNode_fn = function (node) {
    removeNode(node, this.element);
  }),
  (_getPace = new WeakSet()),
  (getPace_fn = function (index = 0) {
    return calculatePace(this.opts)[index];
  }),
  (_derivedCursorPosition = new WeakSet()),
  (derivedCursorPosition_get = function () {
    return this.predictedCursorPosition ?? this.cursorPosition;
  }),
  (_isInput = new WeakSet()),
  (isInput_get = function () {
    return isInput(this.element);
  }),
  (_shouldRenderCursor = new WeakSet()),
  (shouldRenderCursor_get = function () {
    return !!this.opts.cursor && !__privateGet(this, _isInput, isInput_get);
  }),
  (_allChars = new WeakSet()),
  (allChars_get = function () {
    return getAllChars(this.element);
  }),
  _a);
const DynamicElementComponent = forwardRef((props, ref) => {
  const { as: As } = props;
  return /* @__PURE__ */ React.createElement(As, { ref, ...props });
});
const defaultPropOptions = {};
const TypeIt = ({
  as = 'span',
  options = defaultPropOptions,
  children = null,
  getBeforeInit = (instance) => instance,
  getAfterInit = (instance) => instance,
  ...remainingProps
}) => {
  const elementRef = useRef(null);
  const instanceRef = useRef(null);
  const [shouldShowChildren, setShouldShowChildren] = useState(true);
  const [instanceOptions, setInstanceOptions] = useState(null);
  function calculateOptions() {
    const optionsClone = Object.assign({}, options);
    if (children && elementRef.current) {
      optionsClone.strings = elementRef.current.innerHTML;
    }
    setInstanceOptions(optionsClone);
  }
  function generateNewInstance() {
    instanceRef.current = new TypeIt$1(elementRef.current, instanceOptions);
    instanceRef.current = getBeforeInit(instanceRef.current);
    instanceRef.current.go();
    instanceRef.current = getAfterInit(instanceRef.current);
  }
  useEffect(() => {
    calculateOptions();
    setShouldShowChildren(false);
  }, [calculateOptions]);
  useEffect(() => {
    var _a2;
    if (!instanceOptions) return;
    ((_a2 = instanceRef.current) == null ? void 0 : _a2.updateOptions(instanceOptions)) ||
      generateNewInstance();
  }, [instanceOptions, generateNewInstance]);
  useEffect(() => {
    return () => {
      var _a2;
      return (_a2 = instanceRef.current) == null ? void 0 : _a2.destroy();
    };
  }, []);
  return /* @__PURE__ */ React.createElement(DynamicElementComponent, {
    ref: elementRef,
    as,
    children: shouldShowChildren ? children : null,
    style: { opacity: shouldShowChildren ? 0 : 1 },
    ...remainingProps,
  });
};
export { TypeIt as default };
