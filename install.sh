

ok(){
    printf "\033[0;32m✓\033[0m %s\n" "$@"
}
err() {
    printf "\033[0;31mx\033[0m %s\n" "$*"
}

opts="$1"
if test "$opts" = "--help" || test "$opts" = "-h"
then
  printf "\n"
  printf "k3s logs        :\033[0m /tmp/k3s.log\n"
  printf "stop k3s        :\033[0m sudo pkill -9 k3s\n"
  printf "stop containerd :\033[0m sudo pkill -9 '/k3s/containerd/containerd.sock'\n"
  printf "find k3s.yaml   :\033[0m find / -name k3s.yaml\n"
  printf "set KUBECONFIG  :\033[0m export KUBECONFIG='{path to k3s.yaml}'\n"
  printf "\nMake sure to not have K3S running with systemctl"
  exit 0
fi

# check current dir
if ! test -d ".git"
then err "you should be at the root of the project"
fi


# K3S
if k3s --version > /dev/null
then ok "k3s"
else err "k3s not installed" && exit 1
fi

# kubectl
if kubectl help > /dev/null
then ok "kubectl"
else err "kubectl not installed" && exit 1
fi

# run k3s server
if test $(ps -ac | grep k3s | wc -l) -eq 0
then
    echo "start k3s server"
    sudo -v
    sudo k3s server --write-kubeconfig-mode=644 >/dev/null 2>/tmp/k3s.log &
    k3s_pid=$!
    echo "k3s starting with pid $k3s_pid ..."
    
    nb_loop=10
    sleep_by_loop=1
    for i in $(seq 1 $nb_loop)
    do
        if kubectl get nodes >/dev/null 2>&1
        then break
        else sleep $sleep_by_loop
        fi 
    done
    
    if kubectl get nodes >/dev/null 2>&1
    then ok "k3s reachable"
    else err "k3s unreachable after $((nb_loop * sleep_by_loop))s"
    fi
    

else
    ok "k3s server already running"
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
  echo "namespace '$NAMESPACE' exists, delete it"
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
then err "'$ENV' is not a available env" && exit 1
fi

kubectl apply -n $NAMESPACE -f "./envs/$ENV"
