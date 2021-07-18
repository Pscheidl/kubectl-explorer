use std::collections::HashSet;
use std::convert::TryFrom;

use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{Client, Config};

use crate::orphans::find_orphans;
use crate::resources::list_resource;

mod orphans;
mod pod_spec;
mod resources;

#[tokio::main]
async fn main() {
    let config = Config::infer()
        .await
        .expect("Expected a valid KUBECONFIG environment variable");
    let client: Client = Client::try_from(config.clone()).unwrap();
    let configmaps_fut = list_resource::<ConfigMap>(&client, &config.default_namespace);
    let secrets_fut = list_resource::<Secret>(&client, &config.default_namespace);
    let (cfgmaps_res, secrets_res) = tokio::join!(configmaps_fut, secrets_fut);

    // Todo: Can be generalized as both resources implement the `Metadata` trait
    let cfgmaps = cfgmaps_res.unwrap();
    let cfgmap_names: HashSet<&str> = cfgmaps
        .iter()
        .map(|r| r.metadata.name.as_ref().unwrap().as_str())
        .collect();

    let secrets = secrets_res.unwrap();
    let secrets_names: HashSet<&str> = secrets
        .iter()
        .map(|r| r.metadata.name.as_ref().unwrap().as_str())
        .collect();

    let orphans = find_orphans(
        &secrets_names,
        &cfgmap_names,
        &client,
        &config.default_namespace,
    )
    .await;

    println!("Orphan configmaps:");
    orphans.cfgmaps.iter().for_each(|res| {
        println!("{}", res);
    });

    println!("Orphan secrets:");
    orphans.secrets.iter().for_each(|res| {
        println!("{}", res);
    });
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kube API error")]
    KubeError {
        #[from]
        source: kube::Error,
    },
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
    use std::collections::{BTreeMap, HashSet};
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
        let cfgmaps_names = HashSet::<&str>::from_iter(IntoIter::new([cfgmap_name.as_str()]));
        let secrets_names = HashSet::<&str>::from_iter(IntoIter::new([secret_name.as_str()]));
        // Both the ConfigMap and the Secret should not be detected as orphans.
        let orphans = find_orphans(
            &secrets_names,
            &cfgmaps_names,
            &client,
            &config.default_namespace,
        )
        .await;

        assert!(!orphans.cfgmaps.contains(cfgmap_name.as_str()));
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
        let cfgmaps_names = HashSet::<&str>::from_iter(IntoIter::new([cfgmap_name.as_str()]));
        let secrets_names = HashSet::<&str>::from_iter(IntoIter::new([secret_name.as_str()]));
        // Both the ConfigMap and the Secret should not be detected as orphans.
        let orphans = find_orphans(
            &secrets_names,
            &cfgmaps_names,
            &client,
            &config.default_namespace,
        )
        .await;

        assert!(orphans.cfgmaps.contains(cfgmap_name.as_str()));
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
}
