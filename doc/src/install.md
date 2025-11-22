# Install

Kubesleeper is essentially a pod that manages your resources via the Kubernetes API.

To install it, you need to deploy a Kubesleeper deployment. This can be done using Helm (recommended) or manually.

Kubesleeper only operates within the namespace where it is installed.
Therefore, ensure you install it in the correct namespace and that you deploy a Kubesleeper instance in every namespace you intend to manage.

# Helm Install
{explanation}
{link to values.yaml doc}

# Manual install
{explanation of minimal requirement to create own deployment}
