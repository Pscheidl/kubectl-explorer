# KubExplorer
[![Rust build & tests](https://github.com/Pscheidl/kubexplorer/actions/workflows/rust.yml/badge.svg)](https://github.com/Pscheidl/kubexplorer/actions/workflows/rust.yml)

**Warning:** Proof of concept. Feedback is much welcome.

Discovers and prints out any `Configmaps` and `Secrets` not linked to any of the following resources:
1. Deployments,
1. ReplicaSets,
1. StatefulSets,
1. DaemonSets,
1. Jobs,
1. CronJobs,
1. ReplicationControllers,
1. Pods.

## Running

1. [Install Rust](https://www.rust-lang.org/learn/get-started)
1. Simply invoke `cargo run` (add the `--release` flag for optimal performance) with the `KUBECONFIG` environment variable set. 
   

The tool will detect orphans in the `KUBECONFIG`'s default namespace.