# Helm

This Helm chart deploys kubesleeper to your Kubernetes cluster.
Official Chart packages (.tgz) are published directly under the GitHub Releases of the repository.

## Values
|Parameter|Description|Default|
|-|-|-|
|image.name|Container image repository|`ghcr.io/kubesleeper/kubesleeper:0.1.0`|
|image.tag|Container image tag|Inherits `appVersion` defined in Chart.yaml|
|config|kubesleeper YAML configuration|`{}` use default configuration|
