<div align="center">
    <img src="./rsc/kubesleeper-logo.svg" width="200">
    <h1>kubesleeper</h1>
    <i>Let your cluster take naps</i>
    <p>A lite 'scale to zero' kubernetes manager</p>
</div>

<br> 

Kubesleeper is a _scale-to-zero_ Kubernetes manager. It automatically reduces resource usage based on load, helping you cut infrastructure costs. Concretely, if an application is unused for some time, kubesleeper will automatically shut it down and restart it when a new user tries to access it.

Advantages of kubesleeper:
- **Lightweight** – Runs as a single tiny pod, regardless of your cluster size.
- **No Proxy Layer** – Directly interacts with the Kubernetes API; never intercepts or modifies your resources.
- **Respects Your Scaling** – kubesleeper only turns resources on/off. Your own autoscaling rules and fine-grained logic remain untouched and active when resources are awake.
- **Fully Configurable** – Designed to adapt to your environment, whatever it looks like.


## Distributions

- [images](https://github.com/kubesleeper/kubesleeper/pkgs/container/kubesleeper)
- [releases](https://github.com/kubesleeper/kubesleeper/releases)

