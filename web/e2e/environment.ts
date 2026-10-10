function port(name: string, fallback: number): number {
  const value = process.env[name] ?? String(fallback);
  const number = Number(value);
  if (
    !/^\d+$/.test(value) ||
    !Number.isInteger(number) ||
    number < 1 ||
    number > 65535
  )
    throw new Error(`${name} must be an integer from 1 to 65535`);
  return number;
}

export const apiPort = port('CONVEX_E2E_API_PORT', 3001);
export const webPort = port('CONVEX_E2E_WEB_PORT', 5174);
export const apiOrigin = `http://127.0.0.1:${apiPort}`;
export const webOrigin = `http://127.0.0.1:${webPort}`;
