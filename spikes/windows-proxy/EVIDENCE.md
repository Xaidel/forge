# Windows proxy spike: evidence

- **Ticket:** [Spike the Windows proxy path for sandbox network allowlisting](https://github.com/Xaidel/forge/issues/24)
- **Map:** [Forge v1 — Technical Design](https://github.com/Xaidel/forge/issues/19)
- **Date:** 2026-11-04
- **SDK:** `mxc-sdk` 1.0.1
- **Spike branch:** `spike/windows-proxy` (throwaway; do not merge)
- **Harness:** [`spikes/windows-proxy`](https://github.com/Xaidel/forge/tree/spike/windows-proxy/spikes/windows-proxy)
- **Raw run:** [`evidence-windows-proxy.txt`](https://github.com/Xaidel/forge/blob/spike/windows-proxy/spikes/windows-proxy/evidence-windows-proxy.txt)
- **AppContainer probe:** [`evidence-appcontainer-token-probe.txt`](https://github.com/Xaidel/forge/blob/spike/windows-proxy/spikes/windows-proxy/evidence-appcontainer-token-probe.txt)

## Verdict

**The strong peer-scoped proxy path passes on Windows 11 25H2 x64 under MXC `base-container`.**

Windows can ship **`Sandboxed` with network allowlisting**. It does **not** need to fall back to `Sandboxed — network denied` on a supported host when the Forge proxy AppContainer can be created and started. The fallback remains the correct fail-closed mode when the proxy cannot be provisioned or started; the identityless loopback proxy is **not** an acceptable substitute.

| Variant | Proxy deployment | MXC policy | Result |
|---|---|---|---|
| `identityless` | normal host process, loopback listener | `egress=deny`, `ingress=allow`, `hostLoopback=allow`, `runtimeConfig.networkProxy`, no `allowedProxyPeer` | **PASS**, but this is the weak development/testing path the PRD rejects. |
| `identity-scoped-appcontainer` | Forge proxy in a per-user unpackaged AppContainer profile | `egress=deny`, `ingress=allow`, `hostLoopback=deny`, `runtimeConfig.networkProxy`, `allowedProxyPeer=<profile name>` | **PASS**: selected path. |
| `identity-scoped-normal-proxy` | normal host process, but policy names an AppContainer peer it does not have | same peer-scoped policy as above | **EXPECTED-FAIL**: every proxy connection timed out; peer binding is enforced. |
| `network-denied` | no proxy | `egress=deny`, `ingress=deny`, `hostLoopback=deny`, no proxy | **PASS**: all proxy and direct egress attempts denied; no loopback mutation. |

Host: Windows 11 Home Single Language, version 10.0.26200, build 26200 (25H2), x64; Rust 1.96.0 MSVC. The PRD floor is Windows 11 24H2 build 26100.9278+; this host is above the floor. A 24H2 run remains a release-checklist item.

## What was built

A two-binary Rust harness plus an AppContainer probe:

- `proxy-spike` (host): creates fixtures, starts the proxy, creates/launches the AppContainer profile, builds the MXC request, runs the worker, and checks host-side invariants.
- `proxy-client` (worker): runs inside MXC and exercises HTTP, HTTPS/CONNECT, redirects, DNS/private/metadata/loopback denials, direct egress denials, the injected proxy environment, and a dynamic allowlist update.
- `appcontainer-probe`: launches `proxy-spike --token-probe` in an AppContainer and records DNS/TCP behavior.

The proxy is a deliberately small Forge-owned HTTP/CONNECT proxy:

- Binds a caller-supplied `127.0.0.1:<ephemeral>` endpoint.
- Accepts HTTP absolute-form requests and `CONNECT host:port`.
- Loads an allowlist from a file and watches it for mid-task updates.
- Resolves hostnames in the proxy. The tested worker made no successful direct DNS query.
- Rejects loopback, private, link-local, CGNAT, documentation/test, multicast, broadcast, unspecified, and IPv4-mapped private addresses.
- Rejects `localhost`, metadata hostnames, and DNS answers that resolve to a forbidden address (DNS rebinding).
- Rewrites redirects as ordinary new proxy requests; the client follows, and the proxy re-authorises every hop.
- Logs method, target, host:port, allow/deny reason, resolved address, and forwarded bytes per request.
- Does not terminate TLS or inspect CONNECT payloads; that is the PRD's accepted residual.

## Exact MXC policy

The selected Windows policy is:

```json
{
  "network": {
    "egress": { "default": "deny" },
    "ingress": { "default": "allow", "hostLoopback": "deny" }
  },
  "runtimeConfig": { "networkProxy": "http://127.0.0.1:<port>" },
  "processContainer": {
    "network": { "allowedProxyPeer": "<AppContainer profile name>" }
  }
}
```

Typed Rust:

```rust
let mut process_network = ProcessContainerNetwork::default();
process_network.allowed_proxy_peer = Some(profile_name);
let mut process_container = ProcessContainerConfig::default();
process_container.network = Some(process_network);

let network = NetworkPolicy {
    egress: Some(NetworkEgressPolicy { default: Some(NetworkAction::Deny), ..Default::default() }),
    ingress: Some(NetworkIngressPolicy {
        default: Some(NetworkAction::Allow),
        host_loopback: Some(NetworkAction::Deny),
    }),
    runtime_config: Some(NetworkRuntimeConfig {
        network_proxy: Some(format!("http://127.0.0.1:{proxy_port}")),
    }),
};
```

The identityless comparison uses `host_loopback=Allow` and no `allowedProxyPeer`; it works but is not selected. The network-denied fallback uses `egress=deny`, `ingress.default=deny`, `host_loopback=deny`, and no `runtimeConfig.networkProxy`.

## Proxy deployment: the critical finding

The strong path requires the proxy to run in its own **AppContainer** profile. The host does not need admin.

1. **Create a per-user AppContainer profile** with `CreateAppContainerProfile`.
   - Request the capabilities `internetClient`, `internetClientServer`, and `privateNetworkClientServer` at profile creation.
   - The profile name is the value passed as `processContainer.network.allowedProxyPeer`.
2. **Grant the profile SID access** to the proxy executable and its control/log directory with `icacls` (read/execute on the exe; read/write on the control directory). This is a per-user ACL operation; no elevation. The spike grants full control on its run root for fixture convenience; production should grant only the proxy executable and the control/log directory.
3. **Launch the proxy** with `CreateProcessW` and `PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES`.
   - `SECURITY_CAPABILITIES.AppContainerSid` is the profile SID.
   - `SECURITY_CAPABILITIES.Capabilities` contains the three capability SIDs.
   - **Every `SID_AND_ATTRIBUTES.Attributes` must be `SE_GROUP_ENABLED` (4), not `0`.** This is the single most important implementation detail found by the spike. With `Attributes=0`, the process still launches in the AppContainer and can bind loopback, but it has no network: DNS fails with `WSAHOST_NOT_FOUND` (11001) and direct TCP fails with `WSAEACCES` (10013). With `SE_GROUP_ENABLED`, DNS and TCP succeed. See the token-probe evidence.
4. **Start the proxy before the worker**, and keep it alive until the worker exits.
5. **Do not rely on host-to-proxy HTTP.** The host cannot connect to the AppContainer proxy (connection timeout), while the MXC client can. That is the desired loopback isolation. Use a file/pipe control channel for health, allowlist updates, and shutdown; the spike uses an allowlist file that the proxy watches.
6. `CheckNetIsolation LoopbackExempt -a -n=<profile>` failed with `Access Denied` (admin required), but it was **not needed** for the MXC client → proxy direction. Do not make Forge require admin for this path. If a future design needs host → proxy, it needs an admin firewall rule or a packaged proxy; avoid that coupling.

The negative test confirms the identity binding: a normal host process listening on the same endpoint, with the policy naming the AppContainer profile, is unreachable from the worker. Every proxy connection timed out. The peer name is not decorative.

Final selected-run counters: `client_pass=20`, `client_fail=0`, `client_inconclusive=1` (the direct-metadata probe, whose host-side positive control was unavailable). The identityless comparison and the network-denied fallback both had zero failures; the negative peer test was `pass=5`, `fail=15`, `inconclusive=1`, as expected.

## Test hosts and observed behavior

Allowed hosts in the spike: `example.com`, `www.example.com`, `httpbin.org`. Unlisted host: `example.org`. Synthetic test-only names: `redirect.test` (302 fixtures), `rebind.test` (resolves to `127.0.0.1`), `private.test` (resolves to `10.0.0.1`), `metadata.test` (resolves to `169.254.169.254`). These synthetic names are not a production allowlist.

Selected identity-scoped run:

| Check | Observed |
|---|---|
| HTTP allowed (`http://example.com/`) | 200 through the proxy |
| HTTP unlisted (`http://example.org/`) | 403, `host-not-allowlisted` |
| CONNECT allowed (`example.com:443`) | 200 Connection Established |
| CONNECT unlisted (`example.org:443`) | 403, `host-not-allowlisted` |
| Redirect allowed (`redirect.test/allowed` → `example.com`) | 200 after 1 hop |
| Redirect unlisted (`redirect.test/unlisted` → `example.org`) | 403 after 1 hop |
| Redirect private (`redirect.test/private` → `169.254.169.254`) | 403 after 1 hop |
| Proxy loopback (`127.0.0.1:<sentinel>`) | 403, private/loopback address |
| Proxy metadata (`169.254.169.254`) | 403, private/loopback address |
| Proxy DNS rebinding (`rebind.test` → `127.0.0.1`) | 403, `dns-rebinding-private-address` |
| Proxy private (`private.test` → `10.0.0.1`) | 403, `dns-rebinding-private-address` |
| Dynamic allowlist | 403 → 200 after a mid-task grant → 403 after revoke |
| Direct egress (`1.1.1.1:443`) | denied: `WSAEACCES` 10013 (host preflight reachable) |
| Direct DNS (`8.8.8.8:53`) | denied: timeout 10060 (host preflight reachable) |
| Direct loopback (host sentinel) | denied: `WSAEACCES` 10013 |
| Direct metadata (`169.254.169.254:80`) | denied: `WSAEACCES` 10013; host preflight was false, so the comparison is inconclusive |
| Injected proxy env | `HTTP_PROXY`, `HTTPS_PROXY`, `http_proxy`, `https_proxy` all point at the proxy |
| `curl.exe` HTTP | 200 through the injected proxy |
| `curl.exe` HTTPS | 200 over CONNECT |
| `curl.exe` unlisted | 403 |
| Worker secrets | none of the checked model/cloud secrets present |
| Host sentinel | unchanged; zero connections reached it |

The proxy log shows the resolved IP for every allowed request (for example `example.com` → `172.66.147.243`), the denial reason for every blocked request, and the synthetic redirect/rebinding decisions. Direct DNS to a public resolver timed out, and direct egress was denied; hostname resolution is performed by the proxy, and the worker's only reachable network endpoint is the proxy.

## Failure modes and required behavior

| Failure | Required behavior |
|---|---|
| AppContainer profile creation fails | Fail closed to `Sandboxed — network denied`; do not use the identityless proxy; do not fall back to `Guarded` solely for allowlisting. |
| Proxy launch fails or exits before listening | Fail closed to `Sandboxed — network denied`; do not start the worker with a dead proxy endpoint. |
| Proxy DNS or upstream connect fails | Return a proxy error (403/502); never fall back to direct egress. If it persists, pause/stop the task and report it through `forge doctor`. |
| Peer identity does not match | Connection times out; this is the intended enforcement. Do not weaken to `hostLoopback=allow`. |
| Mid-task host grant | Update the proxy allowlist through the control channel; the spike proves 403 → 200 after grant and 200 → 403 after revoke. If the update cannot be applied, deny the request and ask again. Do not restart the proxy on a new endpoint; the worker's proxy URL is pinned. |
| Redirect to unlisted/private host | Deny on the next hop; the proxy re-authorises every request. |
| Loopback, private, link-local, metadata, DNS rebinding | Deny before connecting. |
| Non-cooperative client ignores proxy env/WinHTTP | Direct egress is blocked by WFP; the attempt fails closed. It cannot bypass the proxy. |
| Proxy crashes mid-task | Stop the task; never continue unsandboxed. Restarting the proxy on the same endpoint is a TDD decision, but the worker must not silently bypass it. |
| Host cannot reach the proxy | Expected. Do not use host-side HTTP checks; use the process handle and the control/log channel. |
| `SE_GROUP_ENABLED` omitted | AppContainer has no network; DNS/TCP fail. Fix the capability attributes; do not fall back to a normal process. |

Residuals (accepted by the PRD): an allowlisted host carrying attacker-controlled content remains an exfiltration path; CONNECT tunnels are opaque; host exec bypasses the proxy; the proxy is part of the trusted computing base; the worker and proxy are separate processes with a control channel that must be kept simple.

## TDD-0006 inputs

- **Windows mode:** `Sandboxed` with allowlisting on Windows 11 24H2+ x64 when the peer-scoped proxy starts and the canaries pass. Otherwise `Sandboxed — network denied`. Never `Guarded` solely because domain allowlisting is unavailable.
- **Policy:** exact MXC fields above. `egress.default=deny`, `ingress.default=allow`, `ingress.hostLoopback=deny`, `runtimeConfig.networkProxy`, `allowedProxyPeer=<profile name>`.
- **Proxy identity:** per-user unpackaged AppContainer profile; capabilities `internetClient`, `internetClientServer`, `privateNetworkClientServer` with `SE_GROUP_ENABLED`; per-user ACL grants; no admin.
- **Proxy lifecycle:** create profile at init (or first use), start proxy before the worker, stop after the worker, keep the profile for reuse, delete on uninstall.
- **Control channel:** file-based allowlist updates are proven. The host cannot connect to the proxy; do not design host-side HTTP health checks. A named pipe is an untested alternative; if used, its ACL must grant the host and deny the worker.
- **Endpoint:** choose a free loopback port, bind and verify before launching the worker, pass the exact URL to MXC, and fail closed if the port is taken or the proxy is not listening.
- **Canary 3:** test both the unlisted host through the proxy (403) and direct egress (WSAEACCES/timeout). The canary is not just "the proxy returned 403"; it must prove the worker cannot bypass the proxy.
- **`forge doctor`:** verify the profile exists, the proxy process is running, the endpoint is the configured one, the allowlist loaded, the peer name matches, the capability attributes are correct, and direct egress is denied.
- **Audit:** log every allowed/denied request with host, resolved address, decision, and bytes; redact credentials and query secrets.
- **Residuals:** allowlisted-host exfiltration, opaque CONNECT, proxy code in the TCB, host exec bypass, and the absence of TLS interception.

## Open questions for TDD-0006

1. **Minimum capability set.** The spike uses `internetClient`, `internetClientServer`, and `privateNetworkClientServer`. Which are strictly required for DNS, outbound HTTP/CONNECT, and the loopback listener? Minimise and document.
2. **Port policy.** The spike allows any port on an allowlisted host (tested 443). Should the proxy restrict to 80/443 plus package-registry ports? The PRD's allowlist is host-based; a port policy is a TDD detail.
3. **Control-channel atomicity.** The spike writes the allowlist file and waits 200 ms before acknowledging. Production needs atomic replace, a generation counter, or a pipe protocol so a partial read can never briefly allow/deny the wrong set.
4. **Proxy crash/restart.** The worker's proxy URL is pinned. Decide whether a crashed proxy can be restarted on the same endpoint, or whether the task must stop.
5. **Profile lifecycle.** When to create/delete the profile, how to handle a profile left by a previous install, and how to avoid name collisions across users.
6. **24H2 validation.** Re-run on build 26100.9278+ (the tested host is 25H2 26200).
7. **Arm64.** Not tested; Windows arm64 remains best-effort.
8. **Real package installs.** The spike proves HTTP/CONNECT with `curl.exe` and a raw client; it does not run `npm`, `cargo`, or `pip` through the proxy. Registry installs should be a follow-up canary.

## Reproduction

```powershell
cd spikes\windows-proxy
cargo build --bins
.\target\debug\appcontainer-probe.exe   # token/DNS/TCP probe inside the AppContainer
.\target\debug\proxy-spike.exe          # identityless, identity-scoped, negative, network-denied
```

Exit codes are informational; the verdict is in the summary and the `SPIKE VERDICT` line. The harness uses public `example.com`/`httpbin.org` and synthetic `.test` names only; it is evidence, not production code. The branch is intentionally not merged.
