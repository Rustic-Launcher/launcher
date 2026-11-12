/** Error raised by the cloud client (real or mock): `code` is the API's stable error code. */
export class CloudError extends Error {
  constructor(
    public code: string,
    message: string,
    public status: number,
  ) {
    super(message);
    this.name = 'CloudError';
  }
}

/** Stable error code of any thrown value (`CloudError`, `IpcFailure`...), if it has one. */
export function errorCode(e: unknown): string | null {
  if (e && typeof e === 'object' && 'code' in e && typeof e.code === 'string') return e.code;
  return null;
}
