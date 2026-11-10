# Security

## Reporting a vulnerability

Use GitHub's private vulnerability reporting on this repository: **Security ->
Advisories -> Report a vulnerability**. That opens a private thread visible only
to the maintainers, and it is the preferred route because it keeps the report
out of public issues while it is being fixed.

If private reporting is unavailable to you, open a public issue that says only
that you have a security report and asks for a contact address. Do not put the
details in it.

Expect a first response within five working days. If a report is valid, the fix
and the advisory are published together, and you are credited unless you ask not
to be.

## What this project is, for threat-modelling purposes

Open Ontologies is a local engine. It runs as an MCP server over stdio, as a
CLI, and optionally as an HTTP server. It has no LLM inside it, makes no
outbound calls in a default build, and holds no credentials of its own.

`serve-http` binds to `127.0.0.1` by default and takes an optional bearer token
via `--token` or `OPEN_ONTOLOGIES_TOKEN`. There is no TLS in the binary. If you
expose it beyond localhost, terminate TLS at a reverse proxy and set a token.
Running it on `0.0.0.0` without a token gives anyone who can reach the port full
read and write access to the loaded graphs, and that is the configuration most
likely to hurt someone.

`/mcp` also checks the `Host` header against DNS rebinding and accepts only
`localhost`, `127.0.0.1` and `::1` unless told otherwise. A server reached by
another name (a LAN address, a container name, the reverse proxy's public
name) needs it in `[http] allowed_hosts`, `OPEN_ONTOLOGIES_HTTP_ALLOWED_HOSTS`
or `--allowed-host`. `"*"` turns the check off, which is only reasonable
behind a proxy that sets `Host` itself. `/mcp` refuses request bodies over
4 MiB unless `[http] max_request_body_bytes` raises the cap.

The `plugins` feature runs community WASM modules in-process on wasmi with fuel
metering. Fuel bounds runtime, it does not sandbox intent. Treat a plugin as
code you are choosing to run.

## In scope

Anything that lets an attacker read or modify a graph they should not reach,
escape the intended process boundary, execute code through a crafted input file
or plugin, or cause unrecoverable denial of service through a small input.
Parser and reasoner inputs are attacker-controlled in many deployments, so a
crash or unbounded memory growth on a crafted Turtle, RDF/XML or SHACL file is a
valid report.

## Not vulnerabilities

A reasoner verdict of `undetermined` when the budget is exhausted is the
designed behaviour. It exists so an incomplete run is never reported as a
proof, and returning it is the safe outcome rather than a failure.

SHACL validation under open-world semantics not rejecting an unknown term is
also designed, and it is a property of SHACL rather than of this
implementation. Constraints the validator cannot execute suppress the verdict
and are listed under `skipped_constraints` instead of being silently passed. If
you find a case where an unexecuted constraint is reported as conforming, that
one is a bug and we want to hear about it.

Long reasoning times on a deliberately hard ontology are expected. Description
logic satisfiability is intractable in the worst case, which is why the budget
and the three-valued verdict exist.

## Supported versions

Fixes land on the latest minor release. There are no long-term support
branches.
