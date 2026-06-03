## Contributing

Accept [Code of Conduct](./CODE_OF_CONDUCT.md).
Ask questions and suggest improvements to the Code of Conduct.
Do not participate if you do not accept the terms of the Code of Conduct.

Fork, then clone your new repo.<br>
Replace the **\< >** placeholders with your information:

```sh
git clone https://tangled.org/<your-name.homeserver.xyz>/coven
```

Set ORIGIN for changes to your fork as:

```sh
git@tangled.org:did:plc:<your-plc-identifier-here>/coven
```

Validate inputs and external state early. Extract common logic, prefer simple solutions. Clear error messages.

Include this header in every rust file

// SPDX-License-Identifier: MPL-2.0 AND LicenseRef-Commons-Clause-License-Condition-1.0
// <!-- // /*  d a r k s h a p e s */ -->

Rust Tools:

```
cargo test
cargo build
cargo clippy
cargo xwin clippy       // For linux to windows compatibility
cargo update --precise` // make lockfile changes.
```

Python Tools:

```
pyrefly
ruff
pytest -rPvv            // Verbose, all pass and fail output
```

## Roadmap

- migrate negate dataset loading to nocturne
- shut down inference for training
- headless, gguf, port cli options
- begin work on tahoe-lafs file store
- swappable pytorch
- package the project
- diffusion splitting
- glaze share
- checkpoint specific saving
- container/vm options (smolvm)
