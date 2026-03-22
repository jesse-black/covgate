const envAuthority = process.env.FIXTURE_AUTHORITY;
const envClientId = process.env.FIXTURE_CLIENT_ID;

export const msalConfig = {
  auth: {
    authority: envAuthority,
    clientId: envClientId,
  },
};

export const resolveAuthority = () => {
  if (msalConfig.auth.authority) {
    return msalConfig.auth.authority;
  }

  return "https://login.example.test/common";
};

export const resolveClientId = () => {
  if (msalConfig.auth.clientId) {
    return msalConfig.auth.clientId;
  }

  if (process.env.NODE_ENV === "test") {
    return "fixture-client-id";
  }

  return "fixture-client-id-dev";
};
