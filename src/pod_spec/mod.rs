use std::option::Option;

use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::batch::v1beta1::CronJob;
use k8s_openapi::api::core::v1::{Pod, PodSpec, ReplicationController};

pub trait ResourceWithPodSpec: Sized {
    fn pod_template_spec(&self) -> Option<&PodSpec>;
}

impl ResourceWithPodSpec for Deployment {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.spec.as_ref()
    }
}

impl ResourceWithPodSpec for ReplicaSet {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.as_ref()?.spec.as_ref()
    }
}

impl ResourceWithPodSpec for StatefulSet {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.spec.as_ref()
    }
}

impl ResourceWithPodSpec for DaemonSet {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.spec.as_ref()
    }
}

impl ResourceWithPodSpec for Job {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.spec.as_ref()
    }
}

impl ResourceWithPodSpec for CronJob {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec
            .as_ref()?
            .job_template
            .spec
            .as_ref()?
            .template
            .spec
            .as_ref()
    }
}

impl ResourceWithPodSpec for ReplicationController {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()?.template.as_ref()?.spec.as_ref()
    }
}

impl ResourceWithPodSpec for Pod {
    fn pod_template_spec(&self) -> Option<&PodSpec> {
        self.spec.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
    use k8s_openapi::api::core::v1::{Container, PodSpec, PodTemplateSpec};
    use kube::api::ObjectMeta;

    use crate::pod_spec::ResourceWithPodSpec;

    #[tokio::test]
    async fn deployment_pod_spec() {
        let deployment_w_pod_spec = Deployment {
            metadata: ObjectMeta {
                name: Some("deployment_pod_spec".to_string()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                template: PodTemplateSpec {
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
                ..DeploymentSpec::default()
            }),
            ..Deployment::default()
        };

        let pod_spec_option: Option<&PodSpec> = deployment_w_pod_spec.pod_template_spec();
        assert!(pod_spec_option.is_some());
        assert_eq!(
            pod_spec_option.unwrap(),
            deployment_w_pod_spec
                .spec
                .as_ref()
                .unwrap()
                .template
                .spec
                .as_ref()
                .unwrap()
        );
    }
}
