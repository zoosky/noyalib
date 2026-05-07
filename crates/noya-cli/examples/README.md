<!-- SPDX-License-Identifier: Apache-2.0 OR MIT -->

# `noya-cli` examples

End-to-end demos for the `noyafmt` and `noyavalidate` binaries.
Each example is a self-contained shell script — run it directly,
no project setup needed (assumes the binaries are on `$PATH`,
which `cargo install noya-cli` arranges).

| Script | What it shows |
|---|---|
| [`format-precommit.sh`](format-precommit.sh) | Drop-in `git pre-commit` hook gating commits on `noyafmt --check`. |
| [`validate-k8s.sh`](validate-k8s.sh) | CI step that runs `noyavalidate --schema` over a directory of Kubernetes manifests. |
| [`fix-quoted-numbers.sh`](fix-quoted-numbers.sh) | Walkthrough of the `--fix` autofix flow: quoted scalar → schema-typed integer, with the surrounding comment preserved. |

```bash
# All examples are independent — pick one and run it.
chmod +x crates/noya-cli/examples/*.sh
crates/noya-cli/examples/fix-quoted-numbers.sh
```

## License

Dual-licensed under Apache 2.0 or MIT, at your option.
