use k8s_openapi::serde::__private::fmt::Debug;
use k8s_openapi::serde::de::DeserializeOwned;
use kube::api::ListParams;
use kube::{Api, Client, Resource};

use crate::Error;

pub async fn list_resource<T>(client: &Client, namespace: &str) -> Result<Vec<T>, Error>
where
    T: Clone + Debug + DeserializeOwned + Resource,
    <T as Resource>::DynamicType: Default,
{
    let resource_api = Api::<T>::namespaced(client.clone(), namespace);
    Ok(resource_api.list(&ListParams::default()).await?.items)
}
