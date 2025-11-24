// SSM client for listing and connecting to instances
use crate::error::{Result, SsoError};
use aws_sdk_ec2::Client as Ec2Client;
use aws_sdk_ssm::Client as SsmClient;

/// Information about an EC2 instance with SSM capability
#[derive(Debug, Clone)]
pub struct SsmInstance {
    pub instance_id: String,
    pub name: String,
    pub state: String,
    pub platform: Option<String>,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub ssm_status: SsmStatus,
}

/// SSM connection status for an instance
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsmStatus {
    /// SSM agent is online and ready
    Online,
    /// SSM agent is offline or not installed
    Offline,
    /// Status unknown (couldn't check)
    Unknown,
}

impl SsmStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SsmStatus::Online => "Online",
            SsmStatus::Offline => "Offline",
            SsmStatus::Unknown => "Unknown",
        }
    }

    pub fn is_connectable(&self) -> bool {
        matches!(self, SsmStatus::Online)
    }
}

/// Client for SSM/EC2 operations
pub struct SsmSdkClient {
    ec2_client: Ec2Client,
    ssm_client: SsmClient,
    region: String,
}

impl SsmSdkClient {
    /// Create a new SSM client for the specified region
    pub async fn new(region: &str) -> Result<Self> {
        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .load()
            .await;

        let ec2_client = Ec2Client::new(&config);
        let ssm_client = SsmClient::new(&config);

        Ok(Self {
            ec2_client,
            ssm_client,
            region: region.to_string(),
        })
    }

    /// Create a new SSM client using explicit credentials
    pub async fn with_credentials(
        region: &str,
        access_key_id: &str,
        secret_access_key: &str,
        session_token: &str,
    ) -> Result<Self> {
        let credentials = aws_sdk_ec2::config::Credentials::new(
            access_key_id,
            secret_access_key,
            Some(session_token.to_string()),
            None,
            "awsom",
        );

        let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(region.to_string()))
            .credentials_provider(credentials)
            .load()
            .await;

        let ec2_client = Ec2Client::new(&config);
        let ssm_client = SsmClient::new(&config);

        Ok(Self {
            ec2_client,
            ssm_client,
            region: region.to_string(),
        })
    }

    /// Get the region this client is configured for
    pub fn region(&self) -> &str {
        &self.region
    }

    /// List all EC2 instances and their SSM status
    pub async fn list_instances(&self) -> Result<Vec<SsmInstance>> {
        // First, get all EC2 instances
        let ec2_instances = self.list_ec2_instances().await?;

        if ec2_instances.is_empty() {
            return Ok(vec![]);
        }

        // Get SSM status for all instances
        let instance_ids: Vec<&str> = ec2_instances
            .iter()
            .map(|i| i.instance_id.as_str())
            .collect();
        let ssm_statuses = self.get_ssm_statuses(&instance_ids).await?;

        // Merge the data
        let instances: Vec<SsmInstance> = ec2_instances
            .into_iter()
            .map(|mut instance| {
                instance.ssm_status = ssm_statuses
                    .get(&instance.instance_id)
                    .cloned()
                    .unwrap_or(SsmStatus::Unknown);
                instance
            })
            .collect();

        Ok(instances)
    }

    /// List EC2 instances (without SSM status)
    async fn list_ec2_instances(&self) -> Result<Vec<SsmInstance>> {
        let mut instances = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = self.ec2_client.describe_instances();

            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request.send().await.map_err(|e| {
                SsoError::AwsSdk(format!("Failed to describe EC2 instances: {}", e))
            })?;

            for reservation in response.reservations() {
                for instance in reservation.instances() {
                    let instance_id = instance.instance_id().unwrap_or("").to_string();
                    if instance_id.is_empty() {
                        continue;
                    }

                    // Get instance name from tags
                    let name = instance
                        .tags()
                        .iter()
                        .find(|tag| tag.key() == Some("Name"))
                        .and_then(|tag| tag.value())
                        .unwrap_or("")
                        .to_string();

                    let state = instance
                        .state()
                        .and_then(|s| s.name())
                        .map(|s| s.as_str().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    let platform = instance.platform_details().map(|s| s.to_string());

                    let private_ip = instance.private_ip_address().map(|s| s.to_string());
                    let public_ip = instance.public_ip_address().map(|s| s.to_string());

                    instances.push(SsmInstance {
                        instance_id,
                        name,
                        state,
                        platform,
                        private_ip,
                        public_ip,
                        ssm_status: SsmStatus::Unknown,
                    });
                }
            }

            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(instances)
    }

    /// Get SSM status for a list of instance IDs
    async fn get_ssm_statuses(
        &self,
        instance_ids: &[&str],
    ) -> Result<std::collections::HashMap<String, SsmStatus>> {
        let mut statuses = std::collections::HashMap::new();

        // SSM DescribeInstanceInformation can only handle 50 instances at a time
        for chunk in instance_ids.chunks(50) {
            let mut next_token: Option<String> = None;

            loop {
                let mut request = self.ssm_client.describe_instance_information();

                // Filter by instance IDs in this chunk
                let filter = aws_sdk_ssm::types::InstanceInformationStringFilter::builder()
                    .key("InstanceIds")
                    .set_values(Some(chunk.iter().map(|s| s.to_string()).collect()))
                    .build()
                    .map_err(|e| SsoError::AwsSdk(format!("Failed to build filter: {}", e)))?;

                request = request.filters(filter);

                if let Some(token) = next_token {
                    request = request.next_token(token);
                }

                let response = request.send().await.map_err(|e| {
                    SsoError::AwsSdk(format!("Failed to describe SSM instances: {}", e))
                })?;

                for info in response.instance_information_list() {
                    if let Some(instance_id) = info.instance_id() {
                        let status = match info.ping_status() {
                            Some(ping) => {
                                if ping.as_str() == "Online" {
                                    SsmStatus::Online
                                } else {
                                    SsmStatus::Offline
                                }
                            }
                            None => SsmStatus::Unknown,
                        };
                        statuses.insert(instance_id.to_string(), status);
                    }
                }

                next_token = response.next_token().map(|s| s.to_string());
                if next_token.is_none() {
                    break;
                }
            }
        }

        // Mark any instances not in SSM response as Offline
        for id in instance_ids {
            statuses.entry(id.to_string()).or_insert(SsmStatus::Offline);
        }

        Ok(statuses)
    }

    /// Generate the AWS CLI command to start an SSM session
    pub fn session_command(&self, instance_id: &str) -> String {
        format!(
            "aws ssm start-session --target {} --region {}",
            instance_id, self.region
        )
    }

    /// Start an SSM session (opens in terminal)
    /// Returns the command that was executed
    pub fn start_session(&self, instance_id: &str) -> Result<String> {
        let cmd = self.session_command(instance_id);

        // On macOS, use osascript to open a new Terminal window
        #[cfg(target_os = "macos")]
        {
            let script = format!(
                r#"tell application "Terminal"
                    activate
                    do script "{}"
                end tell"#,
                cmd.replace('"', r#"\""#)
            );

            std::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .spawn()
                .map_err(|e| SsoError::AwsSdk(format!("Failed to open Terminal: {}", e)))?;
        }

        // On Linux, try common terminal emulators
        #[cfg(target_os = "linux")]
        {
            // Try gnome-terminal, then xterm as fallback
            let result = std::process::Command::new("gnome-terminal")
                .arg("--")
                .arg("sh")
                .arg("-c")
                .arg(&cmd)
                .spawn();

            if result.is_err() {
                std::process::Command::new("xterm")
                    .arg("-e")
                    .arg(&cmd)
                    .spawn()
                    .map_err(|e| SsoError::AwsSdk(format!("Failed to open terminal: {}", e)))?;
            }
        }

        // On Windows, use cmd
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .arg("/c")
                .arg("start")
                .arg("cmd")
                .arg("/k")
                .arg(&cmd)
                .spawn()
                .map_err(|e| SsoError::AwsSdk(format!("Failed to open cmd: {}", e)))?;
        }

        Ok(cmd)
    }
}
