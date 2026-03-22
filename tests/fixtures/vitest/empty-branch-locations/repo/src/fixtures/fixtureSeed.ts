import {
  persistAuthToken,
  readAuthToken,
} from "../auth/authService";
import {
  resolveAuthority,
  resolveClientId,
} from "../auth/msalConfig";

type FixtureSeed = {
  authority: string;
  clientId: string;
  token: string | null;
};

export function applyFixtureSeed(value: string): FixtureSeed {
  const trimmedValue = value.trim();

  if (!trimmedValue) {
    persistAuthToken("fallback-token");
  } else {
    persistAuthToken(trimmedValue);
  }

  return {
    authority: resolveAuthority(),
    clientId: resolveClientId(),
    token: readAuthToken(),
  };
}
