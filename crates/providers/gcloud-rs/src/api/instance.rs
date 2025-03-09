/*
    Specifications: https://cloud.google.com/compute/docs/reference/rest/v1/instances
 */
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use reqwest::{header::AUTHORIZATION, Response};
use anyhow::bail;

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
    service_endpoint: String,
    base_url: String,
    project: String,
    region: String,
    zone: String,
    access_token: String,
}

impl Client {
    pub fn new(
        service_endpoint: String,
        base_url: String,
        project: String,
        region: String,
        zone: String,
        access_token: String) -> Self {
        Self {
            service_endpoint,
            base_url,
            project,
            region,
            zone,
            access_token
        }
    }

    pub fn full_url(&self, instance_name: Option<String>) -> String {
        if let Some(name) = instance_name {
            return format!(
                "{0}/{1}/projects/{2}/zones/{3}/instances/{4}",
                self.service_endpoint, self.base_url, self.project, self.zone, name);
        } else {
            return format!(
                "{0}/{1}/projects/{2}/zones/{3}/instances",
                self.service_endpoint, self.base_url, self.project, self.zone);
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

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::*;

    async fn prepare_test_request(
        method: String,
        instance_name: Option<String>,
        status_code: usize,
        res_body: Option<String>) -> (Mock, Client) {
        let base_url = "compute/v1".to_string();
        let access_token = "dummy_token".to_string();
        let project = "test_project".to_string();
        let region = "test_region".to_string();
        let zone = "test_zone".to_string();

        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        let service_endpoint = server.url();
 
        let binding_url = match instance_name {
            Some(name) => format!("/compute/v1/projects/test_project/zones/test_zone/instances/{}", &name).clone(),
            _ => "/compute/v1/projects/test_project/zones/test_zone/instances".to_string()
        };

        // Create a mock
        let mut mock = server.mock(&method, binding_url.as_str())
        .with_status(status_code)
        .with_header("content-type", "application/json")
        .with_header("Authorization", "Bearer dummy_token");

        if let Some(body) = res_body {
            mock = mock.with_body(body);
        }

        let client = Client::new(
            service_endpoint,
            base_url,
            project,
            region,
            zone,
            access_token
        );

        return (mock, client);
    }

    #[tokio::test]
    async fn test_insert_success() {
        let instance_name = "test_instance".to_string();
        let machine_type = "e2-micro".to_string();
        let disk_type = "pd-balanced".to_string();
        let disk_image = "projects/debian-cloud/global/images/debian-12-bookworm-v20250212".to_string();
        let network_interface_name = "External NAT".to_string();
        let res_body_kind = "compute#operation";
        let res_body_id = "1231944675420996843";
        let res_body = Some(format!("{{
            \"kind\": \"{res_body_kind}\",
            \"id\": \"{res_body_id}\"}}
        "));

        let (mut mock, client) = prepare_test_request(
            "POST".to_string(), None, 200, res_body).await;

        let instance_cfg = InstanceCfg {            
            instance_name: instance_name.clone(),
            machine_type: machine_type.clone()
        };
        let disk = client.prepare_disk_data(
            &instance_cfg,
            "10".to_string(),
            disk_type.clone(),
            disk_image.clone()
        );
        let network_interface = client.prepare_network_interface_data(
                network_interface_name.clone(),
                "PREMIUM".to_string(),
                "IPV4_ONLY".to_string()
            );

        mock = mock
            .match_body(
                    mockito::Matcher::AllOf(vec![
                        mockito::Matcher::Regex(instance_name.clone()),
                        mockito::Matcher::Regex(machine_type.clone()),
                        mockito::Matcher::Regex(disk_type.clone()),
                        mockito::Matcher::Regex(disk_image.clone()),
                        mockito::Matcher::Regex(network_interface_name.clone())
                    ])
                )
            .create();

        let result = client.insert::<OperationResult>(
            instance_cfg,
            vec!(disk),
            vec!(network_interface)
            ).await.unwrap();

        mock.assert();
        
        if let OperationResult::OperationSuccess(r) = result {
            assert_eq!(res_body_id, r.id);
            assert_eq!(res_body_kind, r.kind);
        } else {
            panic!("result must be intance of OperationSuccess")
        }
    }

    #[tokio::test]
    async fn test_delete_success() {
        let instance_name = "test_instance".to_string();
        let res_body_kind = "compute#operation";
        let res_body_id = "1231944675420996843";
        let res_body = Some(format!("{{
            \"kind\": \"{res_body_kind}\",
            \"id\": \"{res_body_id}\"}}
        "));

        let (mut mock, client) = prepare_test_request(
            "DELETE".to_string(),
            Some(instance_name.clone()),
            200,
            res_body).await;

        mock = mock.create();
        
        let result = client.delete::<OperationResult>(instance_name).await.unwrap();

        mock.assert();
        
        if let OperationResult::OperationSuccess(r) = result {
            assert_eq!(res_body_id, r.id);
            assert_eq!(res_body_kind, r.kind);
        } else {
            panic!("result must be intance of OperationSuccess")
        }
    }

    #[tokio::test]
    async fn test_get_failure() {
        let instance_name = "test_instance".to_string();
        let res_body_error_code = 400;
        let res_body_erro_message = "The resource was not found".to_string();
        let res_body = Some(format!("{{
            \"error\": {{
                \"code\": {0},
                \"message\": \"{1}\",
                \"errors\": [
                    {{
                        \"message\": \"The resource was not found\",
                        \"domain\": \"global\",
                        \"reason\": \"notFound\"
                    }}
                ]
                }}
        }}", res_body_error_code, res_body_erro_message));

        let (mut mock, client) = prepare_test_request(
            "GET".to_string(),
            Some(instance_name.clone()),
            400,
            res_body).await;

        mock = mock.create();
        
        let result = client.get::<OperationResult>(instance_name).await.unwrap();

        mock.assert();
        
        if let OperationResult::OperationFailure(r) = result {
            assert_eq!(res_body_error_code, r.error.code);
            assert_eq!(res_body_erro_message, r.error.message);
        } else {
            panic!("result must be intance of OperationFailure");
        }
    }
}
