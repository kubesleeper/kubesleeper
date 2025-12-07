# Kubesleeper configuration

Main configuration of kubesleeper should be set in a `./kubesleeper.yaml` file.

See [CLI parameters](/guide/cli#config-path) to set a specific path.

> [!NOTE] 
> All values listed on this page represent the system's default configurations.

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
