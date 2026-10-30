import vectors from "../../test-vectors.json";
import {
  FINAL_ITERATIONS,
  normalize,
  openFinalBytes,
  openWord,
  WORD_ITERATIONS,
} from "./crypto";

const text = new TextDecoder();

describe("normalize", () => {
  it.each(vectors.normalize)("normalizes $note", ({ input, normalized }) => {
    expect(normalize(input)).toBe(normalized);
  });
});

describe("openWord", () => {
  it.each(vectors.word)("opens with $answer", async (v) => {
    expect(await openWord(v.answer, v)).toBe(v.word);
  });

  it("returns null for a wrong answer", async () => {
    expect(await openWord("london", vectors.word[0])).toBeNull();
  });

  it("returns null for an answer that normalizes to nothing", async () => {
    expect(await openWord("東京", vectors.word[0])).toBeNull();
  });
});

describe("openFinalBytes", () => {
  it.each(vectors.final)("opens with $words", async (v) => {
    const plain = await openFinalBytes(v.words, v);
    expect(plain && text.decode(plain)).toBe(v.plaintext);
  });

  it("returns null when the words are out of order", async () => {
    const v = vectors.final[0];
    expect(await openFinalBytes([...v.words].reverse(), v)).toBeNull();
  });
});

it("uses the iteration counts of the crypto spec", () => {
  expect(WORD_ITERATIONS).toBe(150000);
  expect(FINAL_ITERATIONS).toBe(200000);
});
