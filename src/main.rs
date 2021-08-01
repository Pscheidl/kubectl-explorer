use kube::{Client, Config};
use std::convert::TryFrom;

use crate::input::{parse_user_input, Output, UserArgs};
use crate::orphans::find_orphans;
use kube::config::{KubeConfigOptions, Kubeconfig};

mod input;
mod orphans;
mod pod_spec;
mod resources;

#[tokio::main]
async fn main() {
    let user_args: UserArgs = parse_user_input();

    let config = match user_args.kubeconfig {
        None => Config::infer()
            .await
            .expect("Expected a valid KUBECONFIG environment variable"),
        Some(kubeconfig_path) => Config::from_custom_kubeconfig(
            Kubeconfig::read_from(kubeconfig_path)
                .expect("User-provided KUBECONFIG path is invalid."),
            &KubeConfigOptions::default(),
        )
        .await
        .expect("Invalid KUBECONFIG"),
    };
    let namespace = match user_args.namespace {
        None => config.default_namespace.clone(),
        Some(ns) => ns,
    };

    println!(
        "Searching for unused ConfigMaps and Secrets in the '{}' namespace",
        &namespace
    );

    let client: Client = Client::try_from(config.clone()).unwrap();

    let orphans = find_orphans(&client, &namespace).await;

    match user_args.output {
        Output::Yaml => {
            println!("{}", serde_yaml::to_string(&orphans).unwrap());
        }
        Output::Json => {
            println!("{}", serde_json::to_string_pretty(&orphans).unwrap());
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kube API error")]
    KubeError {
        #[from]
        source: kube::Error,
    },
}
