# Security Policy

## Supported Versions

Security patches are applied exclusively to the latest minor release.
Older versions do not receive backported fixes.

| Version | Supported |
|---------|-----------|
| 0.96.x  | Yes       |
| < 0.96  | No        |

## Threat Model

WADM is a system administration panel that operates with elevated host privileges
through a scoped sudoers configuration. It integrates directly with:

- The Docker Unix socket (`/var/run/docker.sock`)
- Linux procfs and sysfs interfaces (`/proc`, `/sys`)
- The host systemd control plane via `systemctl`
- A PTY-backed web terminal under Developer Mode

Security issues in these integration surfaces are considered **critical severity**
and will be prioritized accordingly.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities through GitHub's private Security Advisory mechanism:

1. Navigate to the [Security tab](https://github.com/angrytisback/WADM/security/advisories).
2. Select **Report a vulnerability**.
3. Complete the advisory draft with the information requested below.

Alternatively, contact the maintainer directly via GitHub: [@angrytisback](https://github.com/angrytisback).

### Required Information

Include the following in your report:

- **Summary**: A concise description of the vulnerability and its class (e.g., SSRF, path traversal, privilege escalation).
- **Affected component**: Module name, file path, or API endpoint.
- **Reproduction steps**: Minimal steps to reproduce the issue reliably.
- **Impact assessment**: Describe what an attacker can achieve if the vulnerability is exploited.
- **Proof of concept**: Code, request payloads, or commands demonstrating the issue.
- **Suggested fix**: Optional, but appreciated.

### Response Timeline

| Stage                     | Target Duration       |
|---------------------------|-----------------------|
| Initial acknowledgment    | Within 48 hours       |
| Severity triage           | Within 5 business days|
| Fix development           | Dependent on severity |
| Coordinated public disclosure | After fix is released |

## Scope

The following are considered in-scope security issues:

- Authentication bypass or session token forgery
- JWT secret extraction or manipulation
- Path traversal in the file manager or credential storage
- Privilege escalation beyond the sudoers whitelist
- Remote code execution via crafted API requests
- Docker socket abuse through WADM's API surface
- Cross-site scripting (XSS) in the web UI

The following are considered out of scope:

- Issues requiring physical access to the server
- Vulnerabilities in software dependencies where no WADM-specific mitigating control exists and the upstream vendor has not released a fix
- Denial-of-service through resource exhaustion by authenticated administrative users
