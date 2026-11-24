# How it's built

Back to the [project README](../README.md). This page says who does what on this project and how much work
that has been so far. It is a summary: the rules themselves live in [AGENTS.md](../AGENTS.md) and
[CONTRIBUTING.md](../CONTRIBUTING.md), and are not repeated here.

## Who does what

- **AI coding agents write the code.** They also write the tests, the tools and these documents, and they
  run the checks.
- **The owner directs.** One person sets the scope of each milestone, rules on every decision the agents
  bring to him (the rulings are recorded in the investigation notes, with their dates) and plays the game
  by hand. Agents may not merge: every merge is the owner's.
- **Nobody claims every line was read by a person.** AI-written code can be wrong, and passing tests does
  not guarantee correctness.

## How a change gets in

1. For a change to behavior, an agent first reads the original C source and writes down what it found, with file and line.
2. The change is built on its own branch, with tests, and opened as a pull request.
3. Behavior is checked against the genuine engine: id Software's Linux DOOM 1.10 source runs the same inputs,
   and the results are compared tic by tic ([how](VALIDATION.md), [commands](TESTING.md)).
4. A fresh agent that did not write the change reviews it.
5. The owner reviews the pull request and merges it, or sends it back.

The review rules, the gates a pull request must pass and the limits on what an agent may do are in
[AGENTS.md](../AGENTS.md).

## How much work so far

Figures as of 2026-11-23 (America/Los_Angeles), each rounded down:

- **More than three weeks of history.** The first pull request was merged on 2026-10-27
  ([GitHub](https://github.com/Copgunn/DOOM/pulls?q=is%3Apr+is%3Amerged)).
- **More than 70 pull requests merged**, each by the owner.
- **More than 350,000 tics compared with the genuine engine on real Episode 1 maps**, each run counted
  once: the 61 scripted runs ([M4b record](validation/M4B.md#real-maps-and-demo1)) and the owner's
  hand-played sessions ([M5c](validation/M5C.md#hand-played-checks),
  [M5d](validation/M5D.md#hand-played-checks),
  [M6a](validation/M6A.md#5-after-the-hand-test-gate-2026-11-22-evening)). Runs on synthetic maps and the
  earlier per-function comparisons are not in this figure.

These figures measure effort, not correctness. What is and is not supported is in
[COMPATIBILITY](COMPATIBILITY.md).
