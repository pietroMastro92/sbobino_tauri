## [LRN-20260325-001] correction

**Logged**: 2026-03-25T12:35:00+01:00
**Priority**: medium
**Status**: pending
**Area**: frontend

### Summary
Transcript-specific display controls belong in the right inspector, not in the left navigation sidebar.

### Details
The confidence-color toggle for transcript rendering was initially added to the left app sidebar. User feedback clarified that this control must live in the right inspector panel, next to transcript-specific controls, because it affects the currently open detail view rather than global navigation.

### Suggested Action
When adding transcript, segment, or detail-view options, prefer the right inspector unless the control changes app-level navigation or global layout.

### Metadata
- Source: user_feedback
- Related Files: sbobino_desktop/apps/desktop/src/App.tsx, sbobino_desktop/apps/desktop/src/styles.css
- Tags: inspector, sidebar, transcript, ux

---
