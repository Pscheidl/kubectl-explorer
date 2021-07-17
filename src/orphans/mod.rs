use std::collections::HashSet;
use std::sync::RwLock;

use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::batch::v1beta1::CronJob;
use k8s_openapi::api::core::v1::{Pod, PodSpec, ReplicationController};
use kube::Client;
use rayon::prelude::*;

use crate::pod_spec::ResourceWithPodSpec;
use crate::resources::list_resource;

pub async fn find_orphans<'a>(
    secrets: HashSet<&'a str>,
    cfgmaps: HashSet<&'a str>,
    client: &Client,
    namespace: &str,
) -> Orphans<'a> {
    let mut secrets_orphans = secrets.clone();
    let mut cfgmaps_orphans = cfgmaps.clone();

    // All resources containing `PodSpec` must be inspected, as those may be scaled down, therefore
    // inspecting only `Pods` wouldn't suffice.
    let deployments_fut = list_resource::<Deployment>(client, namespace);
    let replicasets_fut = list_resource::<ReplicaSet>(client, namespace);
    let statefulsets_fut = list_resource::<StatefulSet>(client, namespace);
    let daemonsets_fut = list_resource::<DaemonSet>(client, namespace);
    let jobs_fut = list_resource::<Job>(client, namespace);
    let cronjobs_fut = list_resource::<CronJob>(client, namespace);
    let replication_controllers_fut = list_resource::<ReplicationController>(client, namespace);
    let pods_fut = list_resource::<Pod>(client, namespace);

    // Kubernetes API Denial Of Service attack :)
    let (
        deployments_res,
        replicasets_res,
        statefulsets_res,
        daemonsets_res,
        jobs_res,
        cronjobs_res,
        replication_controllers_res,
        pods_res,
    ) = tokio::join!(
        deployments_fut,
        replicasets_fut,
        statefulsets_fut,
        daemonsets_fut,
        jobs_fut,
        cronjobs_fut,
        replication_controllers_fut,
        pods_fut
    );

    let mut pod_specs: Vec<&PodSpec> = Vec::new();

    let deployments = deployments_res.unwrap();
    extend_with(&mut pod_specs, &deployments);
    let replicasets = replicasets_res.unwrap();
    extend_with(&mut pod_specs, &replicasets);
    let statefulsets = statefulsets_res.unwrap();
    extend_with(&mut pod_specs, &statefulsets);
    let daemonsets = daemonsets_res.unwrap();
    extend_with(&mut pod_specs, &daemonsets);
    let jobs = jobs_res.unwrap();
    extend_with(&mut pod_specs, &jobs);
    let cronjobs = cronjobs_res.unwrap();
    extend_with(&mut pod_specs, &cronjobs);
    let replication_controllers = replication_controllers_res.unwrap();
    extend_with(&mut pod_specs, &replication_controllers);
    let pods = pods_res.unwrap();
    extend_with(&mut pod_specs, &pods);

    let locked_secret_orphans = RwLock::new(&mut secrets_orphans);
    let locked_configmap_orphans = RwLock::new(&mut cfgmaps_orphans);
    pod_specs.par_iter().for_each(|pod_spec| {
        pod_spec
            .containers
            .iter()
            .flat_map(|container| &container.env_from)
            .for_each(|env_from| {
                if let Some(cfgmap) = env_from.config_map_ref.as_ref() {
                    let mut lock = locked_configmap_orphans.write().unwrap();
                    lock.remove(cfgmap.name.as_ref().unwrap().as_str());
                }

                if let Some(secret) = env_from.secret_ref.as_ref() {
                    let mut lock = locked_secret_orphans.write().unwrap();
                    lock.remove(secret.name.as_ref().unwrap().as_str());
                }
            });

        pod_spec.volumes.iter().for_each(|volume| {
            if let Some(cfgmap) = volume.config_map.as_ref() {
                let mut lock = locked_configmap_orphans.write().unwrap();
                lock.remove(cfgmap.name.as_ref().unwrap().as_str());
            }

            if let Some(secret) = volume.secret.as_ref() {
                let mut lock = locked_secret_orphans.write().unwrap();
                lock.remove(secret.secret_name.as_ref().unwrap().as_str());
            }
        });
    });

    Orphans::new(secrets_orphans, cfgmaps_orphans)
}

pub fn extend_with<'a, T: ResourceWithPodSpec>(
    pod_specs: &mut Vec<&'a PodSpec>,
    extensions: &'a [T],
) where
    T: ResourceWithPodSpec,
{
    let mut ext_pod_specs: Vec<&'a PodSpec> = extensions
        .iter()
        .filter_map(|e| e.pod_template_spec())
        .collect();
    pod_specs.append(&mut ext_pod_specs);
}

pub struct Orphans<'a> {
    pub secrets: HashSet<&'a str>,
    pub cfgmaps: HashSet<&'a str>,
}

impl<'a> Orphans<'a> {
    pub fn new(secrets: HashSet<&'a str>, cfgmaps: HashSet<&'a str>) -> Self {
        Orphans { secrets, cfgmaps }
    }
}
