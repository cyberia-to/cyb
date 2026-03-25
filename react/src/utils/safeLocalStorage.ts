/**
 * Safe localStorage wrappers that handle:
 * - Private browsing mode (Safari QuotaExceededError)
 * - Disabled localStorage
 * - Corrupt JSON data from old versions
 */

function getItem(key: string, defaultValue: string | null = null): string | null {
  try {
    return localStorage.getItem(key) ?? defaultValue;
  } catch {
    return defaultValue;
  }
}

function setItem(key: string, value: string): boolean {
  try {
    localStorage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

function getJSON<T>(key: string, defaultValue: T): T {
  try {
    const raw = localStorage.getItem(key);
    if (raw === null) {
      return defaultValue;
    }
    return JSON.parse(raw) as T;
  } catch {
    return defaultValue;
  }
}

function setJSON(key: string, value: unknown): boolean {
  try {
    localStorage.setItem(key, JSON.stringify(value));
    return true;
  } catch {
    return false;
  }
}

function removeItem(key: string): boolean {
  try {
    localStorage.removeItem(key);
    return true;
  } catch {
    return false;
  }
}

const safeLocalStorage = {
  getItem,
  setItem,
  getJSON,
  setJSON,
  removeItem,
};

export default safeLocalStorage;
