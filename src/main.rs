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
        secrets_names,
        cfgmap_names,
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
