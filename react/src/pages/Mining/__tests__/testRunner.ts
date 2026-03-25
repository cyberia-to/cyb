export type TestResult = {
  name: string;
  passed: boolean;
  error?: string;
  duration: number;
};

type TestFn = () => Promise<void>;

type TestSuite = {
  name: string;
  fn: TestFn;
};

export function assert(condition: boolean, message: string): void {
  if (!condition) {
    throw new Error(`Assertion failed: ${message}`);
  }
}

export function assertType(value: unknown, type: string, label: string): void {
  assert(typeof value === type, `${label} should be ${type}, got ${typeof value}`);
}

export function assertDefined(value: unknown, label: string): void {
  assert(value !== undefined && value !== null, `${label} should be defined`);
}

export function assertPositive(value: number, label: string): void {
  assert(value > 0, `${label} should be positive, got ${value}`);
}

export async function runTests(tests: TestSuite[]): Promise<TestResult[]> {
  const results: TestResult[] = [];
  for (const test of tests) {
    const start = performance.now();
    try {
      await test.fn();
      results.push({
        name: test.name,
        passed: true,
        duration: performance.now() - start,
      });
    } catch (err: any) {
      results.push({
        name: test.name,
        passed: false,
        error: err?.message || String(err),
        duration: performance.now() - start,
      });
    }
  }
  return results;
}
