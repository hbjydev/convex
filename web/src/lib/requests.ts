import type { components } from '@/api/schema';

class RequestFailure extends Error {}

export function unwrap<T>(result: {
  data?: T;
  error?: components['schemas']['ApiError'];
  response: Response;
}): T {
  if (!result.response.ok || result.data === undefined) {
    throw new RequestFailure(
      result.error?.message ?? 'The request failed. Try again.',
    );
  }
  return result.data;
}

export function errorMessage(error: unknown) {
  return error instanceof RequestFailure
    ? error.message
    : 'Could not reach the server. Check your connection and try again.';
}
