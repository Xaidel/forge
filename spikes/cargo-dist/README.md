# cargo-dist release spike

Throwaway fixture for [Spike cargo-dist packaging and release artifacts](https://github.com/Xaidel/forge/issues/25).
It exists only to prove the release pipeline and produce [EVIDENCE.md](EVIDENCE.md); it is not
production code and this branch is never merged.

The fixture is a single `forge-harness` package whose only binary is named `forge`, matching the
distribution contract in `docs/PRD.md` NFR-DIST-001. It deliberately carries no Forge domain code.

```text
fixture/
  Cargo.toml             # package forge-harness, bin forge, profile.dist
  dist-workspace.toml    # cargo-dist 0.33.0 config under test
  src/main.rs            # only --version and a placeholder line
```

Run it locally with cargo-dist 0.33.0:

```sh
cd fixture
dist plan --tag=v0.1.0
dist build --tag=v0.1.0 --artifacts=local --target=x86_64-pc-windows-msvc
dist build --tag=v0.1.0 --artifacts=global
```
