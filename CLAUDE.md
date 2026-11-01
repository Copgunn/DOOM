# Agent entry point

Follow [AGENTS.md](AGENTS.md) as the standing engineering instructions; this file only
adds session conventions that AGENTS.md does not state.

- Commit identity comes from the repo-local git config set by the owner (a GitHub
  noreply address, no personal name or email). Never add `Co-Authored-By` trailers,
  "Generated with" lines or any AI attribution to commits, PR titles or PR bodies.
- Never commit to or push `master`. Never force-push, rebase or squash shared history,
  delete branches or enable auto-merge. Never merge a PR; merging is the owner's call.
- Open one focused PR per milestone, draft until every gate passes; mark it ready only
  after the gates, then leave it unmerged.
- Run every performance measurement alone on an idle machine (no concurrent builds,
  tests or fuzzing) and say so in the record.
- Keep scratch builds, baseline checkouts and fuzz corpora outside the working tree;
  delete them at the end of a milestone after their results are recorded.
- Report only observed results; distinguish target compiled, seed replay, coverage-guided
  campaign and sanitizer campaign, and disclose disabled leak detection.
