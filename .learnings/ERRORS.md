## [ERR-20260409-001] github_release_upload

**Logged**: 2026-04-09T17:44:58Z
**Priority**: high
**Status**: pending
**Area**: infra

### Summary
GitHub release asset upload became inconsistent during `v0.1.10` prerelease publishing, leaving partial uploads and forcing manual recovery.

### Error
```
HTTP 422: Validation Failed
tag_name is not a valid tag
Published releases must have a valid tag
Release.target_commitish is invalid

error connecting to api.uploads.github.com

Later release attempts created partial draft/untagged states and asset uploads without stable completion feedback.
```

### Context
- Command/operation attempted: `gh release create`, `gh release upload`, `gh api` upload for `v0.1.10`
- Input or parameters used: local release artifacts under `sbobino_desktop/dist/local-release/v0.1.10`
- Environment details if relevant: public GitHub repo, local candidate branch, large release assets including `pyannote-runtime-macos-aarch64.zip`

### Suggested Fix
Always push the candidate commit and the release tag before creating the prerelease, then upload assets incrementally with post-upload verification instead of a single monolithic upload flow.

### Metadata
- Reproducible: unknown
- Related Files: sbobino_desktop/dist/local-release/v0.1.10, sbobino_desktop/scripts/prepare_local_release.sh

---
