# Kubesleeper configuration

Main configuration of kubesleeper should be set in a `./kubesleeper.yaml` file.

See [CLI parameters](/guide/cli#config-path) to set a specific path.

## Resources management

### Groups
A group should be defined with a name, deploys and services

```yaml
groups:
  <group name>:
    
    services:
      - <list of services>
    deployments:
      - <list of deployments>
```

The customisation off how kubesleeper manage your resources is defined with Groups (`.groups`).

#### Group State
The state of a group determis all the resources (services and deployments) states. The same service canno't be monitored by multiple groups.

#### `groups.services`
The `services` list contains all the services that are monitored to determine the group's state :
- if **ALL** the services listed don't receive activitys for enough time, the group will be set `Asleep`
- if **AT LEAST ONE** service receive activity the group will be set `Awake`

Services are identified with :
- `<namespace>/<service name>` : a specific service
- `<namespace>/*` : meaning "all the services of the namespace <namespace>"

#### `groups.deployments`
The `deplyments` list all the deployments that are part of the group. The same deployment canno't be monitored by multiple groups.

Deployments are identified with :
- `<namespace>/<deployment name>` : a specific deployment
- `<namespace>/*` : meaning "all the deployments of the namespace <namespace>"



## System configuration

### Server

The Kubesleeper server manages two main functions: serving the waiting page to users and fetching incoming network traffic.

#### Port
The port of the kubesleeper server.

```yaml
server:
    port: 8000
```

### Controller

The Kubesleeper controller manages the lifecycle of applications.

#### _Sleepiness_ duration
Inactivity duration (in seconds) before entering [_Asleep_ state](/guide/how_it_works.html#step-3-asleep-state---scaling-down). 
> [!NOTE]
> See _[How it works](/guide/how_it_works.html#how-it-works)_ to have better understanding of _Sleepiness_.

```yaml
controller:
    sleepiness_duration: 15
```

#### Refresh interval
The time interval (in seconds) between two checks of traffic activity.

```yaml
controller:
    refresh_interval: 5
```

---


## Default configuration

```yaml
server:
  port: 10
controller:
  sleepiness_duration: 15
  refresh_interval: 5
```
