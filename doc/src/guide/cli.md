# Kubesleeper CLI

## Global Options

Global options must be added at the beginning of the command

`kubesleeper <global options>`

These flags are universal and apply to all CLI actions and subcommands.

- `-v` : verbose mode for logging
- `-r | --readable-log` : Human readable mode for logging 
- `--config <CONFIG>` : Path to the kubesleeper YAML configuration file


---

## start
`kubesleeper start`

This is the primary function of the CLI and serves as the entrypoint for the Kubesleeper image.

Running this command initiates the nominal behavior and launches the main [Kubesleeper process](./how_it_works.html).

---

## status
`kubesleeper status`

Performs a comprehensive scan and validation of your cluster. It generates a report showing exactly how Kubesleeper perceives your environment.

This is a vital tool for ensuring your configuration meets your needs—for example, verifying that specific resources are being correctly ignored.

---

## msg
`kubesleeper msg <COMMAND>`

Used for advanced manual actions. This command is particularly useful for debugging and performing granular interventions within the cluster.

### dump-config
`kubesleeper msg dump-config`

Dump the computed configuration

### set
`kubesleeper msg set-rsc <RESOURCE_TYPE> <NAMESPACE/NAME> <STATE>`

Set namespace to the desired state

- `STATE` : The target state to which the cluster will be set [possible values: asleep, awake]

### set-rsc
`kubesleeper msg set-rsc <RESOURCE_TYPE> <NAMESPACE/NAME> <STATE>`

Set a specific Deployment or Service to the desired state

- `RESOURCE_TYPE`: the kubernetes shortname of resource [possible values: svc, deploy]
- `NAMESPACE/NAME`: the kube resournce id like {namespace}/{name}, namespace 'default' will be used if id is simply {name}
- `STATE`: The target state to which the resource will be set [possible values: asleep, awake]

### start-server
`kubesleeper msg start-server`

Start web server alone (without kubernetes resource management)
