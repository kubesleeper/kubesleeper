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
Some kubernetese environment are created under `envs/` for tests.


### Documentation

Use `(cd doc && mdbook serve --open)` to run a local server serving the documentation.

The documentation auto rebuilt/serve itself each time a modificqation is made

### Test locally
Use `cargo run -- {kubesleeper args}` to run kubesleeper action from your source.

Check `cargo run -- --help` to see the action you can do.


### build source
Use `cargo build --release --target x86_64-unknown-linux-musl` to build kubesleeper release.

or

`{ docker|podman } build --target binary-export --output type=local,dest=./dist .`

To build with docker to have a working env (and to be iso with production) 

### build kubesleeper image
use `{ docker | podman } build -t kubesleeper` (regarding you image builder tool) to build kubesleeper release image.
