# Kubectl explorer plugin - orphan detection
[![Rust build & tests](https://github.com/Pscheidl/kubexplorer/actions/workflows/rust.yml/badge.svg)](https://github.com/Pscheidl/kubexplorer/actions/workflows/rust.yml)

A kubectl plugin to explore **orphan** `configmaps` and `secrets`. Can be used standalone. 

New issues or any kind of feedback are most welcome. 

## Orphan detection

Discovers and prints out any `Configmaps` and `Secrets` not linked to any of the following resources:
1. Deployments,
2. ReplicaSets,
3. StatefulSets,
4. DaemonSets,
5. Jobs,
6. CronJobs,
7. ReplicationControllers,
8. Pods,
9. Ingresses,
10. ServiceAccounts.

## Usage

The [recommended](#kubectl-plugin) way is to use `kubectl-explore` as a `kubectl` plugin. Alternatively, because every [kubectl plugin](https://kubernetes.io/docs/tasks/extend-kubectl/kubectl-plugins/)
is a standalone binary, it can be used [directly](#standalone).

### Kubectl plugin

1. Download a [pre-compiled](https://github.com/Pscheidl/kubexplorer/releases) binary, or compile [from the source](#optional-compiling-and-running-from-source).
2. Add kubectl-explore to path, e.g. to add temporarily use `export PATH="/path/to/kubectl-explore:$PATH"`.
3. Invoke `kubectl plugin list` - kubectl-explore should be on the list, automatically discovered by `kubectl`.

To see all configuration options, invoke `kubectl-explore orphans -h`.
Default context and namespace found in `$KUBECONFIG` is used, unless specified otherwise.

```shell
kubectl explore orphans
```

```
Searching for unused ConfigMaps and Secrets in the 'default' namespace
configmaps:
- test
secrets: []
```

### Standalone
Pre-compiled `x86_64-unknown-linux-gnu` binaries are available. Compiling and running ["from the source"](#optional-compiling-and-running-from-source) is also an option.

```shell
> kubectl-explore orphans -h
```

```
Options:
  -k, --kubeconfig <PATH_TO_KUBECONFIG>
          Path to a KUBECONFIG file. When not set, env is used.
  -n, --namespace <NAMESPACE>
          Namespace to search in.
  -o, --output <OUTPUT>
          Output format. YAML by default. [default: yaml] [possible values: yaml, json]
  -h, --help
          Print help
```

E.g. `kubectl-explore -k /etc/rancher/k3s/k3s.yaml -n default -o json` to explicitly specify the `KUBECONFIG` and the namespace.
If `KUBECONFIG` is not specified, the `KUBECONFIG` env variable is looked for. When not found, an error is thrown.
If `namespace` is not defined, the default namespace from `KUBECONFIG` is used.

For a list of all commands and general help, invoke `kubectl-explore -h`.



#### (Optional) compiling and running from source
1. [Install Rust](https://www.rust-lang.org/learn/get-started)
1. Simply invoke `cargo run -- -h` (add the `--release` flag for optimal performance) to obtain instructions.

`> cargo run -- -h`

## Testing

Run tests using `cargo test`. Tests require:

1. Running Kubernetes cluster with supported API version `1_26`,
1. `KUBECONFIG` environment variable set.

An easy way to obtain a Kubernetes cluster is [k3s.io](https://k3s.io/) - curl -sfL https://get.k3s.io | sh -. After
installation, `export KUBECONFIG=/etc/rancher/k3s/k3s.yaml` and make sure to `chown` or `chmod` the `$KUBECONFIG` file
for current user to be able to read it.
