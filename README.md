# keen

**Keen** is a universal linter and runner that doesn't stick to the boring slow stuff. it's fast, direct, and just works!

## what does it have?
- **universal linting:** `keen <file>` or `keen -c <file>`
- built-in support for json, and wrapped support for c, c++, go, rust, etc.
- **smart execution:** `keen -o <file>`
- **snippet mode:** runs single files in a temporary sandbox.
- **project mode:** automatically detects Cargo.toml, package.json, go.mod, etc. and runs the whole project.
- **zero configuration:** no .keenrc or any unnecessary BS needed. it just knows and does what its good at.
- **shell integration:** `keen --install` to add it to your `$PATH`!

## installation

**the fast way:**
```bash
curl -sSL https://raw.githubusercontent.com/hnpf/keen/main/scripts/install.sh | bash
```

**the manual way:**
```bash
cargo build --release
./target/release/keen --install
```
- that's it!

## usage
```bash
keen <file>:           ## check syntax!
keen -o <file>:        ## run file
keen -c -o <file>:     ## lint and run
keen -P -o <file>:     ## force project mode and run
```
**!note!** the `--proceed` flag is a work-in-progress and will handle more complex build workflows in the future.

## project structure

```
src/
├── main.rs       entry point and command orchestration
├── args.rs       CLI argument definitions (clap)
├── check.rs      syntax checking for various languages
├── run.rs        running snippets and projects
├── install.rs    installation and shell integration
└── utils.rs      shared helpers (fs walking, path checks)
```

## license

licensed under GPL-3.0.
