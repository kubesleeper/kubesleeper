<br><br>
<div align="center">
<img src="./doc/rsc/logo-anim-rounded.gif" width="300">
<h1>kubesleeper</h1>
<p>A lite 'scale to zero' kubernetes manager</p>
<img src="https://img.shields.io/badge/license-MIT-violet">
</div>

### Project structure

```
.
├── doc/
│   └── *.md
├── target/
│   ├── user-doc/
│   └── tech-doc/
├── src/
│   └── *.rs
├── jobs/
│   └── *.yaml
├── .gitlab-ci.yml
├── Cargo.toml
└── Dockerfile
```

### nix-shell
```shell
nix-shell --pure
```

### V0 Infra

<img src='./doc/rsc/v0_infra_schema.png' width='600px'/>

### Sequence

<img src='./doc/rsc/seq.drawio.png' width='600px'/>

<img src='./doc/rsc/schema-example.png' width='600px'/>
