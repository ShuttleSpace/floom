# Security Policy

## Reporting a vulnerability

Please do **not** open a public GitHub issue for security problems.

Report vulnerabilities privately via GitHub's "Report a vulnerability" button on
the **Security** tab of this repository. This notifies the maintainers directly
and keeps the report confidential until a fix is available.

If that is not possible, open a GitHub issue describing the affected component
and ask for a private contact channel, **without** including details that would
let an attacker reproduce the issue.

## Scope

The crates under `crates/` and the capability plugins under
`crates/ecosystem/plugins/` are the primary attack surface. Reports are welcome
for anything that could compromise the integrity, confidentiality, or
availability of an application built with Floom, in particular:

- unsound or `unsafe` code in the rendering, platform, or native-composition
  layers (`floom-native-ffi`, `floom-platform-*`, `floom-renderer-*`);
- the local development transports in `floom-agent` and `floom-devtools`, which
  listen on localhost with a dev token — reports about their exposure or token
  handling are especially valuable;
- dependency-supply-chain issues.

## Supported versions

Floom is pre-1.0 and security fixes are applied to the latest `main` and the
most recent tagged release. Please confirm the issue reproduces on `main` before
reporting.

## Disclosure

Once a fix is released, we will acknowledge the report, publish an advisory on
the GitHub Security Advisories page, and credit the reporter (unless anonymity
is requested).
