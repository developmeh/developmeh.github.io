/**
 * jco's calling convention for a host-implemented import that returns
 * `result<T, E>`: return T plainly, throw an object carrying `payload` to
 * return E. `getErrorPayload` in the transpiled glue reads own-property
 * `payload` first, so an Error subclass keeps its stack and still marshals.
 *
 * Its own file because `host-core.js` and `identity.js` both throw these and
 * neither should have to import the other.
 */
export class WitError extends Error {
  constructor(tag, val) {
    super(`${tag}${val === undefined ? '' : `: ${val}`}`);
    this.payload = val === undefined ? { tag } : { tag, val };
  }
}
