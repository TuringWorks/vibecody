---
triggers: ["IBM Cloud", "ibm cloud", "code engine", "cloudant", "ibm kubernetes", "ibm cloud functions", "ibm object storage", "ibm watson"]
tools_allowed: ["read_file", "write_file", "bash"]
category: cloud-ibm
---

# IBM Cloud

When working with IBM Cloud:

1. Install and authenticate the IBM Cloud CLI with `ibmcloud login --sso` for federated accounts or `ibmcloud login --apikey $API_KEY` for automation, then target a resource group and region with `ibmcloud target -r us-south -g default` before provisioning resources.
2. Deploy containerized workloads to Code Engine with `ibmcloud ce project create --name myproject && ibmcloud ce app create --name myapp --image icr.io/myns/myapp:latest --port 8080 --min-scale 0 --max-scale 10` for automatic scale-to-zero and pay-per-use billing.
3. Create Cloud Functions actions for event-driven compute: `ibmcloud fn action create hello hello.js --kind nodejs:18 --web true` and chain them into sequences with `ibmcloud fn action create myseq --sequence action1,action2` for composable serverless pipelines.
4. Provision Cloudant NoSQL databases with `ibmcloud resource service-instance-create mydb cloudantnosqldb lite us-south` and use the Node.js SDK: `const { CloudantV1 } = require('@ibm-cloud/cloudant'); const client = CloudantV1.newInstance(); const result = await client.postDocument({ db: 'mydb', document: doc });`.
5. Set up IBM Kubernetes Service clusters with `ibmcloud ks cluster create vpc-gen2 --name prod --zone us-south-1 --vpc-id $VPC --subnet-id $SUBNET --flavor bx2.4x16 --workers 3` and manage with `ibmcloud ks cluster config --cluster prod` to configure kubectl.
6. Use IBM Cloud Object Storage (COS) for scalable storage: create buckets via `ibmcloud cos bucket-create --bucket mydata --ibm-service-instance-id $COS_ID --class smart` and access programmatically with the S3-compatible SDK using HMAC credentials for cross-platform compatibility.
7. Manage IAM access with service IDs and API keys: `ibmcloud iam service-id-create myservice` then `ibmcloud iam service-api-key-create mykey myservice` and assign granular access policies with `ibmcloud iam service-policy-create myservice --roles Writer --service-name cloudantnosqldb`.
8. Use Terraform with the IBM Cloud provider by configuring `provider "ibm" { ibmcloud_api_key = var.api_key; region = "us-south" }` or use IBM Schematics for managed Terraform: `ibmcloud schematics workspace new --file workspace.json` to run plans and applies from the cloud.
9. Integrate Watson AI services (Natural Language Understanding, Speech to Text, Assistant) via the Python SDK: `from ibm_watson import NaturalLanguageUnderstandingV1; nlu = NaturalLanguageUnderstandingV1(version='2022-04-07', authenticator=authenticator); response = nlu.analyze(text=text, features=features).get_result()`.
10. Configure Db2 on Cloud for relational workloads with `ibmcloud resource service-instance-create mydb2 dashdb-for-transactions free us-south` and connect using the `ibm_db` driver: `import ibm_db; conn = ibm_db.connect(dsn, '', '')` with connection pooling for production use.
11. Optimize costs by using Lite (free) plan tiers for development, setting spending notifications with `ibmcloud billing account-usage`, leveraging reserved capacity for predictable workloads, and suspending idle Code Engine applications that auto-scale to zero.
12. Secure workloads by enabling service-to-service authorization with `ibmcloud iam authorization-policy-create`, using private service endpoints (`--service-endpoints private`), encrypting data with Key Protect (`ibmcloud kp key create mykey`), and enabling Activity Tracker for audit logging.
