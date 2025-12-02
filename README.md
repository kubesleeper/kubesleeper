<div align="center">
<img src="./doc/src/rsc/ks.gif" width="300">
<h1>kubesleeper</h1>
<i>let your cluster take naps</i>
<p>A lightweight 'scale to zero' kubernetes manager</p>
  <!-- <a href="https://github.com/kubesleeper/kubesleeper/releases">
    <img src="https://img.shields.io/github/v/release/kubesleeper/kubesleeper?style=flat-square">
  </a> -->
  <a href="./LICENSE">
    <img src="https://img.shields.io/badge/license-MIT-violet">
  </a>
</div>

---

**Documentation**: [https://kubesleeper.com/](https://kubesleeper.com/)

---

Kubesleeper is a "scale-to-zero" Kubernetes manager. It automatically reduces resource usage based on load, helping you cut infrastructure costs. Concretely, if an application is unused for some time, kubesleeper will automatically shut it down and restart it when a new user tries to access it.

Advantages of kubesleeper:
- **Lightweight** – Runs as a single tiny pod, regardless of your cluster size.
- **No Proxy Layer** – Directly interacts with the Kubernetes API; never intercepts or modifies your resources.
- **Respects Your Scaling** – kubesleeper only turns resources on/off. Your own autoscaling rules and fine-grained logic remain untouched and active when resources are awake.
- **Fully Configurable** – Designed to adapt to your environment, whatever it looks like.
- **Safe by Design** – Performs checks before any action, avoids flapping, and never deletes or recreates resources.



<br><br>

## Who we are

<div align="center">
<table>
  <tr>
    <td align="center">
      <a href="https://github.com/eloi-menaud">
        <img src="https://github.com/eloi-menaud.png" width="50"/><br>
        <strong>Eloi M.</strong>
      </a>
    </td>
    <td align="center">
      <a href="https://github.com/jordanlv">
        <img src="https://github.com/jordanlv.png" width="50"/>
        <br>
        <b>Jordan L.</b>
      </a>
    </td>
    <td align="center">
      <a href="https://github.com/j54laurenceau">
        <img src="https://github.com/j54laurenceau.png" width="50"/>
        <br>
        <b>Julien L.</b>
      </a>
    </td>
    <td align="center">
      <a href="https://github.com/MathisCrr">
        <img src="https://github.com/MathisCrr.png" width="50"/>
        <br>
        <b>Mathis C.</b>
      </a>
    </td>
    <td align="center">
      <a href="https://github.com/QuentinSanterne">
        <img src="https://github.com/QuentinSanterne.png" width="50"/>
        <br>
        <b>Quentin S.</b>
      </a>
    </td>
  </tr>
</table>
</div>

## License

Code and documentation are under the [MIT License](LICENSE).
