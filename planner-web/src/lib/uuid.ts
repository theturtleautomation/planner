/**
 * Generate a v4 UUID that works in all environments.
 *
 * `crypto.randomUUID()` requires a Secure Context (HTTPS / localhost).
 * When serving over plain HTTP (e.g. a LAN IP), it throws.
 * This helper falls back to `crypto.getRandomValues()` which works everywhere.
 */
export function uuidv4(): string {
  // Fast path — available in secure contexts (HTTPS, localhost, modern Node)
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return crypto.randomUUID();
  }

  // Fallback using getRandomValues — works in any browser and Node ≥15
  // RFC 4122 §4.4 compliant
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);

  // Set version (4) and variant (10xx) bits
  bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
  bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1

  const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
  return [
    hex.slice(0, 8),
    hex.slice(8, 12),
    hex.slice(12, 16),
    hex.slice(16, 20),
    hex.slice(20, 32),
  ].join('-');
}
