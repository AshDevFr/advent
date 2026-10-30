# Example days

One sample day per puzzle type. Copy one to `events/<event>/dayNN/` to start
a real day; from that point its plaintext is gitignored and only the sealed
output gets committed.

This folder is the one place plaintext is committed on purpose: these are
public examples. Never write a real puzzle here.

| Day | Type    | What it shows                                              |
|-----|---------|------------------------------------------------------------|
| 01  | teaser  | a riddle with alternate accepted answers                   |
| 02  | code    | a code block; its numeric answer triggers the guessable warning |
| 03  | company | trivia about a (fictional) company, with the facts provided |
| 04  | cipher  | a Caesar cipher                                            |
| 05  | logic   | a deduction whose answer combines two names                |
| 06  | image   | an image the answer is hidden in                           |
| 07  | meta    | an answer built from earlier days' answers                 |

The seven words, in order, are the final key: `light returns after the
longest winter night`. `final/final.yaml` is the sample certificate.
