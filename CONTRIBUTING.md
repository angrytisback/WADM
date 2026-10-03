# Contributing to WADM

Thank you for your interest in contributing to WADM (Web Administration for Linux).
This document covers the technical requirements for all contributions.

## Prerequisites

- **Rust**: Latest stable toolchain (`rustup default stable`) with `clippy` and `rustfmt` components.
- **Node.js**: v20 LTS or higher with npm v10+.
- **Docker**: Required for App Store and container module testing.
- **Linux host**: WADM targets Linux exclusively. WSL2 or a Linux VM is acceptable for development.

## Local Development Setup

```bash
git clone https://github.com/angrytisback/WADM.git
cd WADM
cd web && npm install && cd ..
```

Start the frontend dev server (Vite, port 5173):

```bash
cd web && npm run dev
```

Start the backend in a separate terminal (Actix-Web, port 8080):

```bash
RUST_LOG=debug cargo run
```

The frontend dev proxy is configured in `vite.config.ts` to forward `/api` requests to `localhost:8080`.

## Quality Gates

Every pull request must pass the following checks locally before submission.
CI enforces these checks and will reject any PR that fails.

### Backend

```bash
# Check code formatting (must produce no diff)
cargo fmt --all -- --check

# Apply formatting
cargo fmt --all

# Run strict linter (zero warnings tolerated)
cargo clippy --all-targets --all-features -- -D warnings

# Run all unit tests
cargo test --verbose
```

### Frontend

```bash
cd web

# Check TypeScript types
npx tsc -b

# Run ESLint
npm run lint

# Verify production build succeeds
npm run build
```

## Commit Convention

Commits must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
<type>(<scope>): <subject>
```

Accepted types:

| Type       | When to use                                               |
|------------|-----------------------------------------------------------|
| `feat`     | New feature or endpoint                                   |
| `fix`      | Bug fix                                                   |
| `perf`     | Performance improvement without behavioral change         |
| `refactor` | Code restructuring without feature or fix                 |
| `test`     | Adding or updating automated tests                        |
| `docs`     | Documentation only changes                                |
| `chore`    | Build process, dependency updates, tooling configuration  |
| `ci`       | Changes to CI/CD workflow files                           |

Examples:

```
feat(apps): add Portainer CE deployment template
fix(auth): remove panic on corrupted auth store
perf(monitor): replace subshell ip-route with /proc/net/route parser
```

## Pull Request Submission

1. Fork the repository and create a feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```
2. Implement your changes with appropriate test coverage.
3. Verify all quality gates pass locally.
4. Push your branch and open a pull request against `main`.
5. Complete the pull request checklist provided by the template.

Pull requests that do not pass CI checks or are missing a description of the changes will not be reviewed.

## Branch Protection

The `main` branch requires:
- All CI status checks passing (Formatting, Clippy, Backend Tests, Frontend Lint).
- At least one approving review from a maintainer.
- No direct pushes; changes must arrive via pull request.

## Security Issues

Do not open public issues for security vulnerabilities. See [SECURITY.md](SECURITY.md) for the responsible disclosure process.
