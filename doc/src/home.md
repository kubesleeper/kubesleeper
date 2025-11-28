<div align="center">
    <img src="./rsc/kubesleeper-logo.svg" width="200">
    <h1>kubesleeper</h1>
    <i>Let your cluster take naps</i>
    <p>A lite 'scale to zero' kubernetes manager</p>
</div>

<br> 

Kubesleeper is a _scale to zero_ kubernetes manager. It automatically decrease resources regarding their load, offering you less expenses.  

Advantage off kubesleeper :
* **Very Lightweight :** Only one very small pod to manage any cluster size 
* **No Proxy :** Use the k8s api, does not interfere with your Kubernetes resources
* **Respect Your Scaling :** Only turn _off_ or _on_ your resources and do not alter your own scaling logic. When _on_ your finer-grained scaling management rules are retained.
 