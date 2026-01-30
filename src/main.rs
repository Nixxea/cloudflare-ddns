mod cloudflare;

use std::collections::HashSet;

use cloudflare::{CloudflareClient,DnsRecord};
use serde::{Deserialize, Deserializer};

fn unicode_to_punycode<'de, D>(deserializer: D) -> Result<HashSet<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let domains: Vec<String> = Vec::deserialize(deserializer)?;
    domains.into_iter()
        .map(|domain| 
            idna::domain_to_ascii(&domain)
                .map_err(|err| serde::de::Error::custom(format!("Invalid domain {}: {}", domain, err)))
        ).collect()
}

#[derive(Deserialize)]
struct Zone {
    id: String,
    #[serde(deserialize_with = "unicode_to_punycode")]
    domains: HashSet<String>
}

#[derive(Deserialize)]
struct Config {
    token: String,
    zones: Vec<Zone>,
    proxy: Option<String>
}

impl Config {
    fn from_file(path: &str) -> Result<Config, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let parsed: Config = serde_json::from_str(content.as_str())?;
        Ok(parsed)
    }
}

const CACHE_IP_PATH: &str = "./last-ip.txt";
const CONFIG_PATH: &str = "./config.json";

fn read_cached_ip() -> Result<String, std::io::Error> {
    match std::fs::read_to_string(CACHE_IP_PATH) {
        Ok(content) => Ok(content),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            std::fs::File::create(CACHE_IP_PATH)?;
            Ok(String::new())
        },
        Err(err) => {
            return Err(err)
        }
    }
}

fn cache_ip(ip: String) -> Result<(), std::io::Error> {
    std::fs::write(CACHE_IP_PATH, ip.as_bytes())?;

    Ok(())
}

async fn get_str_ip() -> Result<String, reqwest::Error> {
    reqwest::get("https://api.ipify.org")
    .await?
    .text()
    .await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cached_ip = read_cached_ip()?;
    let ip = get_str_ip().await?;
    let config = Config::from_file(CONFIG_PATH)?;

    println!("{ip}");
    if cached_ip == ip {
        print!("Этот IP уже настраивался");
        return Ok(());
    };

    let cloudflare_client = CloudflareClient::new(config.token, config.proxy); 

    for zone in config.zones {
        let dns_records = cloudflare_client.get_dns_records(&zone.id).await?;

        for record in dns_records {
            if record.kind != "A" { continue; }; 
            if !zone.domains.contains(&record.name) { continue };

            let new_record = DnsRecord {
                id: record.id,
                content: ip.clone(),
                name: record.name.clone(),
                kind: String::from("A")
            };

            cloudflare_client.update_dns_record(&zone.id, new_record).await?;
            println!("Обновлено: {}", idna::domain_to_unicode(&record.name).0);
        }
    }

    cache_ip(ip)?;
    Ok(())
}