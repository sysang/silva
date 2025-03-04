// use gcloud_rs::api::instance::{Client, InstanceCfg, OperationResult};
use gcloud_rs::api::instance::{Client, InstanceCfg, OperationResult};

// Current way to do authentication is to use cloud cli.
// Details are described in https://cloud.google.com/docs/authentication/gcloud
// access token is generated in local file, the loaded and passwd to client instance

#[tokio::main]
async fn main() -> Result<(), ()> {
    let client = Client::new(
        "startingone-1671013902513".to_string(),
        "us-central1".to_string(),
        "us-central1-a".to_string(),
        "ya29.a0AeXRPp7Wpzoq6KLFjYO6uTLl57rrnHmLKSdQ-hfXqBXgCU0Cb00ZXDVbm-perPsRLCURws91WXwUf9kYvfVIF9hl5eetfxTegozzYoEGP5LoTM2jwQ8SZ8DhD9c_hl3waXq8MgkU257OuBzyTqFn9PmWekC9ah23dyfuqBxbJwaCgYKAd4SARMSFQHGX2MidHkkB-EkhCRQDXUWo-6CPg0177".to_string()
    );
    // let instance_cfg = InstanceCfg {            
    //     instance_name: "instance-20250222-104328".to_string(),
    //     machine_type: "e2-micro".to_string()
    // };
    // let disk = client.prepare_disk_data(
    //     &instance_cfg,
    //     "10".to_string(),
    //     "pd-balanced".to_string(), 
    //     "projects/debian-cloud/global/images/debian-12-bookworm-v20250212".to_string());
    // let network_interface = client.prepare_network_interface_data(
    //         "External NAT".to_string(),
    //         "PREMIUM".to_string(),
    //         "IPV4_ONLY".to_string()
    //     );

    // let result = client.insert::<OperationResult>(instance_cfg, vec!(disk), vec!(network_interface)).await;
    // let result = client.list::<OperationResult>().await;
    // let result = client.get::<OperationResult>("instance-20250222-104328".to_string()).await;
    let result = client.delete::<OperationResult>("instance-20250222-104328".to_string()).await;

    match result {
        Ok(result) => {print!("result: {:?}", result); Ok(())},
        Err(error) => {print!("managed error: {:?}", error); Err(())}
    }
}
