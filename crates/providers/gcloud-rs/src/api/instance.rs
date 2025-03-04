use serde::{de::DeserializeOwned, Deserialize, Serialize};
use reqwest::{header::AUTHORIZATION, Response};
use anyhow::bail;

const BASE_URL: &str = "compute/v1";
const SERVICE_ENDPOINT: &str = "https://compute.googleapis.com";

#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceData {
    name: String,
    #[serde(rename = "machineType")]
    machine_type: String,
    status: String,
    disks: Vec<DiskData>,
    #[serde(rename = "networkInterfaces")]
    network_interfaces: Vec<NetworkInterfaceData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskData {
    #[serde(rename = "autoDelete")]
    auto_delete: bool,
    boot: bool,
    #[serde(rename = "deviceName")]
    device_name: String,
    #[serde(rename = "initializeParams")]
    initialize_params: Option<DiskInitializeParams>,
    mode: String,
    #[serde(rename = "type")]
    disk_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiskInitializeParams {
    #[serde(rename = "diskSizeGb")]
    disk_size_gb: String,
    #[serde(rename = "diskType")]
    disk_type: String,
    #[serde(rename = "sourceImage")]
    source_image: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterfaceData {
    name: Option<String>,
    #[serde(rename = "accessConfigs")]
    access_configs: Vec<AccessConfigData>,
    #[serde(rename = "stackType")]
    stack_type: String,
    subnetwork: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccessConfigData {
    name: String,
    #[serde(rename = "networkTier")]
    network_tier: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorDetail {
    message: String,
    domain: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorValue {
    code: i32,
    message: String,
    errors: Vec<ErrorDetail>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationSuccess {
        kind: String,
        id: String,
        name: Option<String>,
        zone: Option<String>,
        #[serde(rename = "operationType")]
        operation_type: Option<String>,
        #[serde(rename = "targetLink")]
        target_link: Option<String>,
        #[serde(rename = "targetId")]
        target_id: Option<String>,
        status: Option<String>,
        user: Option<String>,
        progress: Option<i32>,
        #[serde(rename = "insertTime")]
        insert_time: Option<String>,
        #[serde(rename = "startTime")]
        start_time: Option<String>,
        #[serde(rename = "selfLink")]
        self_link: Option<String>,
        items: Option<Vec<InstanceData>>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationFailure {
        error: ErrorValue
    }

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OperationResult {
    OperationSuccess(OperationSuccess),
    OperationFailure(OperationFailure)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OperationInstanceResult {
    name: String,
    #[serde(rename = "machineType")]
    machine_type: String,
    status: String,
}

pub struct InstanceCfg {
    pub instance_name: String,
    pub machine_type: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    project: String,
    region: String,
    zone: String,
    access_token: String,
}

impl Client {
    pub fn new(project: String, region: String, zone: String, access_token: String) -> Self {
        Self {
            project,
            region,
            zone,
            access_token
        }
    }

    pub fn full_url(&self, instance_name: Option<String>) -> String {
        if let Some(name) = instance_name {
            return format!("{SERVICE_ENDPOINT}/{BASE_URL}/projects/{0}/zones/{1}/instances/{2}", self.project, self.zone, name);
        } else {
            return format!("{SERVICE_ENDPOINT}/{BASE_URL}/projects/{0}/zones/{1}/instances", self.project, self.zone);
        };
    }

    pub async fn insert<T: DeserializeOwned>(
        &self,
        instance_cfg: InstanceCfg,
        disks: Vec<DiskData>,
        network_interfaces: Vec<NetworkInterfaceData>
    ) -> anyhow::Result<T> {
        let instance_data = InstanceData {
            name: instance_cfg.instance_name,
            machine_type: format!("zones/{0}/machineTypes/{1}", self.zone, instance_cfg.machine_type),
            status: "STOPPING".to_string(),
            disks,
            network_interfaces
        };

        let client = reqwest::Client::new();
        let res = client
            .post(self.full_url(None))
            .json(&instance_data)
            .header(AUTHORIZATION, format!("Bearer {0}", self.access_token))
            .send()
            .await?;
        let url = res.url();
        let status = res.status().clone();

        if res.status().is_success() || res.status().is_client_error() {           
            return self.parse_json_body::<T>(res).await;
        } else {
            bail!("Request to url: {0}, got server error, status: {1}", url, status.as_str());
        }
    }

    pub fn prepare_disk_data(
        &self,
        instance_cfg: &InstanceCfg,
        disk_size_gb: String,
        disk_type: String,
        source_image: String) -> DiskData {
        return DiskData {
            auto_delete: true,
            boot: true,
            device_name: instance_cfg.instance_name.clone(),
            initialize_params: Some(DiskInitializeParams {
                disk_size_gb,
                disk_type: format!("zones/{0}/diskTypes/{1}", self.zone, disk_type),
                source_image
            }),
            mode: "READ_WRITE".to_string(),
            disk_type: "PERSISTENT".to_string()
        };
    }

    pub fn prepare_network_interface_data(
        &self,
        name: String,
        network_tier: String,
        stack_type: String) -> NetworkInterfaceData{
        return NetworkInterfaceData {
            name: None,
            access_configs: vec!(AccessConfigData {
                name,
                network_tier
            }),
            stack_type,
            subnetwork: format!("regions/{0}/subnetworks/default", self.region)
        };
    }

    pub async fn parse_json_body<T: DeserializeOwned>(&self, res: Response) -> anyhow::Result<T>{
        let url = res.url().clone();
        let full = res.bytes().await?;
        let text = String::from_utf8_lossy(&full);
        let data = serde_json::from_slice(&full);
        match data {
            Ok(t) => Ok(t),
            Err(error) => {bail!("Request to url: {0:?}, parsing data got error: {1}, text: {2}", url, error.to_string(), text);}
        }
    }
 
    pub async fn list<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .get(self.full_url(None))
            .header(AUTHORIZATION, format!("Bearer {0}", self.access_token))
            .send()
            .await?;
        let url = res.url();
        let status = res.status().clone();

        if res.status().is_success() || res.status().is_client_error() {           
            return self.parse_json_body::<T>(res).await;
        } else {
            bail!("Request to url: {0}, got server error, status: {1}", url, status.as_str());
        }
    }
 
    pub async fn get<T: DeserializeOwned>(&self, instance_name: String) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .get(self.full_url(Some(instance_name)))
            .header(AUTHORIZATION, format!("Bearer {0}", self.access_token))
            .send()
            .await?;
        let url = res.url();
        let status = res.status().clone();

        if res.status().is_success() || res.status().is_client_error() {           
            return self.parse_json_body::<T>(res).await;
        } else {
            bail!("Request to url: {0}, got server error, status: {1}", url, status.as_str());
        }
    }
 
    pub async fn delete<T: DeserializeOwned>(&self, instance_name: String) -> anyhow::Result<T> {
        let client = reqwest::Client::new();
        let res = client
            .delete(self.full_url(Some(instance_name)))
            .header(AUTHORIZATION, format!("Bearer {0}", self.access_token))
            .send()
            .await?;
        let url = res.url();
        let status = res.status().clone();

        if res.status().is_success() || res.status().is_client_error() {           
            return self.parse_json_body::<T>(res).await;
        } else {
            bail!("Request to url: {0}, got server error, status: {1}", url, status.as_str());
        }
    }
}