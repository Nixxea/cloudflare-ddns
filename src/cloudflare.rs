use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CloudflareResponse<T> {
    //success: bool,
    pub result: T
}

#[derive(Deserialize, Serialize)]
pub struct DnsRecord {
    pub id: String,
    pub content: String,
    pub name: String,
    pub r#type: String
}

pub struct CloudflareClient {
    client: reqwest::Client,
    token: String
}

impl CloudflareClient {
    pub fn new(token: String) -> Self {
        Self { client: reqwest::Client::new(), token }
    }

    pub async fn get_dns_records(&self, zone_id: &String) -> Result<Vec<DnsRecord>, reqwest::Error> {
        let response = self.client.get(format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records"))
        .header("authorization", format!("Bearer {}", self.token))
        .send()
        .await?
        .json::<CloudflareResponse<Vec<DnsRecord>>>()
        .await?;
    
        Ok(response.result)
    }
    
    pub async fn update_dns_record(&self, zone_id: &String, record: DnsRecord) -> Result<(), reqwest::Error> {
        self.client.patch(format!("https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records/{}", record.id))
        .header("authorization", format!("Bearer {}", self.token))
        .json::<DnsRecord>(&record)
        .send()
        .await?;

        Ok(())
    }
}