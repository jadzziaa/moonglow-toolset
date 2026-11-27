---
type: Decision
title: "NWSync sign-in: user.toml only, bound to one host, redirects followed by hand, no env var"
description: "How Moonglow's NWSync auth (token or Basic) is read, where it is sent, and why redirects are followed in Client::get rather than by ureq."
tags: [nwsync]
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T09:35:51Z }
touches: [crates/mg-nwsync/src/auth.rs, crates/mg-nwsync/src/fetch.rs, "crates/mg-module/src/project_file.rs:SignIn"]
asks:
  - Why does the NWSync client follow redirects itself instead of ureq?
  - Where is the NWSync token read from and why not moonglow.toml?
  - Why is there no MOONGLOW_NWSYNC_TOKEN environment variable?
  - When is the Authorization header sent to an NWSync server?
fingerprint: v2-c392ab308fa7e17a
created: 2026-11-27
author: augustaugur
---
TDoD request for 5.5.0 (2026-11-27). [nwsync] token, or user+password, with host: read from .moonglow/user.toml alone (SignIn in project_file.rs); in moonglow.toml or a .mod's NAME.moonglow.toml refused with file:line and reason, never sent. host defaults to the host of a source set in the SAME user.toml, never the team's: a checkout could otherwise name an attacker's server and receive the token. Settings.auth carries mg_nwsync::Auth (hand-written Debug, <hidden>).

Sending: Auth::sent_to = host match and (https or loopback). fetch::for_settings refuses auth for an http:// non-loopback source on its host. Client uses max_redirects(0) and follows up to 5 redirects itself so the header goes on only to the same (scheme, host, port) as the first URL; ureq's RedirectAuthHeaders::SameHost ignores the port, Never drops it on same-origin redirects too. Client::sent_to also checks ureq::http::Uri's host (both readings must agree). A loopback auth builds the agent with proxy(None) (HTTP_PROXY would see the header in clear).

No leaks: source with userinfo (https://u:p@h, and scheme-less u:p@h) refused without echoing, in project file and --source; host warning never echoes text; TOML parse errors were checked not to echo an unquoted token. mg nwsync-status/pull now print project-file warnings as notes, and append them to an error (otherwise a team-file token gives a bare 401).

No env var: a variable has no host binding and would follow whatever source a checkout names; CI writes user.toml from its secret.
