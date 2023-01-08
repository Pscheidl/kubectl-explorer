use std::convert::TryFrom;

use anyhow::{Context, Result};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};

use crate::input::{parse_user_input, Output, UserArgs};
use crate::orphans::find_orphans;

mod input;
mod orphans;
mod pod_spec;
mod resources;

#[tokio::main]
async fn main() -> Result<()> {
    let user_args: UserArgs = parse_user_input();

    let config = match user_args.kubeconfig {
        None => Config::infer().await.with_context(|| {
            "No KUBECONFIG path specified and the KUBECONFIG environment variable is not set."
        })?,
        Some(kubeconfig_path) => Config::from_custom_kubeconfig(
            Kubeconfig::read_from(kubeconfig_path.as_str()).with_context(|| {
                format!(
                    "User-provided KUBECONFIG path '{}' is invalid.",
                    &kubeconfig_path
                )
            })?,
            &KubeConfigOptions::default(),
        )
        .await
        .with_context(|| {
            format!(
                "Content of the '{}' kubeconfig is invalid.",
                &kubeconfig_path
            )
        })?,
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

    let orphans = find_orphans(&client, &namespace).await?;

    match user_args.output {
        Output::Yaml => {
            println!("{}", serde_yaml::to_string(&orphans).unwrap());
        }
        Output::Json => {
            println!("{}", serde_json::to_string_pretty(&orphans).unwrap());
        }
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Kube API error")]
    KubeError {
        #[from]
        source: kube::Error,
    },
}
