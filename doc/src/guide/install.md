# Install

Kubesleeper is essentially a deploy that manages your resources via the Kubernetes API.

To install it, you need to deploy a Kubesleeper deployment. This can be done using Helm (recommended) or manually.

## Helm Install
_Work in progress… Coming soon_

## Manual install

>[!NOTE]
>It is strongly recommended to use the official Helm Chart for deploying Kubesleeper.
> This ensures that all required configurations and security contexts are automatically applied.

### Requirements
If you prefer to create your own Kubernetes Deployment manifest, certain fields are mandatory for Kubesleeper to function correctly.
Please ensure your manifest includes the following specifications:

```yaml
kind: Deployment
metadata:
  name: kubesleeper
  labels:
    app: kubesleeper
spec:
  replicas: 1
  selector:
    matchLabels:
      app: kubesleeper
  template:
    metadata:
      labels:
        app: kubesleeper
```

### Example of full configurations

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kubesleeper
  labels:
    app: kubesleeper
spec:
  replicas: 1
  selector:
    matchLabels:
      app: kubesleeper
  template:
    metadata:
      labels:
        app: kubesleeper
    spec:
      containers:
        - image: ghcr.io/kubesleeper/kubesleeper:latest
          name: Kubesleeper
          ports:
          - containerPort: 8000
```

### Deploy
Simply deploy your manifest with for example : `kubectl apply <path to your .yaml>`
