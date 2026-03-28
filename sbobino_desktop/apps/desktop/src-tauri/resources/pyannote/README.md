Place bundled offline pyannote assets here.

Use `scripts/setup_bundled_pyannote.sh` from the workspace root to populate
this directory on a macOS build machine without committing the downloaded
payloads to git.

Expected bundle layout:

- `resources/pyannote/model/`
  A local pyannote diarization pipeline directory containing `config.yaml`
  and the referenced model files.
- `resources/pyannote/python/`
  A bundled Python runtime for the current target, or arch-specific runtimes at:
  - `resources/pyannote/python/aarch64-apple-darwin/bin/python3`
  - `resources/pyannote/python/x86_64-apple-darwin/bin/python3`

The app will look for these resources automatically at runtime and will run
speaker diarization without any user path configuration when they are present.
