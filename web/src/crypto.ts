/**
 * The only decryption the browser does: word locks (to check an answer) and
 * the certificate (with every word). Released puzzles and hints arrive in
 * plain form. Every constant here must match the crypto spec and the Rust
 * engine byte for byte; `test-vectors.json` holds both to it.
 */

export const WORD_ITERATIONS = 150_000;
export const FINAL_ITERATIONS = 200_000;

/** A day's word, encrypted with a key derived from one accepted answer. */
export interface WordLock {
  salt: string;
  iv: string;
  ct: string;
}

/** The sealed certificate, as published at `/<event>/api/final.json`. */
export interface SealedFinal {
  salt: string;
  iv: string;
  ct: string;
}

/** What the certificate decrypts to. Images are base64 WebP. */
export interface Certificate {
  title: string;
  message: string;
  image?: string;
  /** Where `image` goes; "bottom" when absent. */
  image_position?: "top" | "bottom";
  subtitle?: string;
  signature?: string;
  background?: string;
  rewards?: Reward[];
}

/** A reward revealed with the certificate: a link, or a file to download. */
export interface Reward {
  label: string;
  url?: string;
  file?: {
    name: string;
    size: number;
    /** Published at `<event>/api/rewards/<index>.bin`. */
    index: number;
    /** Base64 AES-256-GCM key and IV for that file. */
    key: string;
    iv: string;
  };
}

/** Decrypt a reward file with the key from the certificate. */
export async function openReward(
  file: NonNullable<Reward["file"]>,
  ciphertext: ArrayBuffer,
): Promise<Uint8Array<ArrayBuffer> | null> {
  const key = await subtle().importKey(
    "raw",
    fromBase64(file.key),
    "AES-GCM",
    false,
    ["decrypt"],
  );
  try {
    const plain = await subtle().decrypt(
      { name: "AES-GCM", iv: fromBase64(file.iv) },
      key,
      ciphertext,
    );
    return new Uint8Array(plain);
  } catch {
    return null;
  }
}

/**
 * Browsers only expose WebCrypto in secure contexts (https, or localhost),
 * so a calendar served over plain http cannot check answers.
 */
export class InsecureContextError extends Error {
  constructor() {
    super(
      "Checking answers needs a secure connection: open this calendar over https.",
    );
  }
}

function subtle(): SubtleCrypto {
  const api = globalThis.crypto?.subtle;
  if (!api) throw new InsecureContextError();
  return api;
}

/** NFD, strip combining marks, lowercase, keep only `[a-z0-9]`. */
export function normalize(input: string): string {
  return input
    .normalize("NFD")
    .replace(/\p{M}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]/g, "");
}

export function fromBase64(b64: string): Uint8Array<ArrayBuffer> {
  const binary = atob(b64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

async function deriveKey(
  secret: string,
  salt: Uint8Array<ArrayBuffer>,
  iterations: number,
): Promise<CryptoKey> {
  const base = await subtle().importKey(
    "raw",
    new TextEncoder().encode(secret),
    "PBKDF2",
    false,
    ["deriveKey"],
  );
  return subtle().deriveKey(
    { name: "PBKDF2", hash: "SHA-256", salt, iterations },
    base,
    { name: "AES-GCM", length: 256 },
    false,
    ["decrypt"],
  );
}

/** AES-GCM decrypt; a wrong key or tampered data yields null, never throws. */
async function decrypt(
  key: CryptoKey,
  iv: string,
  ct: string,
): Promise<Uint8Array | null> {
  try {
    const plain = await subtle().decrypt(
      { name: "AES-GCM", iv: fromBase64(iv) },
      key,
      fromBase64(ct),
    );
    return new Uint8Array(plain);
  } catch {
    return null;
  }
}

/** Try a guess against a lock: the word if the answer is right, else null. */
export async function openWord(
  answer: string,
  lock: WordLock,
): Promise<string | null> {
  const normalized = normalize(answer);
  if (!normalized) return null;
  const key = await deriveKey(
    normalized,
    fromBase64(lock.salt),
    WORD_ITERATIONS,
  );
  const plain = await decrypt(key, lock.iv, lock.ct);
  return plain && new TextDecoder().decode(plain);
}

/** Open the sealed certificate with every word, in day order. */
export async function openFinalBytes(
  words: string[],
  sealed: SealedFinal,
): Promise<Uint8Array | null> {
  const key = await deriveKey(
    words.join(" "),
    fromBase64(sealed.salt),
    FINAL_ITERATIONS,
  );
  return decrypt(key, sealed.iv, sealed.ct);
}

export async function openFinal(
  words: string[],
  sealed: SealedFinal,
): Promise<Certificate | null> {
  const plain = await openFinalBytes(words, sealed);
  return plain && (JSON.parse(new TextDecoder().decode(plain)) as Certificate);
}
