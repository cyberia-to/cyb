// Minimal assert shim for ipld-dag-pb and other Node.js packages
// that depend on the 'assert' module
function assert(value, message) {
  if (!value) {
    throw new Error(message || 'Assertion failed');
  }
}

assert.ok = assert;

assert.equal = function (actual, expected, message) {
  if (actual != expected) {
    throw new Error(message || `${actual} != ${expected}`);
  }
};

assert.strictEqual = function (actual, expected, message) {
  if (actual !== expected) {
    throw new Error(message || `${actual} !== ${expected}`);
  }
};

assert.deepEqual = assert.deepStrictEqual = function (actual, expected, message) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(message || 'Deep equality assertion failed');
  }
};

assert.notEqual = function (actual, expected, message) {
  if (actual == expected) {
    throw new Error(message || `${actual} == ${expected}`);
  }
};

assert.fail = function (message) {
  throw new Error(message || 'Assert.fail()');
};

module.exports = assert;
module.exports.default = assert;
