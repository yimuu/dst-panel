# Frontend Settings And Profile Optimization Design

## Goal

Improve the Vue frontend's Settings and User Profile pages so they feel complete, explicit, and safer to operate. This phase removes avoidable placeholder text, clarifies which settings are actually persisted by the backend, adds focused validation before saving, and improves account information/password handling without changing backend APIs.

## Scope

This design covers:

- `web-ui/src/pages/SettingsPage.vue`
- `web-ui/src/pages/UserProfilePage.vue`
- Small feature/shared helpers needed by those pages
- Focused unit/component tests and regenerated root `dist/` assets

This phase does not include world configuration editing, browser smoke/E2E, or broad UI redesign. Those remain later optimization batches.

## Non-Goals

- Do not change Rust backend routes, request payloads, response envelopes, or auth behavior.
- Do not introduce i18n; this frontend remains Chinese-only.
- Do not add a new UI component library or validation dependency.
- Do not redesign the `web-ui/` directory structure.
- Do not expose settings that cannot be loaded or saved through existing backend contracts.

## Current State

The Settings page already loads and saves `DstConfig` through `getDstConfig()` and `saveDstConfig()`, but it still shows a broad alert saying registration and Steam API Key persistence are unavailable. The page also normalizes data locally inside the SFC, with limited pre-save validation and no clear distinction between required paths and optional paths.

The User Profile page reads the current user from `authStore` and supports password changes through `changePassword()`. It still displays `待接入` for account ID and creation time when the backend does not return those fields. Password saving only checks for a non-empty value.

## Recommended Approach

Keep the existing page structure and Element Plus form layout, then improve behavior in place. Extract only small helpers when they create direct reuse or testable behavior:

- Settings normalization and validation should live in `features/settings/settings-form.ts` if it is page-specific but testable.
- User display-field normalization and password validation should live in `features/auth` or a small profile helper if the logic grows beyond the page.
- Shared UI helpers should only be added when used by more than one page.

This avoids a large refactor while making the two pages visibly more complete.

## Settings Page Design

The Settings page should present `dst_config` as the persisted source of truth. The existing informational alert should be replaced with a concise note that the page saves server path, cluster storage, and runtime parameters. It should not mention registration or Steam API Key unless those fields are actually shown on the page.

Validation should run before `saveDstConfig()`:

- `steamcmd` is required.
- `force_install_dir` is required.
- `cluster` is required.
- `backup` is required.
- `mod_download_path` is required.
- `bin` must be `32` or `64`.
- `beta` must be `0` or `1`.
- `donot_starve_server_directory`, `ugc_directory`, `persistent_storage_root`, and `conf_dir` remain optional strings.

Validation failures should show a Chinese `ElMessage.error()` and avoid sending the API request. Save success should preserve the current behavior: save, reload settings, then show `设置已保存`.

Normalization should stay conservative:

- Trim all string fields before save.
- Keep missing optional fields as empty strings.
- Coerce invalid `bin` to `32` only while normalizing loaded backend data.
- Coerce invalid `beta` to `0` only while normalizing loaded backend data.
- Do not silently coerce invalid user-submitted `bin` or `beta`; validation should reject impossible values.

## User Profile Page Design

The User Profile page should stop presenting missing backend data as unfinished frontend work. If account ID or creation time is missing, display `暂无数据` rather than `待接入`.

Displayed fields should be normalized as:

- Username: `displayName`, then `username`, then `name`, then `未登录`.
- Role: backend role value if present, otherwise `管理员`.
- Account ID: `id`, then `ID`, then `暂无数据`.
- Created time: `createdAt`, then `created_at`, then `暂无数据`.

Password change should add client-side validation before `changePassword()`:

- New password is required.
- New password must be at least 6 characters after trimming.
- Leading and trailing whitespace should not be submitted.
- After a successful change, clear the input and keep the existing success message.

No current-password field should be added unless the backend requires it. No password strength meter should be added in this phase.

## Error Handling

Both pages should continue using `assertApiSuccess()` and `getErrorMessage()` from `shared/api/envelope.ts`. Backend `msg` and `message` fields remain the preferred user-facing error text. Client-side validation errors should be direct Chinese strings and should not be wrapped as thrown errors.

## Testing Strategy

Use TDD for implementation:

- Extend `settings-form.test.ts` to cover normalization and validation.
- Extend Settings page tests to verify invalid forms block `saveDstConfig()`.
- Add or extend profile tests to verify missing fields render as `暂无数据`.
- Test password validation blocks short passwords and submits trimmed valid passwords.
- Keep full verification commands:
  - `npm run test:unit -- --run`
  - `npm run type-check`
  - `npm run lint:check`
  - `npm run format:check`
  - `npm run build`

Because the repository tracks root `dist/`, implementation must include regenerated `dist/index.html` and new hashed assets after `npm run build`.

## Rollout

Implement in two commits:

1. Settings page normalization and validation.
2. User profile display and password validation.

Each commit should have focused tests and should leave the worktree clean. If either page reveals missing backend contracts, stop at a clear frontend fallback rather than adding speculative API calls.

## Success Criteria

- Settings page no longer displays unrelated unavailable-feature copy.
- Settings validation blocks empty required fields before API calls.
- Settings save still sends the normalized `DstConfig` payload.
- User Profile page no longer shows `待接入`.
- Password change rejects empty and short passwords before API calls.
- All frontend checks and production build pass.
