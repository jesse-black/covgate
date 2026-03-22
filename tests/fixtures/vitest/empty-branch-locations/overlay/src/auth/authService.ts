const fallbackStorage = new Map<string, string>();
const tokenStorageKey = "fixture-token";

export const hasBrowserStorage = () =>
  typeof window !== "undefined" && typeof window.localStorage !== "undefined";

export const persistAuthToken = (token: string) => {
  const normalizedToken = token.trim();

  if (!normalizedToken) {
    throw new Error("token cannot be empty");
  }

  if (hasBrowserStorage()) {
    window.localStorage.setItem(tokenStorageKey, normalizedToken);
    return;
  }

  fallbackStorage.set(tokenStorageKey, normalizedToken);
};

export const readAuthToken = () => {
  const fallbackToken = fallbackStorage.get(tokenStorageKey);

  if (hasBrowserStorage()) {
    return window.localStorage.getItem(tokenStorageKey) ?? fallbackToken ?? null;
  }

  return fallbackToken ?? null;
};
