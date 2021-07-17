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
