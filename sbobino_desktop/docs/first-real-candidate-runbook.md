# First Real Candidate Runbook

## Goal

Bring the new release pipeline into real use without guesswork:

1. register all three self-hosted runners
2. preflight each machine
3. confirm the runner matrix is online on GitHub
4. dispatch the candidate workflow
5. watch it through hosted and self-hosted validation
6. promote only if the full matrix is green

## Required machines

- `AS-PRIMARY`
- `AS-THIRD`
- `INTEL-PRIMARY`

Reference labels and machine prep are documented in [self-hosted-release-runners.md](self-hosted-release-runners.md).

## Step 1: Install the runner on each Mac

Run this on each target machine:

```bash
cd /path/to/sbobino_tauri/sbobino_desktop
./scripts/install_self_hosted_runner_macos.sh <MACHINE_CLASS> pietroMastro92/Sbobino
```

Examples:

```bash
./scripts/install_self_hosted_runner_macos.sh AS-PRIMARY pietroMastro92/Sbobino
./scripts/install_self_hosted_runner_macos.sh AS-THIRD pietroMastro92/Sbobino
./scripts/install_self_hosted_runner_macos.sh INTEL-PRIMARY pietroMastro92/Sbobino
```

## Step 2: Preflight each machine

Run this on the same machine after installation:

```bash
./scripts/preflight_self_hosted_runner.sh <MACHINE_CLASS> pietroMastro92/Sbobino
```

Notes:

- `AS-PRIMARY` and `AS-THIRD` require `SBOBINO_VALIDATION_FIXTURE_AUDIO`
- `INTEL-PRIMARY` does not require the audio fixture
- preflight fails closed if the GitHub runner is not online with the expected labels

## Step 3: Confirm the full matrix is online

Run from any machine with GitHub CLI access:

```bash
./scripts/check_release_runner_matrix.sh pietroMastro92/Sbobino
```

This must report all three classes online before dispatching a real candidate.

## Step 4: Dispatch the candidate

```bash
./scripts/dispatch_release_candidate.sh v<version> pietroMastro92/Sbobino
```

Example:

```bash
./scripts/dispatch_release_candidate.sh v0.1.17 pietroMastro92/Sbobino
```

The dispatch helper refuses to start if the runner matrix is incomplete.

## Step 5: Watch the run

```bash
./scripts/watch_release_candidate.sh pietroMastro92/Sbobino
```

Expected successful jobs:

- `release-readiness`
- `publish-candidate`
- `distribution-readiness`
- `validate-as-primary`
- `validate-as-third`
- `validate-intel-primary`
- `upload-validation-assets`

## Step 6: Promote only after green validation

Use either GitHub Actions `Promote Release Candidate` or:

```bash
./scripts/promote_candidate_release.sh <version> pietroMastro92/Sbobino
```

## Failure rule

If any mandatory job or machine validation fails:

- do not patch the candidate in place
- retire the prerelease
- fix the issue
- cut a new patch version
- rerun the candidate flow
