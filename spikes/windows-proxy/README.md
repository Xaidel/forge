# Windows proxy spike

Throwaway evidence harness for wayfinder ticket
[#24](https://github.com/Xaidel/forge/issues/24). It answers one question: can
Forge run a domain-allowlisting HTTP/CONNECT proxy for the MXC worker on
Windows, using MXC's peer-scoped proxy path, or must Windows ship
`Sandboxed — network denied`?

The answer is **yes**. See [`EVIDENCE.md`](EVIDENCE.md) for the verdict,
policy, observations, failure modes, and TDD-0006 inputs. This code is evidence,
not a Forge adapter. Do not merge it.

## What it does

`proxy-spike` orchestrates four variants:

| Variant | Proxy | MXC policy |
|---|---|---|
| `identityless` | normal host process | `hostLoopback=allow`, no `allowedProxyPeer`; weak development path, not selected |
| `identity-scoped-appcontainer` | Forge proxy in a per-user AppContainer | `hostLoopback=deny`, `allowedProxyPeer=<profile>`; selected path |
| `identity-scoped-normal-proxy` | normal host process under a peer-scoped policy | expected failure; proves peer binding |
| `network-denied` | none | deny-all fallback |

`proxy-client` runs inside MXC and checks:

- allowlisted HTTP and HTTPS/CONNECT;
- unlisted HTTP and CONNECT denial;
- redirect revalidation for allowed, unlisted, and private targets;
- loopback, private, link-local, metadata, and DNS-rebinding denial;
- direct egress, direct DNS, and direct loopback denial;
- the injected `HTTP_PROXY`/`HTTPS_PROXY` environment;
- a real `curl.exe` HTTP and HTTPS request through the proxy;
- mid-task allowlist grant and revoke (`403 -> 200 -> 403`);
- absence of model/cloud secrets in the worker environment.

`appcontainer-probe` launches `proxy-spike --token-probe` in the same
AppContainer profile and records DNS/TCP behavior. This is how the
`SE_GROUP_ENABLED` capability-attribute requirement was found.

## Build and run

```powershell
cd spikes\windows-proxy
cargo build --bins
.\target\debug\appcontainer-probe.exe
.\target\debug\proxy-spike.exe
```

The run needs public `example.com` and `httpbin.org` access. It uses `.test`
names only for synthetic redirect and DNS-rebinding fixtures. The host
preflight records whether direct egress and DNS were reachable before the
sandbox run, so a denial inside the sandbox is not a false positive.

## Files

- [`EVIDENCE.md`](EVIDENCE.md): the report and TDD-0006 inputs.
- [`evidence-windows-proxy.txt`](evidence-windows-proxy.txt): full run log.
- [`evidence-appcontainer-token-probe.txt`](evidence-appcontainer-token-probe.txt): AppContainer DNS/TCP probe.
- `src/bin/proxy-spike.rs`: host orchestration and policy variants.
- `src/bin/proxy-client.rs`: worker-side checks.
- `src/bin/appcontainer-probe.rs`: AppContainer token probe.
- `src/proxy.rs`: HTTP/CONNECT proxy, allowlist, DNS/address checks, logging.
- `src/appcontainer.rs`: profile, ACL, and `CreateProcessW` launch helpers.

## Notes

- The proxy AppContainer must be launched with
  `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES` and capability
  `SID_AND_ATTRIBUTES.Attributes = SE_GROUP_ENABLED`. `0` launches in the
  AppContainer but leaves it without network access.
- The host cannot connect to the AppContainer proxy; the MXC worker can. Use a
  file or pipe control channel, not a host HTTP health check.
- `CheckNetIsolation LoopbackExempt` requires admin and is not needed for the
  worker -> proxy direction.
- The branch is evidence only; it is not merged.
