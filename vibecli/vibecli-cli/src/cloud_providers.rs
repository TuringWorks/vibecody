//! Deep cloud provider integration for AWS, GCP, and Azure.
//!
//! Provides service detection from source code, IAM policy generation with
//! least-privilege principles, Infrastructure-as-Code template generation
//! (CloudFormation, Terraform, Pulumi), and service cost estimation.

use std::collections::HashMap;
use std::fmt;

/// Supported cloud providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
}

impl fmt::Display for CloudProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CloudProvider::AWS => write!(f, "AWS"),
            CloudProvider::GCP => write!(f, "GCP"),
            CloudProvider::Azure => write!(f, "Azure"),
        }
    }
}

/// Categories of cloud service usage.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ServiceUsage {
    Compute,
    Storage,
    Database,
    Messaging,
    Serverless,
    Network,
    AI,
    Container,
    CDN,
    Auth,
    Monitoring,
    Cache,
}

impl fmt::Display for ServiceUsage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ServiceUsage::Compute => "Compute",
            ServiceUsage::Storage => "Storage",
            ServiceUsage::Database => "Database",
            ServiceUsage::Messaging => "Messaging",
            ServiceUsage::Serverless => "Serverless",
            ServiceUsage::Network => "Network",
            ServiceUsage::AI => "AI/ML",
            ServiceUsage::Container => "Container",
            ServiceUsage::CDN => "CDN",
            ServiceUsage::Auth => "Auth",
            ServiceUsage::Monitoring => "Monitoring",
            ServiceUsage::Cache => "Cache",
        };
        write!(f, "{}", s)
    }
}

/// A cloud service identified by provider, name, and usage category.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudService {
    pub provider: CloudProvider,
    pub service_name: String,
    pub usage_type: ServiceUsage,
    pub region: Option<String>,
}

/// A service detected in source code with location and confidence.
#[derive(Debug, Clone, PartialEq)]
pub struct DetectedService {
    pub service: CloudService,
    pub source_file: String,
    pub line_number: usize,
    pub confidence: f64,
}

/// Effect of an IAM policy statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Allow,
    Deny,
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Allow => write!(f, "Allow"),
            Effect::Deny => write!(f, "Deny"),
        }
    }
}

/// A single statement within an IAM policy.
#[derive(Debug, Clone, PartialEq)]
pub struct PolicyStatement {
    pub effect: Effect,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    pub conditions: Vec<String>,
}

/// An IAM policy document for a cloud provider.
#[derive(Debug, Clone, PartialEq)]
pub struct IamPolicy {
    pub provider: CloudProvider,
    pub statements: Vec<PolicyStatement>,
    pub name: String,
}

/// Supported Infrastructure-as-Code formats.
#[derive(Debug, Clone, PartialEq)]
pub enum IacFormat {
    CloudFormation,
    Terraform,
    Pulumi,
}

/// A resource in an IaC template.
#[derive(Debug, Clone, PartialEq)]
pub struct IacResource {
    pub resource_type: String,
    pub logical_name: String,
    pub properties: HashMap<String, String>,
}

/// An output from an IaC template.
#[derive(Debug, Clone, PartialEq)]
pub struct IacOutput {
    pub name: String,
    pub value: String,
    pub description: String,
}

/// An Infrastructure-as-Code template.
#[derive(Debug, Clone, PartialEq)]
pub struct IacTemplate {
    pub provider: CloudProvider,
    pub format: IacFormat,
    pub resources: Vec<IacResource>,
    pub outputs: Vec<IacOutput>,
}

/// Cost estimate for a single service.
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceCost {
    pub service_name: String,
    pub tier: String,
    pub monthly_usd: f64,
    pub notes: String,
}

/// Aggregate cost estimate across services.
#[derive(Debug, Clone, PartialEq)]
pub struct CostEstimate {
    pub provider: CloudProvider,
    pub services: Vec<ServiceCost>,
    pub total_monthly_usd: f64,
    pub total_yearly_usd: f64,
}

/// Manages cloud provider detection, policy generation, IaC output, and cost estimation.
#[derive(Debug, Clone, PartialEq)]
pub struct CloudProviderManager {
    pub detected_services: Vec<DetectedService>,
    pub policies: Vec<IamPolicy>,
}

/// Internal mapping from a code pattern to a cloud service descriptor.
struct ServicePattern {
    pattern: &'static str,
    provider: CloudProvider,
    service_name: &'static str,
    usage_type: ServiceUsage,
    confidence: f64,
}

fn aws_patterns() -> Vec<ServicePattern> {
    vec![
        ServicePattern { pattern: "s3_client", provider: CloudProvider::AWS, service_name: "S3", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "S3Client", provider: CloudProvider::AWS, service_name: "S3", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "create_bucket", provider: CloudProvider::AWS, service_name: "S3", usage_type: ServiceUsage::Storage, confidence: 0.85 },
        ServicePattern { pattern: "put_object", provider: CloudProvider::AWS, service_name: "S3", usage_type: ServiceUsage::Storage, confidence: 0.85 },
        ServicePattern { pattern: "get_object", provider: CloudProvider::AWS, service_name: "S3", usage_type: ServiceUsage::Storage, confidence: 0.85 },
        ServicePattern { pattern: "DynamoDB", provider: CloudProvider::AWS, service_name: "DynamoDB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "dynamodb_client", provider: CloudProvider::AWS, service_name: "DynamoDB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "DynamoDbClient", provider: CloudProvider::AWS, service_name: "DynamoDB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "put_item", provider: CloudProvider::AWS, service_name: "DynamoDB", usage_type: ServiceUsage::Database, confidence: 0.75 },
        ServicePattern { pattern: "Lambda", provider: CloudProvider::AWS, service_name: "Lambda", usage_type: ServiceUsage::Serverless, confidence: 0.80 },
        ServicePattern { pattern: "lambda_client", provider: CloudProvider::AWS, service_name: "Lambda", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "invoke_function", provider: CloudProvider::AWS, service_name: "Lambda", usage_type: ServiceUsage::Serverless, confidence: 0.85 },
        ServicePattern { pattern: "LambdaClient", provider: CloudProvider::AWS, service_name: "Lambda", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "SqsClient", provider: CloudProvider::AWS, service_name: "SQS", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "sqs_client", provider: CloudProvider::AWS, service_name: "SQS", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "send_message", provider: CloudProvider::AWS, service_name: "SQS", usage_type: ServiceUsage::Messaging, confidence: 0.70 },
        ServicePattern { pattern: "receive_message", provider: CloudProvider::AWS, service_name: "SQS", usage_type: ServiceUsage::Messaging, confidence: 0.70 },
        ServicePattern { pattern: "SnsClient", provider: CloudProvider::AWS, service_name: "SNS", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "sns_client", provider: CloudProvider::AWS, service_name: "SNS", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "publish", provider: CloudProvider::AWS, service_name: "SNS", usage_type: ServiceUsage::Messaging, confidence: 0.50 },
        ServicePattern { pattern: "Ec2Client", provider: CloudProvider::AWS, service_name: "EC2", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "ec2_client", provider: CloudProvider::AWS, service_name: "EC2", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "run_instances", provider: CloudProvider::AWS, service_name: "EC2", usage_type: ServiceUsage::Compute, confidence: 0.90 },
        ServicePattern { pattern: "describe_instances", provider: CloudProvider::AWS, service_name: "EC2", usage_type: ServiceUsage::Compute, confidence: 0.90 },
        ServicePattern { pattern: "ElastiCache", provider: CloudProvider::AWS, service_name: "ElastiCache", usage_type: ServiceUsage::Cache, confidence: 0.95 },
        ServicePattern { pattern: "CloudFront", provider: CloudProvider::AWS, service_name: "CloudFront", usage_type: ServiceUsage::CDN, confidence: 0.90 },
        ServicePattern { pattern: "SageMaker", provider: CloudProvider::AWS, service_name: "SageMaker", usage_type: ServiceUsage::AI, confidence: 0.90 },
        ServicePattern { pattern: "Bedrock", provider: CloudProvider::AWS, service_name: "Bedrock", usage_type: ServiceUsage::AI, confidence: 0.85 },
        ServicePattern { pattern: "EcsClient", provider: CloudProvider::AWS, service_name: "ECS", usage_type: ServiceUsage::Container, confidence: 0.95 },
        ServicePattern { pattern: "EksClient", provider: CloudProvider::AWS, service_name: "EKS", usage_type: ServiceUsage::Container, confidence: 0.95 },
        ServicePattern { pattern: "CloudWatch", provider: CloudProvider::AWS, service_name: "CloudWatch", usage_type: ServiceUsage::Monitoring, confidence: 0.90 },
        ServicePattern { pattern: "Cognito", provider: CloudProvider::AWS, service_name: "Cognito", usage_type: ServiceUsage::Auth, confidence: 0.90 },
        ServicePattern { pattern: "RdsClient", provider: CloudProvider::AWS, service_name: "RDS", usage_type: ServiceUsage::Database, confidence: 0.95 },
    ]
}

fn gcp_patterns() -> Vec<ServicePattern> {
    vec![
        ServicePattern { pattern: "storage.Client", provider: CloudProvider::GCP, service_name: "Cloud Storage", usage_type: ServiceUsage::Storage, confidence: 0.90 },
        ServicePattern { pattern: "storage::Client", provider: CloudProvider::GCP, service_name: "Cloud Storage", usage_type: ServiceUsage::Storage, confidence: 0.90 },
        ServicePattern { pattern: "google.cloud.storage", provider: CloudProvider::GCP, service_name: "Cloud Storage", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "bigquery", provider: CloudProvider::GCP, service_name: "BigQuery", usage_type: ServiceUsage::Database, confidence: 0.90 },
        ServicePattern { pattern: "BigQueryClient", provider: CloudProvider::GCP, service_name: "BigQuery", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "google.cloud.bigquery", provider: CloudProvider::GCP, service_name: "BigQuery", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "pubsub", provider: CloudProvider::GCP, service_name: "Pub/Sub", usage_type: ServiceUsage::Messaging, confidence: 0.85 },
        ServicePattern { pattern: "PublisherClient", provider: CloudProvider::GCP, service_name: "Pub/Sub", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "SubscriberClient", provider: CloudProvider::GCP, service_name: "Pub/Sub", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "google.cloud.pubsub", provider: CloudProvider::GCP, service_name: "Pub/Sub", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "cloud_run", provider: CloudProvider::GCP, service_name: "Cloud Run", usage_type: ServiceUsage::Serverless, confidence: 0.90 },
        ServicePattern { pattern: "CloudRunClient", provider: CloudProvider::GCP, service_name: "Cloud Run", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "google.cloud.run", provider: CloudProvider::GCP, service_name: "Cloud Run", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "cloud_functions", provider: CloudProvider::GCP, service_name: "Cloud Functions", usage_type: ServiceUsage::Serverless, confidence: 0.90 },
        ServicePattern { pattern: "ComputeClient", provider: CloudProvider::GCP, service_name: "Compute Engine", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "google.cloud.compute", provider: CloudProvider::GCP, service_name: "Compute Engine", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "FirestoreClient", provider: CloudProvider::GCP, service_name: "Firestore", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "google.cloud.firestore", provider: CloudProvider::GCP, service_name: "Firestore", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "VertexAI", provider: CloudProvider::GCP, service_name: "Vertex AI", usage_type: ServiceUsage::AI, confidence: 0.90 },
        ServicePattern { pattern: "google.cloud.aiplatform", provider: CloudProvider::GCP, service_name: "Vertex AI", usage_type: ServiceUsage::AI, confidence: 0.95 },
        ServicePattern { pattern: "GkeClient", provider: CloudProvider::GCP, service_name: "GKE", usage_type: ServiceUsage::Container, confidence: 0.95 },
        ServicePattern { pattern: "Memorystore", provider: CloudProvider::GCP, service_name: "Memorystore", usage_type: ServiceUsage::Cache, confidence: 0.90 },
        ServicePattern { pattern: "CloudCDN", provider: CloudProvider::GCP, service_name: "Cloud CDN", usage_type: ServiceUsage::CDN, confidence: 0.90 },
        ServicePattern { pattern: "Stackdriver", provider: CloudProvider::GCP, service_name: "Cloud Monitoring", usage_type: ServiceUsage::Monitoring, confidence: 0.85 },
        ServicePattern { pattern: "google.cloud.monitoring", provider: CloudProvider::GCP, service_name: "Cloud Monitoring", usage_type: ServiceUsage::Monitoring, confidence: 0.95 },
    ]
}

fn azure_patterns() -> Vec<ServicePattern> {
    vec![
        ServicePattern { pattern: "BlobServiceClient", provider: CloudProvider::Azure, service_name: "Blob Storage", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "BlobContainerClient", provider: CloudProvider::Azure, service_name: "Blob Storage", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "azure.storage.blob", provider: CloudProvider::Azure, service_name: "Blob Storage", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "azure_storage_blobs", provider: CloudProvider::Azure, service_name: "Blob Storage", usage_type: ServiceUsage::Storage, confidence: 0.95 },
        ServicePattern { pattern: "CosmosClient", provider: CloudProvider::Azure, service_name: "Cosmos DB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "cosmos_client", provider: CloudProvider::Azure, service_name: "Cosmos DB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "azure.cosmos", provider: CloudProvider::Azure, service_name: "Cosmos DB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "azure_cosmos", provider: CloudProvider::Azure, service_name: "Cosmos DB", usage_type: ServiceUsage::Database, confidence: 0.95 },
        ServicePattern { pattern: "FunctionApp", provider: CloudProvider::Azure, service_name: "Functions", usage_type: ServiceUsage::Serverless, confidence: 0.85 },
        ServicePattern { pattern: "azure.functions", provider: CloudProvider::Azure, service_name: "Functions", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "azure_functions", provider: CloudProvider::Azure, service_name: "Functions", usage_type: ServiceUsage::Serverless, confidence: 0.95 },
        ServicePattern { pattern: "ServiceBusClient", provider: CloudProvider::Azure, service_name: "Service Bus", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "azure.servicebus", provider: CloudProvider::Azure, service_name: "Service Bus", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "EventHubClient", provider: CloudProvider::Azure, service_name: "Event Hubs", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "azure.eventhub", provider: CloudProvider::Azure, service_name: "Event Hubs", usage_type: ServiceUsage::Messaging, confidence: 0.95 },
        ServicePattern { pattern: "VirtualMachineClient", provider: CloudProvider::Azure, service_name: "Virtual Machines", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "azure.mgmt.compute", provider: CloudProvider::Azure, service_name: "Virtual Machines", usage_type: ServiceUsage::Compute, confidence: 0.95 },
        ServicePattern { pattern: "ContainerInstanceClient", provider: CloudProvider::Azure, service_name: "Container Instances", usage_type: ServiceUsage::Container, confidence: 0.95 },
        ServicePattern { pattern: "AksClient", provider: CloudProvider::Azure, service_name: "AKS", usage_type: ServiceUsage::Container, confidence: 0.95 },
        ServicePattern { pattern: "AzureOpenAI", provider: CloudProvider::Azure, service_name: "Azure OpenAI", usage_type: ServiceUsage::AI, confidence: 0.90 },
        ServicePattern { pattern: "azure.ai.openai", provider: CloudProvider::Azure, service_name: "Azure OpenAI", usage_type: ServiceUsage::AI, confidence: 0.95 },
        ServicePattern { pattern: "AzureCacheForRedis", provider: CloudProvider::Azure, service_name: "Cache for Redis", usage_type: ServiceUsage::Cache, confidence: 0.90 },
        ServicePattern { pattern: "AzureCDN", provider: CloudProvider::Azure, service_name: "CDN", usage_type: ServiceUsage::CDN, confidence: 0.90 },
        ServicePattern { pattern: "MonitorClient", provider: CloudProvider::Azure, service_name: "Monitor", usage_type: ServiceUsage::Monitoring, confidence: 0.85 },
        ServicePattern { pattern: "azure.identity", provider: CloudProvider::Azure, service_name: "Active Directory", usage_type: ServiceUsage::Auth, confidence: 0.85 },
        ServicePattern { pattern: "SqlClient", provider: CloudProvider::Azure, service_name: "SQL Database", usage_type: ServiceUsage::Database, confidence: 0.80 },
    ]
}

impl CloudProviderManager {
    /// Create a new empty manager.
    pub fn new() -> Self {
        Self {
            detected_services: Vec::new(),
            policies: Vec::new(),
        }
    }

    /// Scan source code for cloud service usage patterns.
    ///
    /// Searches each line for known SDK patterns across all three providers
    /// and returns detected services with file location and confidence scores.
    pub fn scan_code(&mut self, source: &str, file_path: &str) -> Vec<DetectedService> {
        let all_patterns: Vec<ServicePattern> = aws_patterns()
            .into_iter()
            .chain(gcp_patterns())
            .chain(azure_patterns())
            .collect();

        let mut results: Vec<DetectedService> = Vec::new();
        let mut seen: HashMap<(String, String), usize> = HashMap::new();

        for (line_idx, line) in source.lines().enumerate() {
            for pat in &all_patterns {
                if line.contains(pat.pattern) {
                    let key = (pat.provider.to_string(), pat.service_name.to_string());
                    // Deduplicate: keep highest confidence per (provider, service) combo
                    if let Some(&existing_idx) = seen.get(&key) {
                        if results[existing_idx].confidence < pat.confidence {
                            results[existing_idx] = DetectedService {
                                service: CloudService {
                                    provider: pat.provider.clone(),
                                    service_name: pat.service_name.to_string(),
                                    usage_type: pat.usage_type.clone(),
                                    region: None,
                                },
                                source_file: file_path.to_string(),
                                line_number: line_idx + 1,
                                confidence: pat.confidence,
                            };
                        }
                    } else {
                        let idx = results.len();
                        results.push(DetectedService {
                            service: CloudService {
                                provider: pat.provider.clone(),
                                service_name: pat.service_name.to_string(),
                                usage_type: pat.usage_type.clone(),
                                region: None,
                            },
                            source_file: file_path.to_string(),
                            line_number: line_idx + 1,
                            confidence: pat.confidence,
                        });
                        seen.insert(key, idx);
                    }
                }
            }
        }

        self.detected_services.extend(results.clone());
        results
    }

    /// Generate a least-privilege IAM policy for detected services.
    pub fn generate_iam_policy(
        &self,
        provider: &CloudProvider,
        services: &[DetectedService],
    ) -> IamPolicy {
        let statements: Vec<PolicyStatement> = services
            .iter()
            .filter(|s| &s.service.provider == provider)
            .map(|s| {
                let (actions, resources) = Self::iam_actions_for_service(provider, &s.service.service_name);
                PolicyStatement {
                    effect: Effect::Allow,
                    actions,
                    resources,
                    conditions: Vec::new(),
                }
            })
            .collect();

        let name = format!("vibecody-{}-policy", provider.to_string().to_lowercase());
        IamPolicy {
            provider: provider.clone(),
            statements,
            name,
        }
    }

    /// Return least-privilege actions and resource ARNs for a given service.
    fn iam_actions_for_service(provider: &CloudProvider, service_name: &str) -> (Vec<String>, Vec<String>) {
        match provider {
            CloudProvider::AWS => Self::aws_iam_actions(service_name),
            CloudProvider::GCP => Self::gcp_iam_actions(service_name),
            CloudProvider::Azure => Self::azure_iam_actions(service_name),
        }
    }

    fn aws_iam_actions(service_name: &str) -> (Vec<String>, Vec<String>) {
        match service_name {
            "S3" => (
                vec![
                    "s3:GetObject".into(), "s3:PutObject".into(),
                    "s3:ListBucket".into(), "s3:DeleteObject".into(),
                ],
                vec!["arn:aws:s3:::*".into(), "arn:aws:s3:::*/*".into()],
            ),
            "DynamoDB" => (
                vec![
                    "dynamodb:GetItem".into(), "dynamodb:PutItem".into(),
                    "dynamodb:Query".into(), "dynamodb:Scan".into(),
                    "dynamodb:UpdateItem".into(), "dynamodb:DeleteItem".into(),
                ],
                vec!["arn:aws:dynamodb:*:*:table/*".into()],
            ),
            "Lambda" => (
                vec![
                    "lambda:InvokeFunction".into(), "lambda:GetFunction".into(),
                    "lambda:ListFunctions".into(),
                ],
                vec!["arn:aws:lambda:*:*:function:*".into()],
            ),
            "SQS" => (
                vec![
                    "sqs:SendMessage".into(), "sqs:ReceiveMessage".into(),
                    "sqs:DeleteMessage".into(), "sqs:GetQueueAttributes".into(),
                ],
                vec!["arn:aws:sqs:*:*:*".into()],
            ),
            "SNS" => (
                vec![
                    "sns:Publish".into(), "sns:Subscribe".into(),
                    "sns:ListTopics".into(),
                ],
                vec!["arn:aws:sns:*:*:*".into()],
            ),
            "EC2" => (
                vec![
                    "ec2:DescribeInstances".into(), "ec2:RunInstances".into(),
                    "ec2:StopInstances".into(), "ec2:TerminateInstances".into(),
                ],
                vec!["*".into()],
            ),
            "RDS" => (
                vec![
                    "rds:DescribeDBInstances".into(), "rds:CreateDBInstance".into(),
                ],
                vec!["arn:aws:rds:*:*:db:*".into()],
            ),
            "ECS" => (
                vec![
                    "ecs:RunTask".into(), "ecs:StopTask".into(),
                    "ecs:DescribeTasks".into(), "ecs:ListTasks".into(),
                ],
                vec!["arn:aws:ecs:*:*:*".into()],
            ),
            "EKS" => (
                vec![
                    "eks:DescribeCluster".into(), "eks:ListClusters".into(),
                ],
                vec!["arn:aws:eks:*:*:cluster/*".into()],
            ),
            "CloudWatch" => (
                vec![
                    "cloudwatch:PutMetricData".into(), "cloudwatch:GetMetricData".into(),
                    "logs:PutLogEvents".into(), "logs:CreateLogGroup".into(),
                ],
                vec!["*".into()],
            ),
            "Cognito" => (
                vec![
                    "cognito-idp:InitiateAuth".into(), "cognito-idp:SignUp".into(),
                    "cognito-idp:GetUser".into(),
                ],
                vec!["arn:aws:cognito-idp:*:*:userpool/*".into()],
            ),
            "SageMaker" => (
                vec![
                    "sagemaker:InvokeEndpoint".into(), "sagemaker:CreateEndpoint".into(),
                ],
                vec!["arn:aws:sagemaker:*:*:endpoint/*".into()],
            ),
            "Bedrock" => (
                vec![
                    "bedrock:InvokeModel".into(), "bedrock:ListFoundationModels".into(),
                ],
                vec!["arn:aws:bedrock:*:*:*".into()],
            ),
            "ElastiCache" => (
                vec![
                    "elasticache:DescribeCacheClusters".into(),
                    "elasticache:CreateCacheCluster".into(),
                ],
                vec!["arn:aws:elasticache:*:*:*".into()],
            ),
            "CloudFront" => (
                vec![
                    "cloudfront:GetDistribution".into(),
                    "cloudfront:CreateInvalidation".into(),
                ],
                vec!["arn:aws:cloudfront::*:distribution/*".into()],
            ),
            _ => (
                vec![format!("{}:*", service_name.to_lowercase())],
                vec!["*".into()],
            ),
        }
    }

    fn gcp_iam_actions(service_name: &str) -> (Vec<String>, Vec<String>) {
        match service_name {
            "Cloud Storage" => (
                vec![
                    "storage.objects.get".into(), "storage.objects.create".into(),
                    "storage.objects.delete".into(), "storage.buckets.list".into(),
                ],
                vec!["projects/_/buckets/*".into()],
            ),
            "BigQuery" => (
                vec![
                    "bigquery.jobs.create".into(), "bigquery.tables.getData".into(),
                    "bigquery.datasets.get".into(),
                ],
                vec!["projects/*/datasets/*".into()],
            ),
            "Pub/Sub" => (
                vec![
                    "pubsub.topics.publish".into(), "pubsub.subscriptions.consume".into(),
                    "pubsub.topics.list".into(),
                ],
                vec!["projects/*/topics/*".into(), "projects/*/subscriptions/*".into()],
            ),
            "Cloud Run" => (
                vec![
                    "run.services.get".into(), "run.services.create".into(),
                    "run.routes.invoke".into(),
                ],
                vec!["projects/*/locations/*/services/*".into()],
            ),
            "Cloud Functions" => (
                vec![
                    "cloudfunctions.functions.invoke".into(),
                    "cloudfunctions.functions.get".into(),
                ],
                vec!["projects/*/locations/*/functions/*".into()],
            ),
            "Compute Engine" => (
                vec![
                    "compute.instances.get".into(), "compute.instances.create".into(),
                    "compute.instances.delete".into(),
                ],
                vec!["projects/*/zones/*/instances/*".into()],
            ),
            "Firestore" => (
                vec![
                    "datastore.entities.get".into(), "datastore.entities.create".into(),
                ],
                vec!["projects/*/databases/*".into()],
            ),
            "Vertex AI" => (
                vec![
                    "aiplatform.endpoints.predict".into(),
                    "aiplatform.models.get".into(),
                ],
                vec!["projects/*/locations/*/endpoints/*".into()],
            ),
            _ => (
                vec![format!("{}.admin", service_name.to_lowercase().replace(' ', ""))],
                vec!["projects/*".into()],
            ),
        }
    }

    fn azure_iam_actions(service_name: &str) -> (Vec<String>, Vec<String>) {
        match service_name {
            "Blob Storage" => (
                vec![
                    "Microsoft.Storage/storageAccounts/blobServices/containers/blobs/read".into(),
                    "Microsoft.Storage/storageAccounts/blobServices/containers/blobs/write".into(),
                    "Microsoft.Storage/storageAccounts/blobServices/containers/blobs/delete".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.Storage/storageAccounts/*".into()],
            ),
            "Cosmos DB" => (
                vec![
                    "Microsoft.DocumentDB/databaseAccounts/readonlykeys/action".into(),
                    "Microsoft.DocumentDB/databaseAccounts/sqlDatabases/read".into(),
                    "Microsoft.DocumentDB/databaseAccounts/sqlDatabases/write".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.DocumentDB/databaseAccounts/*".into()],
            ),
            "Functions" => (
                vec![
                    "Microsoft.Web/sites/functions/read".into(),
                    "Microsoft.Web/sites/functions/action".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.Web/sites/*".into()],
            ),
            "Service Bus" => (
                vec![
                    "Microsoft.ServiceBus/namespaces/queues/send/action".into(),
                    "Microsoft.ServiceBus/namespaces/queues/receive/action".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.ServiceBus/namespaces/*".into()],
            ),
            "Event Hubs" => (
                vec![
                    "Microsoft.EventHub/namespaces/eventhubs/send/action".into(),
                    "Microsoft.EventHub/namespaces/eventhubs/receive/action".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.EventHub/namespaces/*".into()],
            ),
            "Virtual Machines" => (
                vec![
                    "Microsoft.Compute/virtualMachines/read".into(),
                    "Microsoft.Compute/virtualMachines/start/action".into(),
                    "Microsoft.Compute/virtualMachines/deallocate/action".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.Compute/virtualMachines/*".into()],
            ),
            "Azure OpenAI" => (
                vec![
                    "Microsoft.CognitiveServices/accounts/OpenAI/deployments/completions/action".into(),
                    "Microsoft.CognitiveServices/accounts/OpenAI/deployments/embeddings/action".into(),
                ],
                vec!["/subscriptions/*/resourceGroups/*/providers/Microsoft.CognitiveServices/accounts/*".into()],
            ),
            _ => (
                vec![format!("Microsoft.*/{}/read", service_name.to_lowercase().replace(' ', ""))],
                vec!["/subscriptions/*".into()],
            ),
        }
    }

    /// Generate an Infrastructure-as-Code template from detected services.
    pub fn generate_iac_template(
        &self,
        provider: &CloudProvider,
        format: IacFormat,
        services: &[DetectedService],
    ) -> IacTemplate {
        let provider_services: Vec<&DetectedService> = services
            .iter()
            .filter(|s| &s.service.provider == provider)
            .collect();

        let mut resources = Vec::new();
        let mut outputs = Vec::new();

        for svc in &provider_services {
            let (resource_type, logical_name, properties) =
                Self::iac_resource_for_service(provider, &format, &svc.service.service_name);
            resources.push(IacResource {
                resource_type,
                logical_name: logical_name.clone(),
                properties,
            });
            outputs.push(IacOutput {
                name: format!("{}_arn", logical_name.to_lowercase().replace('-', "_")),
                value: format!("${{{}.arn}}", logical_name),
                description: format!("{} resource identifier", svc.service.service_name),
            });
        }

        IacTemplate {
            provider: provider.clone(),
            format,
            resources,
            outputs,
        }
    }

    fn iac_resource_for_service(
        provider: &CloudProvider,
        format: &IacFormat,
        service_name: &str,
    ) -> (String, String, HashMap<String, String>) {
        let mut props = HashMap::new();

        match (provider, format) {
            (CloudProvider::AWS, IacFormat::Terraform) => {
                let (resource_type, logical) = match service_name {
                    "S3" => ("aws_s3_bucket", "app_bucket"),
                    "DynamoDB" => { props.insert("billing_mode".into(), "PAY_PER_REQUEST".into()); ("aws_dynamodb_table", "app_table") },
                    "Lambda" => { props.insert("runtime".into(), "provided.al2023".into()); props.insert("handler".into(), "bootstrap".into()); ("aws_lambda_function", "app_function") },
                    "SQS" => ("aws_sqs_queue", "app_queue"),
                    "SNS" => ("aws_sns_topic", "app_topic"),
                    "EC2" => { props.insert("instance_type".into(), "t3.micro".into()); props.insert("ami".into(), "ami-0c55b159cbfafe1f0".into()); ("aws_instance", "app_instance") },
                    "RDS" => { props.insert("engine".into(), "postgres".into()); props.insert("instance_class".into(), "db.t3.micro".into()); ("aws_db_instance", "app_db") },
                    "ECS" => ("aws_ecs_cluster", "app_cluster"),
                    _ => { let t = format!("aws_{}", service_name.to_lowercase().replace(' ', "_")); let l = format!("app_{}", service_name.to_lowercase().replace(' ', "_")); return (t, l, props); },
                };
                (resource_type.into(), logical.into(), props)
            }
            (CloudProvider::AWS, IacFormat::CloudFormation) => {
                let (resource_type, logical) = match service_name {
                    "S3" => ("AWS::S3::Bucket", "AppBucket"),
                    "DynamoDB" => { props.insert("BillingMode".into(), "PAY_PER_REQUEST".into()); ("AWS::DynamoDB::Table", "AppTable") },
                    "Lambda" => { props.insert("Runtime".into(), "provided.al2023".into()); props.insert("Handler".into(), "bootstrap".into()); ("AWS::Lambda::Function", "AppFunction") },
                    "SQS" => ("AWS::SQS::Queue", "AppQueue"),
                    "SNS" => ("AWS::SNS::Topic", "AppTopic"),
                    "EC2" => { props.insert("InstanceType".into(), "t3.micro".into()); ("AWS::EC2::Instance", "AppInstance") },
                    _ => { let t = format!("AWS::{}::Resource", service_name); let l = format!("App{}", service_name); return (t, l, props); },
                };
                (resource_type.into(), logical.into(), props)
            }
            (CloudProvider::AWS, IacFormat::Pulumi) => {
                let (resource_type, logical) = match service_name {
                    "S3" => ("aws.s3.Bucket", "appBucket"),
                    "DynamoDB" => { props.insert("billingMode".into(), "PAY_PER_REQUEST".into()); ("aws.dynamodb.Table", "appTable") },
                    "Lambda" => { props.insert("runtime".into(), "provided.al2023".into()); ("aws.lambda.Function", "appFunction") },
                    "SQS" => ("aws.sqs.Queue", "appQueue"),
                    "SNS" => ("aws.sns.Topic", "appTopic"),
                    _ => { let t = format!("aws.{}.Resource", service_name.to_lowercase()); let l = format!("app{}", service_name); return (t, l, props); },
                };
                (resource_type.into(), logical.into(), props)
            }
            (CloudProvider::GCP, IacFormat::Terraform) => {
                let (resource_type, logical) = match service_name {
                    "Cloud Storage" => ("google_storage_bucket", "app_bucket"),
                    "BigQuery" => ("google_bigquery_dataset", "app_dataset"),
                    "Pub/Sub" => ("google_pubsub_topic", "app_topic"),
                    "Cloud Run" => { props.insert("location".into(), "us-central1".into()); ("google_cloud_run_service", "app_service") },
                    "Cloud Functions" => { props.insert("runtime".into(), "nodejs20".into()); ("google_cloudfunctions_function", "app_function") },
                    "Compute Engine" => { props.insert("machine_type".into(), "e2-micro".into()); ("google_compute_instance", "app_instance") },
                    _ => { let t = format!("google_{}", service_name.to_lowercase().replace([' ', '/'], "_")); let l = format!("app_{}", service_name.to_lowercase().replace([' ', '/'], "_")); return (t, l, props); },
                };
                (resource_type.into(), logical.into(), props)
            }
            (CloudProvider::Azure, IacFormat::Terraform) => {
                let (resource_type, logical) = match service_name {
                    "Blob Storage" => ("azurerm_storage_account", "app_storage"),
                    "Cosmos DB" => { props.insert("offer_type".into(), "Standard".into()); ("azurerm_cosmosdb_account", "app_cosmosdb") },
                    "Functions" => { props.insert("os_type".into(), "Linux".into()); ("azurerm_function_app", "app_function") },
                    "Service Bus" => { props.insert("sku".into(), "Standard".into()); ("azurerm_servicebus_namespace", "app_servicebus") },
                    "Virtual Machines" => { props.insert("vm_size".into(), "Standard_B1s".into()); ("azurerm_virtual_machine", "app_vm") },
                    _ => { let t = format!("azurerm_{}", service_name.to_lowercase().replace(' ', "_")); let l = format!("app_{}", service_name.to_lowercase().replace(' ', "_")); return (t, l, props); },
                };
                (resource_type.into(), logical.into(), props)
            }
            _ => {
                let t = format!("{}_{}", provider.to_string().to_lowercase(), service_name.to_lowercase().replace(' ', "_"));
                let l = format!("app_{}", service_name.to_lowercase().replace(' ', "_"));
                (t, l, props)
            }
        }
    }

    /// Estimate monthly and yearly costs for detected services.
    pub fn estimate_costs(&self, services: &[DetectedService]) -> CostEstimate {
        let mut service_costs = Vec::new();
        let mut provider = CloudProvider::AWS;

        for svc in services {
            provider = svc.service.provider.clone();
            let (tier, monthly, notes) = Self::estimate_service_cost(&svc.service);
            service_costs.push(ServiceCost {
                service_name: svc.service.service_name.clone(),
                tier,
                monthly_usd: monthly,
                notes,
            });
        }

        let total_monthly: f64 = service_costs.iter().map(|c| c.monthly_usd).sum();
        let total_yearly = total_monthly * 12.0;

        CostEstimate {
            provider,
            services: service_costs,
            total_monthly_usd: total_monthly,
            total_yearly_usd: total_yearly,
        }
    }

    fn estimate_service_cost(service: &CloudService) -> (String, f64, String) {
        match (&service.provider, service.service_name.as_str()) {
            // AWS
            (CloudProvider::AWS, "S3") => ("Standard".into(), 23.0, "Estimated 1TB storage + requests".into()),
            (CloudProvider::AWS, "DynamoDB") => ("On-Demand".into(), 25.0, "Pay-per-request, ~1M reads/writes".into()),
            (CloudProvider::AWS, "Lambda") => ("Free Tier".into(), 0.0, "1M requests/month free".into()),
            (CloudProvider::AWS, "SQS") => ("Standard".into(), 0.40, "~1M messages/month".into()),
            (CloudProvider::AWS, "SNS") => ("Standard".into(), 0.50, "~1M notifications/month".into()),
            (CloudProvider::AWS, "EC2") => ("t3.micro".into(), 8.35, "On-demand Linux, us-east-1".into()),
            (CloudProvider::AWS, "RDS") => ("db.t3.micro".into(), 15.0, "Single-AZ PostgreSQL".into()),
            (CloudProvider::AWS, "ECS") => ("Fargate".into(), 36.0, "0.25 vCPU, 0.5GB, always-on".into()),
            (CloudProvider::AWS, "EKS") => ("Standard".into(), 73.0, "Cluster fee + t3.medium node".into()),
            (CloudProvider::AWS, "CloudWatch") => ("Basic".into(), 3.0, "10 custom metrics + 5GB logs".into()),
            (CloudProvider::AWS, "Cognito") => ("Free Tier".into(), 0.0, "First 50K MAU free".into()),
            (CloudProvider::AWS, "SageMaker") => ("ml.t3.medium".into(), 50.0, "Notebook + endpoint".into()),
            (CloudProvider::AWS, "Bedrock") => ("On-Demand".into(), 30.0, "~1M tokens/month".into()),
            (CloudProvider::AWS, "ElastiCache") => ("cache.t3.micro".into(), 12.0, "Redis single node".into()),
            (CloudProvider::AWS, "CloudFront") => ("Standard".into(), 10.0, "~100GB transfer/month".into()),
            // GCP
            (CloudProvider::GCP, "Cloud Storage") => ("Standard".into(), 20.0, "Estimated 1TB storage".into()),
            (CloudProvider::GCP, "BigQuery") => ("On-Demand".into(), 25.0, "~1TB queries/month".into()),
            (CloudProvider::GCP, "Pub/Sub") => ("Standard".into(), 0.40, "~1M messages/month".into()),
            (CloudProvider::GCP, "Cloud Run") => ("Pay-per-use".into(), 5.0, "Low-traffic service".into()),
            (CloudProvider::GCP, "Cloud Functions") => ("Free Tier".into(), 0.0, "2M invocations/month free".into()),
            (CloudProvider::GCP, "Compute Engine") => ("e2-micro".into(), 6.11, "Always-on, us-central1".into()),
            (CloudProvider::GCP, "Firestore") => ("Spark".into(), 0.0, "Free tier limits".into()),
            (CloudProvider::GCP, "Vertex AI") => ("Standard".into(), 50.0, "Prediction endpoint".into()),
            (CloudProvider::GCP, "GKE") => ("Autopilot".into(), 65.0, "Cluster management + compute".into()),
            (CloudProvider::GCP, "Memorystore") => ("Basic M1".into(), 36.0, "1GB Redis instance".into()),
            // Azure
            (CloudProvider::Azure, "Blob Storage") => ("Hot".into(), 20.0, "~1TB storage".into()),
            (CloudProvider::Azure, "Cosmos DB") => ("Serverless".into(), 25.0, "~1M RU/s".into()),
            (CloudProvider::Azure, "Functions") => ("Consumption".into(), 0.0, "1M executions/month free".into()),
            (CloudProvider::Azure, "Service Bus") => ("Standard".into(), 9.81, "Base + messaging".into()),
            (CloudProvider::Azure, "Event Hubs") => ("Basic".into(), 11.16, "1 throughput unit".into()),
            (CloudProvider::Azure, "Virtual Machines") => ("B1s".into(), 7.59, "1 vCPU, 1GB, Linux".into()),
            (CloudProvider::Azure, "Container Instances") => ("Standard".into(), 30.0, "1 vCPU, 1.5GB always-on".into()),
            (CloudProvider::Azure, "AKS") => ("Standard".into(), 73.0, "Cluster + B2s node".into()),
            (CloudProvider::Azure, "Azure OpenAI") => ("Pay-as-you-go".into(), 30.0, "~1M tokens/month".into()),
            (CloudProvider::Azure, "SQL Database") => ("Basic".into(), 4.90, "5 DTUs, 2GB".into()),
            (CloudProvider::Azure, "Active Directory") => ("Free".into(), 0.0, "Free tier".into()),
            _ => ("Unknown".into(), 10.0, "Estimated baseline".into()),
        }
    }

    /// Render an IaC template as Terraform HCL.
    pub fn render_terraform(template: &IacTemplate) -> String {
        let mut output = String::with_capacity(1024);

        // Provider block
        let provider_name = match template.provider {
            CloudProvider::AWS => "aws",
            CloudProvider::GCP => "google",
            CloudProvider::Azure => "azurerm",
        };
        output.push_str(&format!("terraform {{\n  required_providers {{\n    {} = {{\n      source = \"{}\"\n    }}\n  }}\n}}\n\n", provider_name, match template.provider {
            CloudProvider::AWS => "hashicorp/aws",
            CloudProvider::GCP => "hashicorp/google",
            CloudProvider::Azure => "hashicorp/azurerm",
        }));
        output.push_str(&format!("provider \"{}\" {{\n  region = \"us-east-1\"\n}}\n\n", provider_name));

        for resource in &template.resources {
            output.push_str(&format!("resource \"{}\" \"{}\" {{\n", resource.resource_type, resource.logical_name));
            for (key, value) in &resource.properties {
                output.push_str(&format!("  {} = \"{}\"\n", key, value));
            }
            output.push_str("}\n\n");
        }

        for out in &template.outputs {
            output.push_str(&format!("output \"{}\" {{\n  value       = {}\n  description = \"{}\"\n}}\n\n", out.name, out.value, out.description));
        }

        output
    }

    /// Render an IaC template as a CloudFormation YAML template.
    pub fn render_cloudformation(template: &IacTemplate) -> String {
        let mut output = String::with_capacity(1024);
        output.push_str("AWSTemplateFormatVersion: '2010-09-09'\n");
        output.push_str("Description: Generated by VibeCody\n\n");
        output.push_str("Resources:\n");

        for resource in &template.resources {
            output.push_str(&format!("  {}:\n", resource.logical_name));
            output.push_str(&format!("    Type: {}\n", resource.resource_type));
            if !resource.properties.is_empty() {
                output.push_str("    Properties:\n");
                for (key, value) in &resource.properties {
                    output.push_str(&format!("      {}: '{}'\n", key, value));
                }
            }
            output.push('\n');
        }

        if !template.outputs.is_empty() {
            output.push_str("Outputs:\n");
            for out in &template.outputs {
                output.push_str(&format!("  {}:\n", out.name));
                output.push_str(&format!("    Value: {}\n", out.value));
                output.push_str(&format!("    Description: {}\n", out.description));
            }
        }

        output
    }

    /// Render an IaC template as Pulumi TypeScript.
    pub fn render_pulumi_typescript(template: &IacTemplate) -> String {
        let mut output = String::with_capacity(1024);

        let import_pkg = match template.provider {
            CloudProvider::AWS => "@pulumi/aws",
            CloudProvider::GCP => "@pulumi/gcp",
            CloudProvider::Azure => "@pulumi/azure-native",
        };
        output.push_str("import * as pulumi from \"@pulumi/pulumi\";\n");
        output.push_str(&format!("import * as cloud from \"{}\";\n\n", import_pkg));

        for resource in &template.resources {
            let var_name = &resource.logical_name;
            output.push_str(&format!("const {} = new cloud.{}(\"{}\", {{\n", var_name, resource.resource_type, var_name));
            for (key, value) in &resource.properties {
                output.push_str(&format!("  {}: \"{}\",\n", key, value));
            }
            output.push_str("});\n\n");
        }

        for out in &template.outputs {
            output.push_str(&format!("export const {} = {}.id; // {}\n", out.name.replace('-', "_"), template.resources.first().map_or("resource", |r| r.logical_name.as_str()), out.description));
        }

        output
    }

    /// Serialize an IAM policy to a JSON string.
    pub fn policy_to_json(policy: &IamPolicy) -> String {
        let mut output = String::with_capacity(512);
        output.push_str("{\n");
        output.push_str("  \"Version\": \"2012-10-17\",\n");
        output.push_str(&format!("  \"PolicyName\": \"{}\",\n", policy.name));
        output.push_str(&format!("  \"Provider\": \"{}\",\n", policy.provider));
        output.push_str("  \"Statement\": [\n");

        for (i, stmt) in policy.statements.iter().enumerate() {
            output.push_str("    {\n");
            output.push_str(&format!("      \"Effect\": \"{}\",\n", stmt.effect));
            output.push_str("      \"Action\": [\n");
            for (j, action) in stmt.actions.iter().enumerate() {
                let comma = if j + 1 < stmt.actions.len() { "," } else { "" };
                output.push_str(&format!("        \"{}\"{}\n", action, comma));
            }
            output.push_str("      ],\n");
            output.push_str("      \"Resource\": [\n");
            for (j, resource) in stmt.resources.iter().enumerate() {
                let comma = if j + 1 < stmt.resources.len() { "," } else { "" };
                output.push_str(&format!("        \"{}\"{}\n", resource, comma));
            }
            output.push_str("      ]\n");
            let comma = if i + 1 < policy.statements.len() { "," } else { "" };
            output.push_str(&format!("    }}{}\n", comma));
        }

        output.push_str("  ]\n");
        output.push_str("}\n");
        output
    }

    /// List all supported services for a given provider.
    pub fn list_supported_services(provider: &CloudProvider) -> Vec<&'static str> {
        match provider {
            CloudProvider::AWS => vec![
                "S3", "DynamoDB", "Lambda", "SQS", "SNS", "EC2", "RDS",
                "ECS", "EKS", "CloudWatch", "Cognito", "SageMaker",
                "Bedrock", "ElastiCache", "CloudFront",
            ],
            CloudProvider::GCP => vec![
                "Cloud Storage", "BigQuery", "Pub/Sub", "Cloud Run",
                "Cloud Functions", "Compute Engine", "Firestore",
                "Vertex AI", "GKE", "Memorystore", "Cloud CDN",
                "Cloud Monitoring",
            ],
            CloudProvider::Azure => vec![
                "Blob Storage", "Cosmos DB", "Functions", "Service Bus",
                "Event Hubs", "Virtual Machines", "Container Instances",
                "AKS", "Azure OpenAI", "Cache for Redis", "CDN",
                "Monitor", "Active Directory", "SQL Database",
            ],
        }
    }

    /// Detect cloud providers from dependency files (Cargo.toml, package.json).
    pub fn detect_provider_from_deps(cargo_toml: &str, package_json: &str) -> Vec<CloudProvider> {
        let mut providers = Vec::new();

        let aws_cargo = ["aws-sdk-", "rusoto", "aws-config", "aws-types"];
        let gcp_cargo = ["google-cloud", "gcloud", "tonic-google"];
        let azure_cargo = ["azure_", "azure-", "azure_core", "azure_storage"];

        let aws_npm = ["@aws-sdk/", "aws-sdk", "aws-cdk", "aws-amplify", "amazon-cognito"];
        let gcp_npm = ["@google-cloud/", "firebase", "gcp-metadata", "@google-analytics"];
        let azure_npm = ["@azure/", "azure-", "msal-", "@microsoft/"];

        let has_aws_cargo = aws_cargo.iter().any(|p| cargo_toml.contains(p));
        let has_aws_npm = aws_npm.iter().any(|p| package_json.contains(p));
        if has_aws_cargo || has_aws_npm {
            providers.push(CloudProvider::AWS);
        }

        let has_gcp_cargo = gcp_cargo.iter().any(|p| cargo_toml.contains(p));
        let has_gcp_npm = gcp_npm.iter().any(|p| package_json.contains(p));
        if has_gcp_cargo || has_gcp_npm {
            providers.push(CloudProvider::GCP);
        }

        let has_azure_cargo = azure_cargo.iter().any(|p| cargo_toml.contains(p));
        let has_azure_npm = azure_npm.iter().any(|p| package_json.contains(p));
        if has_azure_cargo || has_azure_npm {
            providers.push(CloudProvider::Azure);
        }

        providers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- AWS Service Detection ---

    #[test]
    fn test_detect_aws_s3_client() {
        let mut mgr = CloudProviderManager::new();
        let code = "let client = s3_client.get_object(req).await;";
        let results = mgr.scan_code(code, "src/storage.rs");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].service.provider, CloudProvider::AWS);
        assert_eq!(results[0].service.service_name, "S3");
        assert_eq!(results[0].service.usage_type, ServiceUsage::Storage);
        assert_eq!(results[0].source_file, "src/storage.rs");
        assert_eq!(results[0].line_number, 1);
    }

    #[test]
    fn test_detect_aws_s3_class() {
        let mut mgr = CloudProviderManager::new();
        let code = "const client = new S3Client({ region: 'us-east-1' });";
        let results = mgr.scan_code(code, "src/upload.ts");
        assert!(results.iter().any(|r| r.service.service_name == "S3"));
    }

    #[test]
    fn test_detect_aws_dynamodb() {
        let mut mgr = CloudProviderManager::new();
        let code = "let db = DynamoDbClient::new(&config);";
        let results = mgr.scan_code(code, "src/db.rs");
        assert!(results.iter().any(|r| r.service.service_name == "DynamoDB"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Database));
    }

    #[test]
    fn test_detect_aws_dynamodb_by_name() {
        let mut mgr = CloudProviderManager::new();
        let code = "// Uses DynamoDB for persistence";
        let results = mgr.scan_code(code, "README.md");
        assert!(results.iter().any(|r| r.service.service_name == "DynamoDB"));
    }

    #[test]
    fn test_detect_aws_lambda() {
        let mut mgr = CloudProviderManager::new();
        let code = "let resp = lambda_client.invoke_function(req).await?;";
        let results = mgr.scan_code(code, "src/invoke.rs");
        assert!(results.iter().any(|r| r.service.service_name == "Lambda"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Serverless));
    }

    #[test]
    fn test_detect_aws_lambda_client_class() {
        let mut mgr = CloudProviderManager::new();
        let code = "const lambda = new LambdaClient({});";
        let results = mgr.scan_code(code, "handler.ts");
        assert!(results.iter().any(|r| r.service.service_name == "Lambda"));
    }

    #[test]
    fn test_detect_aws_sqs() {
        let mut mgr = CloudProviderManager::new();
        let code = "let sqs = SqsClient::new(&config);\nsqs.send_message(msg).await?;";
        let results = mgr.scan_code(code, "src/queue.rs");
        assert!(results.iter().any(|r| r.service.service_name == "SQS"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Messaging));
    }

    #[test]
    fn test_detect_aws_sns() {
        let mut mgr = CloudProviderManager::new();
        let code = "let sns = SnsClient::new(&config);";
        let results = mgr.scan_code(code, "src/notify.rs");
        assert!(results.iter().any(|r| r.service.service_name == "SNS"));
    }

    #[test]
    fn test_detect_aws_ec2() {
        let mut mgr = CloudProviderManager::new();
        let code = "let ec2 = Ec2Client::new(&config);\nec2.describe_instances().await?;";
        let results = mgr.scan_code(code, "src/infra.rs");
        assert!(results.iter().any(|r| r.service.service_name == "EC2"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Compute));
    }

    #[test]
    fn test_detect_aws_ec2_run_instances() {
        let mut mgr = CloudProviderManager::new();
        let code = "ec2.run_instances(req).await?;";
        let results = mgr.scan_code(code, "src/launch.rs");
        assert!(results.iter().any(|r| r.service.service_name == "EC2"));
    }

    // --- GCP Service Detection ---

    #[test]
    fn test_detect_gcp_storage() {
        let mut mgr = CloudProviderManager::new();
        let code = "from google.cloud.storage import Client";
        let results = mgr.scan_code(code, "upload.py");
        assert!(results.iter().any(|r| r.service.service_name == "Cloud Storage"));
        assert!(results.iter().any(|r| r.service.provider == CloudProvider::GCP));
    }

    #[test]
    fn test_detect_gcp_storage_client() {
        let mut mgr = CloudProviderManager::new();
        let code = "client = storage.Client()";
        let results = mgr.scan_code(code, "gcs.py");
        assert!(results.iter().any(|r| r.service.service_name == "Cloud Storage"));
    }

    #[test]
    fn test_detect_gcp_bigquery() {
        let mut mgr = CloudProviderManager::new();
        let code = "from google.cloud.bigquery import Client\nclient = BigQueryClient()";
        let results = mgr.scan_code(code, "analytics.py");
        assert!(results.iter().any(|r| r.service.service_name == "BigQuery"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Database));
    }

    #[test]
    fn test_detect_gcp_pubsub() {
        let mut mgr = CloudProviderManager::new();
        let code = "from google.cloud.pubsub import PublisherClient";
        let results = mgr.scan_code(code, "events.py");
        assert!(results.iter().any(|r| r.service.service_name == "Pub/Sub"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Messaging));
    }

    #[test]
    fn test_detect_gcp_cloud_run() {
        let mut mgr = CloudProviderManager::new();
        let code = "const client = new CloudRunClient();";
        let results = mgr.scan_code(code, "deploy.ts");
        assert!(results.iter().any(|r| r.service.service_name == "Cloud Run"));
    }

    #[test]
    fn test_detect_gcp_pubsub_subscriber() {
        let mut mgr = CloudProviderManager::new();
        let code = "let sub = SubscriberClient::new().await;";
        let results = mgr.scan_code(code, "sub.rs");
        assert!(results.iter().any(|r| r.service.service_name == "Pub/Sub"));
    }

    // --- Azure Service Detection ---

    #[test]
    fn test_detect_azure_blob_storage() {
        let mut mgr = CloudProviderManager::new();
        let code = "let client = BlobServiceClient::new(account, cred);";
        let results = mgr.scan_code(code, "src/blob.rs");
        assert!(results.iter().any(|r| r.service.service_name == "Blob Storage"));
        assert!(results.iter().any(|r| r.service.provider == CloudProvider::Azure));
    }

    #[test]
    fn test_detect_azure_blob_python() {
        let mut mgr = CloudProviderManager::new();
        let code = "from azure.storage.blob import BlobServiceClient";
        let results = mgr.scan_code(code, "blobs.py");
        assert!(results.iter().any(|r| r.service.service_name == "Blob Storage"));
    }

    #[test]
    fn test_detect_azure_cosmos_db() {
        let mut mgr = CloudProviderManager::new();
        let code = "let client = CosmosClient::new(endpoint, key);";
        let results = mgr.scan_code(code, "src/cosmos.rs");
        assert!(results.iter().any(|r| r.service.service_name == "Cosmos DB"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Database));
    }

    #[test]
    fn test_detect_azure_functions() {
        let mut mgr = CloudProviderManager::new();
        let code = "import azure.functions as func";
        let results = mgr.scan_code(code, "handler.py");
        assert!(results.iter().any(|r| r.service.service_name == "Functions"));
        assert!(results.iter().any(|r| r.service.usage_type == ServiceUsage::Serverless));
    }

    #[test]
    fn test_detect_azure_service_bus() {
        let mut mgr = CloudProviderManager::new();
        let code = "const client = new ServiceBusClient(connectionString);";
        let results = mgr.scan_code(code, "bus.ts");
        assert!(results.iter().any(|r| r.service.service_name == "Service Bus"));
    }

    // --- IAM Policy Generation ---

    #[test]
    fn test_iam_policy_aws_s3() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "src/lib.rs".into(),
            line_number: 10,
            confidence: 0.95,
        }];
        let policy = mgr.generate_iam_policy(&CloudProvider::AWS, &services);
        assert_eq!(policy.name, "vibecody-aws-policy");
        assert_eq!(policy.provider, CloudProvider::AWS);
        assert_eq!(policy.statements.len(), 1);
        assert_eq!(policy.statements[0].effect, Effect::Allow);
        assert!(policy.statements[0].actions.contains(&"s3:GetObject".to_string()));
        assert!(policy.statements[0].actions.contains(&"s3:PutObject".to_string()));
    }

    #[test]
    fn test_iam_policy_least_privilege_dynamodb() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "DynamoDB".into(), usage_type: ServiceUsage::Database, region: None },
            source_file: "src/db.rs".into(),
            line_number: 5,
            confidence: 0.95,
        }];
        let policy = mgr.generate_iam_policy(&CloudProvider::AWS, &services);
        assert_eq!(policy.statements.len(), 1);
        // Should have specific DynamoDB actions, not wildcard
        assert!(policy.statements[0].actions.iter().all(|a| a.starts_with("dynamodb:")));
        assert!(policy.statements[0].resources[0].contains("table"));
    }

    #[test]
    fn test_iam_policy_multiple_services() {
        let mgr = CloudProviderManager::new();
        let services = vec![
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
                source_file: "src/lib.rs".into(), line_number: 1, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "Lambda".into(), usage_type: ServiceUsage::Serverless, region: None },
                source_file: "src/lib.rs".into(), line_number: 5, confidence: 0.9,
            },
        ];
        let policy = mgr.generate_iam_policy(&CloudProvider::AWS, &services);
        assert_eq!(policy.statements.len(), 2);
    }

    #[test]
    fn test_iam_policy_filters_by_provider() {
        let mgr = CloudProviderManager::new();
        let services = vec![
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
                source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::GCP, service_name: "BigQuery".into(), usage_type: ServiceUsage::Database, region: None },
                source_file: "lib.rs".into(), line_number: 2, confidence: 0.9,
            },
        ];
        let aws_policy = mgr.generate_iam_policy(&CloudProvider::AWS, &services);
        assert_eq!(aws_policy.statements.len(), 1);
        let gcp_policy = mgr.generate_iam_policy(&CloudProvider::GCP, &services);
        assert_eq!(gcp_policy.statements.len(), 1);
    }

    #[test]
    fn test_iam_policy_gcp() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::GCP, service_name: "Cloud Storage".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "gcs.py".into(), line_number: 1, confidence: 0.9,
        }];
        let policy = mgr.generate_iam_policy(&CloudProvider::GCP, &services);
        assert!(policy.statements[0].actions.iter().any(|a| a.contains("storage")));
    }

    #[test]
    fn test_iam_policy_azure() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::Azure, service_name: "Blob Storage".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "blob.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let policy = mgr.generate_iam_policy(&CloudProvider::Azure, &services);
        assert!(policy.statements[0].actions.iter().any(|a| a.contains("Microsoft.Storage")));
    }

    #[test]
    fn test_policy_to_json() {
        let policy = IamPolicy {
            provider: CloudProvider::AWS,
            name: "test-policy".into(),
            statements: vec![PolicyStatement {
                effect: Effect::Allow,
                actions: vec!["s3:GetObject".into()],
                resources: vec!["arn:aws:s3:::*".into()],
                conditions: vec![],
            }],
        };
        let json = CloudProviderManager::policy_to_json(&policy);
        assert!(json.contains("\"Version\": \"2012-10-17\""));
        assert!(json.contains("\"Effect\": \"Allow\""));
        assert!(json.contains("s3:GetObject"));
        assert!(json.contains("test-policy"));
    }

    #[test]
    fn test_policy_to_json_deny() {
        let policy = IamPolicy {
            provider: CloudProvider::AWS,
            name: "deny-policy".into(),
            statements: vec![PolicyStatement {
                effect: Effect::Deny,
                actions: vec!["s3:DeleteBucket".into()],
                resources: vec!["*".into()],
                conditions: vec![],
            }],
        };
        let json = CloudProviderManager::policy_to_json(&policy);
        assert!(json.contains("\"Effect\": \"Deny\""));
    }

    // --- IaC / Terraform Rendering ---

    #[test]
    fn test_render_terraform_aws_s3() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Terraform, &services);
        let tf = CloudProviderManager::render_terraform(&template);
        assert!(tf.contains("hashicorp/aws"));
        assert!(tf.contains("aws_s3_bucket"));
        assert!(tf.contains("app_bucket"));
        assert!(tf.contains("output"));
    }

    #[test]
    fn test_render_terraform_aws_dynamodb() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "DynamoDB".into(), usage_type: ServiceUsage::Database, region: None },
            source_file: "db.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Terraform, &services);
        let tf = CloudProviderManager::render_terraform(&template);
        assert!(tf.contains("aws_dynamodb_table"));
        assert!(tf.contains("PAY_PER_REQUEST"));
    }

    #[test]
    fn test_render_terraform_gcp() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::GCP, service_name: "Cloud Storage".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "gcs.py".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::GCP, IacFormat::Terraform, &services);
        let tf = CloudProviderManager::render_terraform(&template);
        assert!(tf.contains("hashicorp/google"));
        assert!(tf.contains("google_storage_bucket"));
    }

    #[test]
    fn test_render_terraform_azure() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::Azure, service_name: "Blob Storage".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "blob.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::Azure, IacFormat::Terraform, &services);
        let tf = CloudProviderManager::render_terraform(&template);
        assert!(tf.contains("hashicorp/azurerm"));
        assert!(tf.contains("azurerm_storage_account"));
    }

    // --- CloudFormation Rendering ---

    #[test]
    fn test_render_cloudformation() {
        let mgr = CloudProviderManager::new();
        let services = vec![
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
                source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "Lambda".into(), usage_type: ServiceUsage::Serverless, region: None },
                source_file: "lib.rs".into(), line_number: 5, confidence: 0.9,
            },
        ];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::CloudFormation, &services);
        let cf = CloudProviderManager::render_cloudformation(&template);
        assert!(cf.contains("AWSTemplateFormatVersion"));
        assert!(cf.contains("AWS::S3::Bucket"));
        assert!(cf.contains("AWS::Lambda::Function"));
        assert!(cf.contains("Resources:"));
        assert!(cf.contains("Outputs:"));
    }

    #[test]
    fn test_render_cloudformation_properties() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "Lambda".into(), usage_type: ServiceUsage::Serverless, region: None },
            source_file: "fn.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::CloudFormation, &services);
        let cf = CloudProviderManager::render_cloudformation(&template);
        assert!(cf.contains("Runtime:"));
        assert!(cf.contains("provided.al2023"));
    }

    // --- Pulumi Rendering ---

    #[test]
    fn test_render_pulumi_typescript() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "SQS".into(), usage_type: ServiceUsage::Messaging, region: None },
            source_file: "queue.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Pulumi, &services);
        let pulumi = CloudProviderManager::render_pulumi_typescript(&template);
        assert!(pulumi.contains("@pulumi/aws"));
        assert!(pulumi.contains("aws.sqs.Queue"));
        assert!(pulumi.contains("export const"));
    }

    #[test]
    fn test_render_pulumi_gcp() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::GCP, service_name: "Cloud Storage".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "gcs.py".into(), line_number: 1, confidence: 0.9,
        }];
        let template = mgr.generate_iac_template(&CloudProvider::GCP, IacFormat::Pulumi, &services);
        let pulumi = CloudProviderManager::render_pulumi_typescript(&template);
        assert!(pulumi.contains("@pulumi/gcp"));
    }

    // --- Cost Estimation ---

    #[test]
    fn test_cost_estimate_aws_s3() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.provider, CloudProvider::AWS);
        assert_eq!(cost.services.len(), 1);
        assert_eq!(cost.services[0].service_name, "S3");
        assert!(cost.total_monthly_usd > 0.0);
        assert!((cost.total_yearly_usd - cost.total_monthly_usd * 12.0).abs() < 0.01);
    }

    #[test]
    fn test_cost_estimate_free_tier() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "Lambda".into(), usage_type: ServiceUsage::Serverless, region: None },
            source_file: "fn.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.total_monthly_usd, 0.0);
        assert_eq!(cost.total_yearly_usd, 0.0);
    }

    #[test]
    fn test_cost_estimate_multiple_services() {
        let mgr = CloudProviderManager::new();
        let services = vec![
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
                source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "EC2".into(), usage_type: ServiceUsage::Compute, region: None },
                source_file: "lib.rs".into(), line_number: 5, confidence: 0.9,
            },
        ];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.services.len(), 2);
        let expected_monthly = 23.0 + 8.35;
        assert!((cost.total_monthly_usd - expected_monthly).abs() < 0.01);
    }

    #[test]
    fn test_cost_estimate_gcp() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::GCP, service_name: "BigQuery".into(), usage_type: ServiceUsage::Database, region: None },
            source_file: "bq.py".into(), line_number: 1, confidence: 0.9,
        }];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.provider, CloudProvider::GCP);
        assert!(cost.total_monthly_usd > 0.0);
    }

    #[test]
    fn test_cost_estimate_azure() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::Azure, service_name: "Cosmos DB".into(), usage_type: ServiceUsage::Database, region: None },
            source_file: "cosmos.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.provider, CloudProvider::Azure);
        assert!(cost.services[0].monthly_usd > 0.0);
    }

    // --- Provider Detection from Dependencies ---

    #[test]
    fn test_detect_aws_from_cargo() {
        let cargo = "[dependencies]\naws-sdk-s3 = \"1.0\"";
        let providers = CloudProviderManager::detect_provider_from_deps(cargo, "");
        assert!(providers.contains(&CloudProvider::AWS));
    }

    #[test]
    fn test_detect_gcp_from_npm() {
        let npm = r#"{ "dependencies": { "@google-cloud/storage": "^7.0" } }"#;
        let providers = CloudProviderManager::detect_provider_from_deps("", npm);
        assert!(providers.contains(&CloudProvider::GCP));
    }

    #[test]
    fn test_detect_azure_from_cargo() {
        let cargo = "[dependencies]\nazure_storage = \"0.1\"";
        let providers = CloudProviderManager::detect_provider_from_deps(cargo, "");
        assert!(providers.contains(&CloudProvider::Azure));
    }

    #[test]
    fn test_detect_multiple_providers() {
        let cargo = "aws-sdk-s3 = \"1.0\"\nazure_core = \"0.1\"";
        let npm = r#"{ "dependencies": { "@google-cloud/bigquery": "^6.0" } }"#;
        let providers = CloudProviderManager::detect_provider_from_deps(cargo, npm);
        assert_eq!(providers.len(), 3);
        assert!(providers.contains(&CloudProvider::AWS));
        assert!(providers.contains(&CloudProvider::GCP));
        assert!(providers.contains(&CloudProvider::Azure));
    }

    #[test]
    fn test_detect_no_providers() {
        let providers = CloudProviderManager::detect_provider_from_deps("", "");
        assert!(providers.is_empty());
    }

    #[test]
    fn test_detect_firebase_as_gcp() {
        let npm = r#"{ "dependencies": { "firebase": "^10.0" } }"#;
        let providers = CloudProviderManager::detect_provider_from_deps("", npm);
        assert!(providers.contains(&CloudProvider::GCP));
    }

    #[test]
    fn test_detect_rusoto_as_aws() {
        let cargo = "rusoto_core = \"0.48\"";
        let providers = CloudProviderManager::detect_provider_from_deps(cargo, "");
        assert!(providers.contains(&CloudProvider::AWS));
    }

    // --- Edge Cases ---

    #[test]
    fn test_scan_empty_source() {
        let mut mgr = CloudProviderManager::new();
        let results = mgr.scan_code("", "empty.rs");
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_no_services_detected() {
        let mut mgr = CloudProviderManager::new();
        let code = "fn main() { println!(\"hello\"); }";
        let results = mgr.scan_code(code, "main.rs");
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_mixed_providers() {
        let mut mgr = CloudProviderManager::new();
        let code = "let s3 = S3Client::new(&config);\nlet bq = BigQueryClient::new();\nlet blob = BlobServiceClient::new(acct, cred);";
        let results = mgr.scan_code(code, "multi.rs");
        let providers: Vec<&CloudProvider> = results.iter().map(|r| &r.service.provider).collect();
        assert!(providers.contains(&&CloudProvider::AWS));
        assert!(providers.contains(&&CloudProvider::GCP));
        assert!(providers.contains(&&CloudProvider::Azure));
    }

    #[test]
    fn test_scan_deduplicates_services() {
        let mut mgr = CloudProviderManager::new();
        let code = "s3_client.get_object(req);\ns3_client.put_object(req);";
        let results = mgr.scan_code(code, "s3.rs");
        let s3_count = results.iter().filter(|r| r.service.service_name == "S3").count();
        assert_eq!(s3_count, 1, "S3 should only appear once (deduplicated)");
    }

    #[test]
    fn test_scan_updates_manager_state() {
        let mut mgr = CloudProviderManager::new();
        assert!(mgr.detected_services.is_empty());
        mgr.scan_code("let s3 = S3Client::new(&config);", "lib.rs");
        assert!(!mgr.detected_services.is_empty());
    }

    #[test]
    fn test_iac_template_empty_services() {
        let mgr = CloudProviderManager::new();
        let services: Vec<DetectedService> = vec![];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Terraform, &services);
        assert!(template.resources.is_empty());
        assert!(template.outputs.is_empty());
    }

    #[test]
    fn test_iam_policy_empty_services() {
        let mgr = CloudProviderManager::new();
        let services: Vec<DetectedService> = vec![];
        let policy = mgr.generate_iam_policy(&CloudProvider::AWS, &services);
        assert!(policy.statements.is_empty());
    }

    #[test]
    fn test_cost_estimate_empty_services() {
        let mgr = CloudProviderManager::new();
        let services: Vec<DetectedService> = vec![];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.total_monthly_usd, 0.0);
        assert_eq!(cost.total_yearly_usd, 0.0);
        assert!(cost.services.is_empty());
    }

    #[test]
    fn test_list_supported_services_aws() {
        let services = CloudProviderManager::list_supported_services(&CloudProvider::AWS);
        assert!(services.contains(&"S3"));
        assert!(services.contains(&"DynamoDB"));
        assert!(services.contains(&"Lambda"));
        assert!(services.contains(&"EC2"));
        assert!(services.len() >= 10);
    }

    #[test]
    fn test_list_supported_services_gcp() {
        let services = CloudProviderManager::list_supported_services(&CloudProvider::GCP);
        assert!(services.contains(&"Cloud Storage"));
        assert!(services.contains(&"BigQuery"));
        assert!(services.len() >= 8);
    }

    #[test]
    fn test_list_supported_services_azure() {
        let services = CloudProviderManager::list_supported_services(&CloudProvider::Azure);
        assert!(services.contains(&"Blob Storage"));
        assert!(services.contains(&"Cosmos DB"));
        assert!(services.len() >= 8);
    }

    #[test]
    fn test_cloud_provider_display() {
        assert_eq!(CloudProvider::AWS.to_string(), "AWS");
        assert_eq!(CloudProvider::GCP.to_string(), "GCP");
        assert_eq!(CloudProvider::Azure.to_string(), "Azure");
    }

    #[test]
    fn test_service_usage_display() {
        assert_eq!(ServiceUsage::Compute.to_string(), "Compute");
        assert_eq!(ServiceUsage::Serverless.to_string(), "Serverless");
        assert_eq!(ServiceUsage::AI.to_string(), "AI/ML");
    }

    #[test]
    fn test_effect_display() {
        assert_eq!(Effect::Allow.to_string(), "Allow");
        assert_eq!(Effect::Deny.to_string(), "Deny");
    }

    #[test]
    fn test_confidence_score_range() {
        let mut mgr = CloudProviderManager::new();
        let code = "let s3 = S3Client::new(&config);";
        let results = mgr.scan_code(code, "lib.rs");
        for r in &results {
            assert!(r.confidence >= 0.0 && r.confidence <= 1.0);
        }
    }

    #[test]
    fn test_line_number_accuracy() {
        let mut mgr = CloudProviderManager::new();
        let code = "// line 1\n// line 2\nlet db = DynamoDbClient::new(&config);";
        let results = mgr.scan_code(code, "db.rs");
        assert!(results.iter().any(|r| r.line_number == 3));
    }

    #[test]
    fn test_terraform_multiple_resources() {
        let mgr = CloudProviderManager::new();
        let services = vec![
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
                source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "SQS".into(), usage_type: ServiceUsage::Messaging, region: None },
                source_file: "lib.rs".into(), line_number: 5, confidence: 0.9,
            },
            DetectedService {
                service: CloudService { provider: CloudProvider::AWS, service_name: "Lambda".into(), usage_type: ServiceUsage::Serverless, region: None },
                source_file: "lib.rs".into(), line_number: 10, confidence: 0.9,
            },
        ];
        let template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Terraform, &services);
        assert_eq!(template.resources.len(), 3);
        assert_eq!(template.outputs.len(), 3);
        let tf = CloudProviderManager::render_terraform(&template);
        assert!(tf.contains("aws_s3_bucket"));
        assert!(tf.contains("aws_sqs_queue"));
        assert!(tf.contains("aws_lambda_function"));
    }

    #[test]
    fn test_unknown_service_cost_fallback() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "UnknownService".into(), usage_type: ServiceUsage::Network, region: None },
            source_file: "lib.rs".into(), line_number: 1, confidence: 0.5,
        }];
        let cost = mgr.estimate_costs(&services);
        assert_eq!(cost.services.len(), 1);
        assert_eq!(cost.services[0].tier, "Unknown");
        assert!(cost.total_monthly_usd > 0.0);
    }

    #[test]
    fn test_detect_aws_amplify_npm() {
        let npm = r#"{ "dependencies": { "aws-amplify": "^6.0" } }"#;
        let providers = CloudProviderManager::detect_provider_from_deps("", npm);
        assert!(providers.contains(&CloudProvider::AWS));
    }

    #[test]
    fn test_detect_azure_npm() {
        let npm = r#"{ "dependencies": { "@azure/storage-blob": "^12.0" } }"#;
        let providers = CloudProviderManager::detect_provider_from_deps("", npm);
        assert!(providers.contains(&CloudProvider::Azure));
    }

    #[test]
    fn test_manager_new_is_empty() {
        let mgr = CloudProviderManager::new();
        assert!(mgr.detected_services.is_empty());
        assert!(mgr.policies.is_empty());
    }

    #[test]
    fn test_iac_template_format_preserved() {
        let mgr = CloudProviderManager::new();
        let services = vec![DetectedService {
            service: CloudService { provider: CloudProvider::AWS, service_name: "S3".into(), usage_type: ServiceUsage::Storage, region: None },
            source_file: "lib.rs".into(), line_number: 1, confidence: 0.9,
        }];
        let tf_template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Terraform, &services);
        assert_eq!(tf_template.format, IacFormat::Terraform);
        let cf_template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::CloudFormation, &services);
        assert_eq!(cf_template.format, IacFormat::CloudFormation);
        let pu_template = mgr.generate_iac_template(&CloudProvider::AWS, IacFormat::Pulumi, &services);
        assert_eq!(pu_template.format, IacFormat::Pulumi);
    }
}
