# Security Policy

## Reporting a vulnerability

Please do not report security vulnerabilities through public GitHub Issues,
Discussions, or pull requests.

Instead, use GitHub's private vulnerability reporting:

1. Go to the **Security** tab of this repository.
2. Click **Report a vulnerability**.
3. Fill in the advisory form with as much detail as you can: affected
   version(s), a description of the issue, and, if possible, steps to
   reproduce or a proof of concept.

This opens a private draft security advisory visible only to you and the
maintainer, so the issue can be discussed and fixed before anything is
public.

## What to expect

rustdv is currently maintained by one person, during a beta/evaluation
release. Response times are best-effort, not a guaranteed SLA — expect an
initial acknowledgment within a few days. If a report is confirmed, a fix
will be prepared privately and a GitHub Security Advisory published
alongside the patched release, crediting the reporter unless they'd rather
stay anonymous.

## Scope

rustdv loads as a native shared library into a Verilog simulator over VPI
and drives simulation from Rust. The most relevant class of concern is
memory-safety issues at the `unsafe` boundary — that code lives entirely in
`rustdv-gpi`, by design. The framework itself does no network I/O, so
network-facing vulnerabilities aren't applicable here. Vulnerabilities in
the simulators rustdv drives (Icarus Verilog and friends) are out of
scope for this repository — report those upstream, to the simulator's own
project.

## Supported versions

rustdv is pre-1.0 and moving quickly. Security fixes target the latest
published release on crates.io; older 0.x versions are not backported.
