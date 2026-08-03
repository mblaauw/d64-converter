# Agent Coding Guidelines

Follow these rules for every coding task unless the user explicitly overrides them.

## 1. Understand Before Editing

Before changing files:

* Read the relevant code, configuration, tests, and nearby documentation.
* Identify the smallest set of files likely required.
* Briefly state:

  * your understanding of the goal;
  * any important assumptions;
  * the intended implementation and verification steps.

Do not begin implementation based only on filenames or guesses about the repository.

### Handling ambiguity

Use reasonable inference for small, low-risk details when the repository already establishes a clear pattern.

Ask a clarifying question only when the uncertainty:

* materially changes behavior, architecture, security, data, or public APIs;
* has multiple significantly different implementation paths;
* could cause destructive or difficult-to-reverse changes;
* cannot be resolved by inspecting the repository.

Do not block progress over naming, formatting, or other easily reversible details. State the assumption and continue.

## 2. Prefer the Simplest Complete Solution

Implement the smallest solution that fully satisfies the request.

* Prefer direct code over unnecessary abstractions.
* Reuse existing functions, utilities, libraries, conventions, and patterns.
* Do not introduce speculative extensibility or future-proofing.
* Do not add configuration options unless they are required.
* Do not create wrappers that merely rename or forward existing behavior.
* Avoid new dependencies when the existing stack can solve the problem cleanly.
* Do not implement requirements that were not requested or clearly implied.

If the solution becomes disproportionately complex, stop and reconsider whether a simpler implementation exists.

## 3. Make Surgical Changes

Limit changes to what is required for the task.

* Do not refactor unrelated code.
* Do not reformat entire files.
* Do not rename unrelated symbols.
* Do not rewrite working code merely to match personal preferences.
* Do not fix unrelated warnings or failing tests unless they prevent completion.
* Do not modify generated files, lockfiles, migrations, snapshots, or vendored code unless the task requires it.
* Remove temporary code, debug output, unused imports, and files introduced by your changes.

When an unrelated issue is discovered, report it separately instead of silently expanding the scope.

## 4. Preserve Existing Behaviour

Unless the task explicitly requires a breaking change:

* preserve public APIs and command-line interfaces;
* preserve configuration compatibility;
* preserve existing defaults;
* preserve error semantics where practical;
* follow the repository’s established architecture and style.

Do not silently change behavior outside the requested scope.

## 5. Work Toward Explicit Success Criteria

Before implementation, translate the request into observable success criteria.

For bug fixes:

* identify the failing behavior;
* reproduce it when feasible;
* add or update a regression test where appropriate;
* verify that the test fails before the fix when practical.

For features:

* define the expected inputs, outputs, edge cases, and failure behavior;
* add focused tests for the new behavior;
* avoid broad snapshot tests when direct assertions are clearer.

Tests should validate behavior, not merely mirror implementation details.

## 6. Verify Before Finishing

Run the narrowest relevant checks first, followed by broader checks when practical.

Use the repository’s own documented commands and tooling. Typical checks include:

* targeted unit or integration tests;
* formatter or formatting check;
* linter;
* type checker such as `tsc`, `pyright`, or `mypy`;
* build or compile command;
* relevant end-to-end or smoke test.

Do not claim that a command passed unless it was actually run successfully.

If a check cannot be run:

* state which check was skipped;
* explain why;
* describe any remaining risk.

Do not hide existing failures. Clearly distinguish failures caused by your changes from failures already present in the repository.

## 7. Protect Data and Repository State

Do not perform destructive or irreversible operations without explicit permission.

This includes:

* deleting user data;
* resetting databases;
* force-pushing;
* rewriting Git history;
* deleting branches or tags;
* running destructive migrations;
* discarding uncommitted changes;
* using commands such as `git reset --hard`, `git clean -fd`, or equivalent destructive operations.

Never overwrite or remove changes that you did not create.

Do not expose, log, commit, or hard-code secrets, tokens, credentials, private keys, or sensitive environment values.

## 8. Keep Communication Focused

During the task:

* provide brief progress updates for multi-step work;
* report blockers as soon as they are known;
* avoid narrating routine tool usage;
* do not present internal chain-of-thought or speculative reasoning.

When finished, provide:

1. a concise summary of what changed;
2. the important files changed;
3. the verification commands run and their results;
4. any assumptions, limitations, or remaining risks.

Do not produce a long retrospective unless requested.

## 9. Definition of Done

A task is complete only when:

* the requested behavior is implemented;
* the changes are limited to the intended scope;
* relevant tests or checks have been run;
* temporary artifacts have been removed;
* the final response accurately describes what was and was not verified.
