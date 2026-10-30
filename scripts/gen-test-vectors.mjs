// Generates the seal cases in test-vectors.json with WebCrypto, the same API
// the browser uses. The Rust crate is tested against these, so they must not
// be produced by the Rust code itself: that would only check it against itself.
//
// Usage: node scripts/gen-test-vectors.mjs
// Inputs are fixed, so rerunning must leave the file unchanged.

import { readFileSync, writeFileSync } from "node:fs";
import { webcrypto } from "node:crypto";

const { subtle } = webcrypto;
const WORD_ITERATIONS = 150_000;
const FINAL_ITERATIONS = 200_000;
const path = new URL("../test-vectors.json", import.meta.url);

const enc = new TextEncoder();
const b64 = (bytes) => Buffer.from(bytes).toString("base64");
// Deterministic bytes from a label, so each case has distinct fixed inputs.
const fixed = (label, length) =>
  Uint8Array.from({ length }, (_, i) => (label.charCodeAt(i % label.length) * 31 + i * 7) & 0xff);

const normalize = (s) =>
  s.normalize("NFD").replace(/\p{M}/gu, "").toLowerCase().replace(/[^a-z0-9]/g, "");

async function pbkdf2Key(secret, salt, iterations) {
  const base = await subtle.importKey("raw", enc.encode(secret), "PBKDF2", false, ["deriveKey"]);
  return subtle.deriveKey(
    { name: "PBKDF2", hash: "SHA-256", salt, iterations },
    base,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt"],
  );
}

async function aesEncrypt(key, iv, plaintext) {
  return new Uint8Array(await subtle.encrypt({ name: "AES-GCM", iv }, key, plaintext));
}

async function rawKey(bytes) {
  return subtle.importKey("raw", bytes, "AES-GCM", false, ["encrypt"]);
}

async function wordCase(label, answer, word) {
  const salt = fixed(`${label}-salt`, 16);
  const iv = fixed(`${label}-iv`, 12);
  const key = await pbkdf2Key(normalize(answer), salt, WORD_ITERATIONS);
  const ct = await aesEncrypt(key, iv, enc.encode(word));
  return { answer, normalized: normalize(answer), word, salt: b64(salt), iv: b64(iv), ct: b64(ct) };
}

async function keyedCase(label, plaintext) {
  const keyBytes = fixed(`${label}-key`, 32);
  const iv = fixed(`${label}-iv`, 12);
  const ct = await aesEncrypt(await rawKey(keyBytes), iv, enc.encode(plaintext));
  return { key: b64(keyBytes), iv: b64(iv), plaintext, ct: b64(ct) };
}

async function finalCase(label, words, plaintext) {
  const salt = fixed(`${label}-salt`, 16);
  const iv = fixed(`${label}-iv`, 12);
  const key = await pbkdf2Key(words.join(" "), salt, FINAL_ITERATIONS);
  const ct = await aesEncrypt(key, iv, enc.encode(plaintext));
  return { words, salt: b64(salt), iv: b64(iv), plaintext, ct: b64(ct) };
}

const vectors = JSON.parse(readFileSync(path, "utf8"));

vectors.word = [
  await wordCase("w1", "Paris", "light"),
  await wordCase("w2", "  L'Été 2014! ", "returns"),
  await wordCase("w3", "Ångström", "after"),
];
vectors.bundle = [
  await keyedCase("b1", '{"title":"Day 1","type":"teaser","markdown":"# Hello","assets":{}}'),
  await keyedCase("b2", "puzzle body with ünïcödé"),
  await keyedCase("b3", ""),
];
vectors.hint = [
  await keyedCase("h1", "Think about capitals."),
  await keyedCase("h2", "The year matters."),
  await keyedCase("h3", "Look closer at the picture: 🔍"),
];
vectors.final = [
  await finalCase("f1", ["light", "returns", "after"], '{"title":"Well done","message":"You made it."}'),
  await finalCase("f2", ["one"], "single word"),
  await finalCase("f3", ["light", "returns", "after", "the", "longest", "night"], "Certificate"),
];

writeFileSync(path, JSON.stringify(vectors, null, 2) + "\n");
console.log("test-vectors.json: seal cases written");
