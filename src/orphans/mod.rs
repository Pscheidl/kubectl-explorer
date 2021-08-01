use std::collections::HashSet;
use std::sync::Mutex;

use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::batch::v1beta1::CronJob;
use k8s_openapi::api::core::v1::{ConfigMap, Pod, PodSpec, ReplicationController, Secret};
use kube::Client;
use rayon::prelude::*;
use serde::Serialize;

use crate::pod_spec::ResourceWithPodSpec;
use crate::resources::list_resource;
use k8s_openapi::api::networking::v1::Ingress;

pub async fn find_orphans(client: &Client, namespace: &str) -> Orphans {
    let configmaps_fut = list_resource::<ConfigMap>(client, namespace);
    let secrets_fut = list_resource::<Secret>(client, namespace);
    let (cfgmaps_res, secrets_res) = tokio::join!(configmaps_fut, secrets_fut);

    let cfgmaps = cfgmaps_res.unwrap();
    let mut cfgmaps_orphans: HashSet<String> = cfgmaps
        .into_iter()
        .filter_map(|r| r.metadata.name)
        .collect();

    let secrets = secrets_res.unwrap();
    let mut secrets_orphans: HashSet<String> = secrets
        .into_iter()
        .filter_map(|r| r.metadata.name)
        .collect();

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
    let ingresses_fut = list_resource::<Ingress>(client, namespace);

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
        ingresses,
    ) = tokio::join!(
        deployments_fut,
        replicasets_fut,
        statefulsets_fut,
        daemonsets_fut,
        jobs_fut,
        cronjobs_fut,
        replication_controllers_fut,
        pods_fut,
        ingresses_fut
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

    let locked_secret_orphans = Mutex::new(&mut secrets_orphans);
    let locked_configmap_orphans = Mutex::new(&mut cfgmaps_orphans);
    pod_specs.par_iter().for_each(|pod_spec| {
        pod_spec
            .containers
            .iter()
            .flat_map(|container| &container.env_from)
            .for_each(|env_from| {
                if let Some(cfgmap) = env_from.config_map_ref.as_ref() {
                    let mut locked_cfg_maps = locked_configmap_orphans.lock().unwrap();
                    locked_cfg_maps.remove(cfgmap.name.as_ref().unwrap());
                }

                if let Some(secret) = env_from.secret_ref.as_ref() {
                    let mut locked_secrets = locked_secret_orphans.lock().unwrap();
                    locked_secrets.remove(secret.name.as_ref().unwrap());
                }
            });

        pod_spec.volumes.iter().for_each(|volume| {
            if let Some(cfgmap) = volume.config_map.as_ref() {
                let mut locked_cfgmaps = locked_configmap_orphans.lock().unwrap();
                locked_cfgmaps.remove(cfgmap.name.as_ref().unwrap());
            }

            if let Some(secret) = volume.secret.as_ref() {
                let mut lock_secrets = locked_secret_orphans.lock().unwrap();
                lock_secrets.remove(secret.secret_name.as_ref().unwrap());
            }
        });
    });

    ingresses
        .unwrap()
        .iter()
        .flat_map(|ingress| &ingress.spec.as_ref().unwrap().tls)
        .filter_map(|tls| tls.secret_name.as_ref())
        .for_each(|secret| {
            secrets_orphans.remove(secret.as_str());
        });

    Orphans::new(cfgmaps_orphans, secrets_orphans)
}

pub fn extend_with<'a, T: ResourceWithPodSpec>(
    pod_specs: &mut Vec<&'a PodSpec>,
    extensions: &'a [T],
) where
    T: ResourceWithPodSpec,
{
    let ext_pod_specs = extensions.iter().filter_map(|e| e.pod_template_spec());
    pod_specs.extend(ext_pod_specs);
}

#[derive(Serialize)]
pub struct Orphans {
    pub configmaps: HashSet<String>,
    pub secrets: HashSet<String>,
}

impl Orphans {
    pub fn new(configmaps: HashSet<String>, secrets: HashSet<String>) -> Self {
        Orphans {
            configmaps,
            secrets,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::orphans::find_orphans;
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
    use k8s_openapi::api::core::v1::{
        ConfigMap, ConfigMapEnvSource, Container, EnvFromSource, PodSpec, PodTemplateSpec, Secret,
        SecretEnvSource,
    };
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
    use k8s_openapi::serde::__private::TryFrom;
    use k8s_openapi::ByteString;
    use kube::api::{DeleteParams, ObjectMeta, PostParams};
    use kube::{Api, Client, Config, ResourceExt};
    use std::array::IntoIter;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    #[tokio::test]
    async fn cfgmap_secret_referenced_by_deployment() {
        let config = Config::infer()
            .await
            .expect("KUBECONFIG env var not set or invalid path/content.");
        let client = Client::try_from(config.clone()).expect("Kubernetes cluster unreachable.");

        // Create configmap in Kubernetes cluster
        let mut config_data: BTreeMap<String, String> = BTreeMap::new();
        config_data.insert("test_key".to_owned(), "test_value".to_owned());
        let cfgmap = ConfigMap {
            metadata: ObjectMeta {
                name: Some("configmap".to_string()),
                ..ObjectMeta::default()
            },
            data: config_data.clone(),
            ..ConfigMap::default()
        };

        let cfgmap_api = Api::<ConfigMap>::namespaced(client.clone(), &config.default_namespace);
        cfgmap_api
            .create(&PostParams::default(), &cfgmap)
            .await
            .expect("Configmap not created.");

        // Create secretg in the Kubernetes cluster
        let mut secret_data: BTreeMap<String, ByteString> = BTreeMap::new(); // Used both for cfgmap and secret
        secret_data.insert(
            "test_key".to_owned(),
            ByteString(base64::encode("test_value").into_bytes()),
        );
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("secret".to_string()),
                ..ObjectMeta::default()
            },
            data: secret_data.clone(),
            ..Secret::default()
        };
        let secret_api = Api::<Secret>::namespaced(client.clone(), &config.default_namespace);
        secret_api
            .create(&PostParams::default(), &secret)
            .await
            .expect("Secret not created.");

        // Create a deployment linking to both `ConfigMap` and the `Secret`
        let deployment_w_pod_spec = Deployment {
            metadata: ObjectMeta {
                name: Some("deployment".to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: BTreeMap::<String, String>::from_iter(IntoIter::new([(
                            "app".to_string(),
                            "deployment".to_string(),
                        )])),
                        ..ObjectMeta::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "nginx".to_string(),
                            image: Some("alpine:latest".to_string()),
                            command: vec!["sleep".to_string()],
                            args: vec!["infinity".to_string()],
                            env_from: vec![
                                EnvFromSource {
                                    config_map_ref: Some(ConfigMapEnvSource {
                                        name: Some(cfgmap.name()),
                                        ..ConfigMapEnvSource::default()
                                    }),
                                    ..EnvFromSource::default()
                                },
                                EnvFromSource {
                                    secret_ref: Some(SecretEnvSource {
                                        name: Some(secret.name()),
                                        ..SecretEnvSource::default()
                                    }),
                                    ..EnvFromSource::default()
                                },
                            ],
                            ..Container::default()
                        }],
                        ..PodSpec::default()
                    }),
                    ..PodTemplateSpec::default()
                },
                selector: LabelSelector {
                    match_labels: BTreeMap::<String, String>::from_iter(IntoIter::new([(
                        "app".to_string(),
                        "deployment".to_string(),
                    )])),
                    ..LabelSelector::default()
                },
                ..DeploymentSpec::default()
            }),
            ..Deployment::default()
        };

        let dep_api = Api::<Deployment>::namespaced(client.clone(), &config.default_namespace);
        dep_api
            .create(&PostParams::default(), &deployment_w_pod_spec)
            .await
            .expect("Deployment resources not created in Kubernetes cluster.");

        let cfgmap_name = cfgmap.name();
        let secret_name = secret.name();
        // Both the ConfigMap and the Secret should not be detected as orphans.
        let orphans = find_orphans(&client, &config.default_namespace).await;

        assert!(!orphans.configmaps.contains(cfgmap_name.as_str()));
        assert!(!orphans.secrets.contains(secret_name.as_str()));

        // Free resources after the test
        dep_api
            .delete(&deployment_w_pod_spec.name(), &DeleteParams::default())
            .await
            .expect("Deployment not deleted.");
        secret_api
            .delete(&secret_name, &DeleteParams::default())
            .await
            .expect("Secret not deleted.");
        cfgmap_api
            .delete(&cfgmap_name, &DeleteParams::default())
            .await
            .expect("ConfigMap not deleted.");
    }

    #[tokio::test]
    async fn cfgmap_secret_orphans() {
        let config = Config::infer()
            .await
            .expect("KUBECONFIG env var not set or invalid path/content.");
        let client = Client::try_from(config.clone()).expect("Kubernetes cluster unreachable.");

        // Create configmap in Kubernetes cluster
        let mut config_data: BTreeMap<String, String> = BTreeMap::new();
        config_data.insert("test_key".to_owned(), "test_value".to_owned());
        let cfgmap = ConfigMap {
            metadata: ObjectMeta {
                name: Some("orphan-cfgmap".to_string()),
                ..ObjectMeta::default()
            },
            data: config_data.clone(),
            ..ConfigMap::default()
        };

        let cfgmap_api = Api::<ConfigMap>::namespaced(client.clone(), &config.default_namespace);
        cfgmap_api
            .create(&PostParams::default(), &cfgmap)
            .await
            .expect("Configmap not created.");

        // Create secretg in the Kubernetes cluster
        let mut secret_data: BTreeMap<String, ByteString> = BTreeMap::new(); // Used both for cfgmap and secret
        secret_data.insert(
            "test_key".to_owned(),
            ByteString(base64::encode("test_value").into_bytes()),
        );
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("orphan-secret".to_string()),
                ..ObjectMeta::default()
            },
            data: secret_data.clone(),
            ..Secret::default()
        };
        let secret_api = Api::<Secret>::namespaced(client.clone(), &config.default_namespace);
        secret_api
            .create(&PostParams::default(), &secret)
            .await
            .expect("Secret not created.");

        let cfgmap_name = cfgmap.name();
        let secret_name = secret.name();
        // Both the ConfigMap and the Secret should not be detected as orphans.
        let orphans = find_orphans(&client, &config.default_namespace).await;
        assert!(orphans.configmaps.contains(cfgmap_name.as_str()));
        assert!(orphans.secrets.contains(secret_name.as_str()));

        // Free resources after the test
        secret_api
            .delete(&secret_name, &DeleteParams::default())
            .await
            .expect("Secret not deleted.");
        cfgmap_api
            .delete(&cfgmap_name, &DeleteParams::default())
            .await
            .expect("ConfigMap not deleted.");
    }

    #[tokio::test]
    async fn cfgmap_secret_not_referenced_no_envfrom() {
        let config = Config::infer()
            .await
            .expect("KUBECONFIG env var not set or invalid path/content.");
        let client = Client::try_from(config.clone()).expect("Kubernetes cluster unreachable.");

        // Create configmap in Kubernetes cluster
        let mut config_data: BTreeMap<String, String> = BTreeMap::new();
        config_data.insert("test_key".to_owned(), "test_value".to_owned());
        let cfgmap = ConfigMap {
            metadata: ObjectMeta {
                name: Some("configmap-not-linked-no-envfrom".to_string()),
                ..ObjectMeta::default()
            },
            data: config_data.clone(),
            ..ConfigMap::default()
        };

        let cfgmap_api = Api::<ConfigMap>::namespaced(client.clone(), &config.default_namespace);
        cfgmap_api
            .create(&PostParams::default(), &cfgmap)
            .await
            .expect("Configmap not created.");

        // Create secretg in the Kubernetes cluster
        let mut secret_data: BTreeMap<String, ByteString> = BTreeMap::new(); // Used both for cfgmap and secret
        secret_data.insert(
            "test_key".to_owned(),
            ByteString(base64::encode("test_value").into_bytes()),
        );
        let secret = Secret {
            metadata: ObjectMeta {
                name: Some("secret-not-linked-no-envfrom".to_string()),
                ..ObjectMeta::default()
            },
            data: secret_data.clone(),
            ..Secret::default()
        };
        let secret_api = Api::<Secret>::namespaced(client.clone(), &config.default_namespace);
        secret_api
            .create(&PostParams::default(), &secret)
            .await
            .expect("Secret not created.");

        // Create a deployment linking to both `ConfigMap` and the `Secret`
        let deployment_w_pod_spec = Deployment {
            metadata: ObjectMeta {
                name: Some("deployment-no-envfrom".to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                template: PodTemplateSpec {
                    metadata: Some(ObjectMeta {
                        labels: BTreeMap::<String, String>::from_iter(IntoIter::new([(
                            "app".to_string(),
                            "deployment".to_string(),
                        )])),
                        ..ObjectMeta::default()
                    }),
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: "nginx".to_string(),
                            image: Some("alpine:latest".to_string()),
                            command: vec!["sleep".to_string()],
                            args: vec!["infinity".to_string()],
                            ..Container::default()
                        }],
                        ..PodSpec::default()
                    }),
                    ..PodTemplateSpec::default()
                },
                selector: LabelSelector {
                    match_labels: BTreeMap::<String, String>::from_iter(IntoIter::new([(
                        "app".to_string(),
                        "deployment".to_string(),
                    )])),
                    ..LabelSelector::default()
                },
                ..DeploymentSpec::default()
            }),
            ..Deployment::default()
        };

        let dep_api = Api::<Deployment>::namespaced(client.clone(), &config.default_namespace);
        dep_api
            .create(&PostParams::default(), &deployment_w_pod_spec)
            .await
            .expect("Deployment resources not created in Kubernetes cluster.");

        let cfgmap_name = cfgmap.name();
        let secret_name = secret.name();
        // Both the ConfigMap and the Secret should not be detected as orphans.
        let orphans = find_orphans(&client, &config.default_namespace).await;

        assert!(orphans.configmaps.contains(cfgmap_name.as_str()));
        assert!(orphans.secrets.contains(secret_name.as_str()));

        // Free resources after the test
        dep_api
            .delete(&deployment_w_pod_spec.name(), &DeleteParams::default())
            .await
            .expect("Deployment not deleted.");
        secret_api
            .delete(&secret_name, &DeleteParams::default())
            .await
            .expect("Secret not deleted.");
        cfgmap_api
            .delete(&cfgmap_name, &DeleteParams::default())
            .await
            .expect("ConfigMap not deleted.");
    }
}
