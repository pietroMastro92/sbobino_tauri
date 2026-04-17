# Self-Hosted Release Runners

## Goal

Use GitHub Actions as the orchestrator while running distribution-critical validation on real Macs already available to the team.

## Required runners

### `AS-PRIMARY`

- machine: primary Apple Silicon Mac used by the team
- labels:
  - `self-hosted`
  - `macos`
  - `apple-silicon`
  - `as-primary`
- purpose:
  - real upgrade-path validation from the latest stable public release
  - warm restart validation
  - diarization smoke after update

### `AS-THIRD`

- machine: third-party Apple Silicon Mac reserved for clean-room validation
- labels:
  - `self-hosted`
  - `macos`
  - `apple-silicon`
  - `as-third`
- purpose:
  - clean-room install from the public DMG
  - first-launch setup validation
  - warm restart validation
  - diarization smoke on a machine without developer residue

### `INTEL-PRIMARY`

- machine: Intel MacBook Pro x86_64
- labels:
  - `self-hosted`
  - `macos`
  - `x64`
  - `intel-primary`
- purpose:
  - bootstrap-layer validation for the arm64 release process
  - manifest/updater/download flow verification
  - future bridge toward Intel and Windows expansion

## GitHub runner registration

Register each runner at repo or organization scope with the exact label sets above.

Recommended service mode:

- run the GitHub runner as a persistent launch agent or service
- enable automatic start after reboot
- keep the runner online only on trusted machines

## Security boundaries

- use these runners only from trusted workflows and trusted tags
- do not expose secrets to workflows triggered from forks
- keep release publication and promote flows on `workflow_dispatch`
- prefer repository or organization variables for non-secret paths and fixtures

## Required local tooling

Install and keep available on every runner:

- Xcode command line tools
- Rust toolchain
- `cargo`
- `python3`
- `curl`
- `hdiutil`
- `ditto`

Apple Silicon runners also need enough free disk for the full first-launch runtime and pyannote installation.

## Workspace hygiene

Each self-hosted job should start from a clean workspace.

Minimum practice:

- remove old checkout directories before the runner starts the next job
- keep validation output inside the checked-out repo or the runner temp directory
- do not reuse old validation JSON files between runs

## Validation fixture

Apple Silicon machine validation requires:

- environment or repository variable: `SBOBINO_VALIDATION_FIXTURE_AUDIO`

This must point to an absolute path on the runner host for a short audio file with at least two speakers.

The validation flow is fail-closed:

- if the fixture is missing, `AS-PRIMARY` and `AS-THIRD` fail
- the candidate must not be promoted to stable

## Clean-room guidance for `AS-THIRD`

Preferred setup:

- use a dedicated macOS user account only for release validation

Minimum acceptable setup:

- remove `/Applications/Sbobino.app`
- remove `~/Library/Application Support/com.sbobino.desktop`
- ensure the validation does not depend on Homebrew or developer-installed runtime state

## Expected workflow contract

1. Hosted GitHub Actions builds the candidate and publishes the prerelease.
2. Hosted GitHub Actions runs `distribution_readiness.sh` and uploads `distribution-readiness-proof.json`.
3. Self-hosted runners validate the exact public release assets and upload:
   - `AS-PRIMARY.validation-report.json`
   - `AS-THIRD.validation-report.json`
   - `INTEL-PRIMARY.validation-report.json`
4. Stable promotion remains manual and blocked unless all required reports are present and valid.
