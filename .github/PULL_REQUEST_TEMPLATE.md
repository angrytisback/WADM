## Summary

<!-- Provide a concise description of what this PR changes and why. Reference the issue it closes if applicable. -->

Closes #

---

## Type of Change

- [ ] Bug fix
- [ ] New feature or endpoint
- [ ] Performance optimization
- [ ] Security hardening
- [ ] Refactoring without behavioral change
- [ ] Documentation update
- [ ] CI/CD or build system change

---

## Implementation Notes

<!-- Describe any non-obvious design decisions, trade-offs, or implementation details reviewers should be aware of. -->

---

## Pre-Submission Checklist

- [ ] `cargo fmt --all -- --check` produces no diff.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes with zero warnings.
- [ ] `cargo test --verbose` passes with all tests green.
- [ ] Frontend: `cd web && npm run lint` reports no errors.
- [ ] Frontend: `cd web && npx tsc -b` reports no type errors.
- [ ] New or modified behavior is covered by automated tests where applicable.
- [ ] Documentation has been updated to reflect the change (README, API docs, or inline doc comments).
- [ ] No secrets, credentials, or runtime-generated files (`.wadm_jwt_secret`, `wadm-auth.json`) are included in the diff.
