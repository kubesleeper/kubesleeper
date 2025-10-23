

ok(){
    printf "\033[0;32m✓\033[0m %s\n" "$@"
}
err() {
    printf "\033[0;31mx\033[0m %s\n" "$*"
    exit 1
}

opts="$1"
if test "$opts" = "--help" || test "$opts" = "-h"
then
  printf "\n"
  printf "\033[90mk3s logs        :\033[0m /tmp/k3s.log\n"
  printf "\033[90mstop k3s        :\033[0m sudo pkill k3s\n"
  printf "\033[90mstop containerd :\033[0m sudo pkill -f '/k3s/containerd/containerd.sock'\n"
  printf "\033[90mfind k3s.yaml   :\033[0m find / -name k3s.yaml\n"
  printf "\033[90mset KUBECONFIG  :\033[0m export KUBECONFIG='{path to k3s.yaml}'\n"
  exit 0
fi

# check current dir
if ! test -d ".git"
then err "you should be at the root of the project"
fi


# K3S
if k3s --version > /dev/null
then ok "k3s"
else err "k3s not installed"
fi

# kubectl
if kubectl help > /dev/null
then ok "kubectl"
else err "kubectl not installed"
fi

# run k3s server
if test $(ps -ac | grep k3s | wc -l) -eq 0
then
    printf "start k3s server\n"
    sudo -v
    sudo k3s server --write-kubeconfig-mode=644 >/dev/null 2>/tmp/k3s.log &
    printf "k3s starting, sleeping 8s to let it start\n"
    sleep 10
else
    ok "k3s server already running"
fi

if kubectl get ns </dev/null
then ok "k3s server reachable"
else err "k3s unreachable"
fi


# check KUBECONFIG
if test -z $KUBECONFIG
then
  err "KUBECONFIG env var is empty"
else ok "KUBECONFIG env var"
fi

# clear namespace
NAMESPACE="ks"
if kubectl get namespaces $NAMESPACE >/dev/null 2>/dev/null
then
  printf "namespace '$NAMESPACE' exists, delete it\n"
  kubectl delete namespace $NAMESPACE >/dev/null
fi

if kubectl create namespace $NAMESPACE >/dev/null
then
    ok "'$NAMESPACE' namespace created"
else err "failed to clear '$NAMESPACE' namespace"
fi

# chose env
printf "\nAvailable envs :\n  none\n"
available_envs=$(find envs/ -type d | sed 1d | sed 's#envs/##')
printf "%s" $available_envs | sed "s/^/  /"
printf "\nWich env to install : "
read ENV
printf "\n"

if test -z ENV || test $ENV = "none" 
then echo "Don't install env" && exit 0
fi

if ! echo "$available_envs" | grep -xq "$ENV"
then err "'$ENV' is not a available env"
fi

kubectl apply -n $NAMESPACE -f ./envs/$ENV
