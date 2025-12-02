# Contribute

## Workflow
Contributions are welcome! Whether it’s a bug report, a new feature, an improvement to the documentation, or feedback about the project, we’d love to hear from you.

1. **Fork** the repository  
2. **Create a new branch** (`git checkout -b my-branch-name`)  
3. **Commit** your changes with clear messages  
4. **Open a Pull Request** explaining your work

Please make sure your contribution follows our coding style and includes tests when relevant.
If you’re unsure about anything, feel free to open an issue — we’re happy to help!

## Dev hint

### Dev namespace
Use `sh isntall.sh` to deploy a fresh dev env on the `ks` namespace.

This script uses k3s as local kube engine, it will automatically check your configuration,
start k3s, clean your `ks` dev namespace and reinstall the one you chose.
So it will serve you a brand new fresh dev namespace.


### Documentation

Use `(cd doc && mdbook serve --open)` to run a local server serving the documentation.

The documentation auto rebuilt/serve itself each time a modificqation is made

### Test locally
Use `cargo run -- {kubesleeper args}` to run kubesleeper action from your source.

Check `cargo run -- manual --help` to see the manual action you can do (useful for debug/test works)

> See [Dev namespace](#dev-namespace) to install an already pre-configured namespace

### build source
Use `cargo build --release` to build kubesleeper release.

### build kubesleeper image
use `{ docker | podman } build -t kubesleeper .` (regarding you image builder tool) to build kubesleeper release image.
