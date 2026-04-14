# Kubesleeper configuration

Main configuration of kubesleeper should be set in a `./kubesleeper.yaml` file.

See [CLI parameters](/guide/cli#config-path) to set a specific path.



## Groups
A Group is the fundamental unit of management in Kubesleeper.
It maps network activity (Services) to resource scaling (Deployments).

```yaml
groups:
  <group name>:
    
    services:
      - <namespace>/<service_name>
      - <namespace>/*
    deployments:
      - <namespace>/<deployment_name>
      - <namespace>/*
```

The **state** of a group dictates the scale of all associated resources.
- **Asleep**: When **ALL** group's services report no activity for a defined period, the group sleeps meaning that associated deployments and services are set to sleep state too.
- **Awake**: As soon as **AT LEAST ONE** group's service receives activity, the group wakes up meaning that associated deployments and services are set to awake state too.
    
### services
Services are monitored to determine if the group should stay awake.
- Specific: namespace/service_name
- Wildcard: namespace/* (Includes all services within the namespace).

> [!WARNING]
> Exclusivity: A same service cannot be monitored by multiple groups.

### deployments

The deployments list defines which resources Kubesleeper will scale up or down based on the group's state.
- Specific: namespace/deployment_name
- Wildcard: namespace/* (Includes all deployments within the namespace).

> [!WARNING]
> Exclusivity: A same deployment cannot be monitored by multiple groups.




## Server

The Kubesleeper server manages two main functions: serving the waiting page to users and fetching incoming network traffic.

### Port
The port of the kubesleeper server.

```yaml
server:
    port: 8000
```




## Controller

The Kubesleeper controller manages the lifecycle of applications.

### _Sleepiness_ duration
Inactivity duration (in seconds) before entering [_Asleep_ state](/guide/how_it_works.html#step-3-asleep-state---scaling-down). 
> [!NOTE]
> See _[How it works](/guide/how_it_works.html#how-it-works)_ to have better understanding of _Sleepiness_.

```yaml
controller:
    sleepiness_duration: 15
```

### Refresh interval
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
