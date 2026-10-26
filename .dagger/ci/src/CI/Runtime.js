// The only FFI in the CI modules: credentials injected by the Dagger engine.
export const sessionCredentials = () => ({
  port: Number(process.env.DAGGER_SESSION_PORT),
  token: process.env.DAGGER_SESSION_TOKEN ?? "",
});
