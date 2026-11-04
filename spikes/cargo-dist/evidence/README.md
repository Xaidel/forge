# Evidence files

Text extracts from the cargo-dist spike runs. The binary archives and MSIs are not committed; they
are reproducible from the fixture and are attached to the CI runs.

- `plan.txt` / `plan.json`: dist plan --tag=v0.1.0 with cargo-dist 0.33.0.
- `build-*.txt`: dist build output for each target.
- `version-*.txt`: orge --version from the unpacked build.
- `checksum-*.txt`: archive SHA-256 verification.
- `signatures-win-x64.txt`: Authenticode status for the Windows x64 exe and MSI.
- `msi-install-path.txt` / `msi-version.txt`: where the MSI installed and what it ran.
- `cargo-install-windows.txt`: local cargo install --path proof that the package installs a
  binary named `forge`.
