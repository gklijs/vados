# Project working agreements

- Follow the issue order in `PLAN.md` and keep its progress current.
- Leave all Git commits and pushes to the user. Do not commit or push changes,
  or request permission to do so.

## Validation and pull requests

- Default to focused local tests for the changed behaviour, plus formatting
  and relevant lint checks. Use GitHub Actions for the full test suite,
  browser tests and minimum-Rust-version checks.
- If a local test requires permissions unavailable in the sandbox (such as
  binding a local HTTP server), record that limitation and defer that test
  to GitHub CI rather than requesting escalation just to run it.
- Once the user has committed and pushed the changes to a feature branch,
  open or update a focused PR. If changes are still local, prepare the PR
  title and description and explain that the commit and push remain with
  the user.
- Describe the behaviour change, link the issue, and distinguish checks
  passed locally from checks pending in CI. Do not claim completion of
  validation until the required CI checks pass.
- Inspect CI results and fix failures related to the change. Leave any
  follow-up commits and pushes to the user as well.
