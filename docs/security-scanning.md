# Security scanning

Drug-Ya handles HOSxP credentials and patient data. A secret that reaches a
public GitHub repository must be treated as compromised even if it is deleted
in a later commit, because it remains in the git history and in forks.

This document covers the secret-scanning layers around the repository. It does
not replace [`SECURITY.md`](../SECURITY.md) or the hard rules in
[`AGENTS.md`](../AGENTS.md).

## Layers

| Layer | Where it runs | Purpose |
|---|---|---|
| `.gitignore` and review | Local, before commit | Keep credentials, patient data, and build output out of git |
| `ggshield` pre-commit hook | Local, optional | Block a secret before it enters a commit |
| GitHub secret scanning | GitHub, after push | Detect supported provider tokens in the repository |
| GitHub push protection | GitHub, before accepting a push | Block supported provider tokens before they land |
| GitGuardian | GitGuardian dashboard / GitHub App | Deep detection, historical scan, pull request checks, incident workflow |

No single layer is sufficient. GitHub push protection only covers supported
provider patterns; it does not replace a review of configuration files, test
fixtures, and documentation.

## Configuration file

The `.gitguardian.yaml` file in the repository root configures `ggshield` only.
It does not change the GitGuardian dashboard, the GitGuardian GitHub App, or
GitHub native secret scanning.

The file deliberately excludes only generated output:

- `target/**`
- `apps/drug-ya-app/frontend/dist/**`
- `apps/drug-ya-app/gen/**`
- `**/node_modules/**`
- `**/.trunk/**`

Do not add `src/`, `docs/`, or `*.rs` to `ignored_paths`. That hides future real
secrets in the same area.

The only `ignored_matches` entries are synthetic test fixtures. Prefer changing
a fixture over adding a new exception.

## Local scanning with ggshield

Install once:

```bash
pipx install ggshield
ggshield auth login
```

Scan the working tree and the git history:

```bash
ggshield secret scan path -r .
ggshield secret scan repo .
```

The pre-commit hook is optional:

```bash
pipx install pre-commit
pre-commit install
```

The hook is defined in [`.pre-commit-config.yaml`](../.pre-commit-config.yaml)
and calls the local `ggshield` binary. It does not run unless pre-commit is
installed.

If `ggshield` reports a finding, do not paste the secret value into a public
issue or pull request. Rotate the credential first, then clean the tree and
decide whether history rewriting is required.

## False positives

Prefer one of these, in order:

1. Change the fixture or example so it is obviously synthetic and is not a
   valid credential shape.
2. Add a `# ggignore` comment on the exact line when the value must stay.
3. Add a narrow `ignored_matches` entry in `.gitguardian.yaml` with a name that
   explains why the value is safe.
4. Ignore the incident in the GitGuardian dashboard after review.

Never add a broad path exclusion to make a finding disappear.

## GitHub repository setup

After the first push, confirm in **Settings > Code security and analysis**:

- Secret scanning: enabled.
- Push protection: enabled.
- Non-provider patterns: enable if the account plan offers it.
- Validity checks: enable if the account plan offers it.
- Dependabot alerts: enabled.
- Private vulnerability reporting: enabled.

Install the GitGuardian GitHub App for the repository and enable pull request
check runs. Add the GitGuardian check to the `main` branch protection rule once
the first pull request has produced a check run.

Then protect `main` with a ruleset or branch protection rule:

- Require a pull request before merging.
- Require the `CI` status check.
- Require the branch to be up to date before merging.
- Block force pushes and deletion.

## What must never be committed

- HOSxP hosts, usernames, passwords, or connection strings.
- `.env` files, key files, certificates, settings exports, and connection
  profiles.
- Patient names, HN, CID, addresses, phone numbers, visit data, or drug lines.
- Real hospital screenshots, logs, or database dumps.
- A `settings.json` or `connection.json` copied from a workstation.

If a secret is committed:

1. Revoke or rotate it immediately. Deleting the file is not enough.
2. Remove it from the current tree.
3. Decide whether history rewriting is required. For a public repository,
   assume the value is already exposed.
4. Record the incident and the rotation in the private security channel, not in
   a public issue.
