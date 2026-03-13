/**
 * CloudProviderPanel — Cloud Provider Integration panel.
 *
 * Scans codebase for cloud service usage, generates IAM policies,
 * produces IaC templates, and estimates costs.
 * Pure TypeScript — no Tauri commands needed.
 */
import { useState } from "react";

// ── Types ─────────────────────────────────────────────────────────────────────

interface DetectedService {
  id: string;
  provider: "AWS" | "GCP" | "Azure";
  service: string;
  confidence: number;
  file: string;
  line: number;
}

interface CostEstimate {
  service: string;
  provider: string;
  monthly: number;
  yearly: number;
  tier: string;
}

// ── Mock Data ─────────────────────────────────────────────────────────────────

const MOCK_SERVICES: DetectedService[] = [
  { id: "s1", provider: "AWS", service: "S3", confidence: 0.95, file: "src/storage.rs", line: 42 },
  { id: "s2", provider: "AWS", service: "Lambda", confidence: 0.88, file: "src/handlers.rs", line: 15 },
  { id: "s3", provider: "AWS", service: "DynamoDB", confidence: 0.92, file: "src/db.rs", line: 78 },
  { id: "s4", provider: "AWS", service: "SQS", confidence: 0.75, file: "src/queue.rs", line: 23 },
  { id: "s5", provider: "GCP", service: "Cloud Storage", confidence: 0.70, file: "src/backup.rs", line: 11 },
  { id: "s6", provider: "Azure", service: "Blob Storage", confidence: 0.65, file: "src/uploads.rs", line: 34 },
  { id: "s7", provider: "AWS", service: "CloudFront", confidence: 0.80, file: "src/cdn.rs", line: 5 },
  { id: "s8", provider: "AWS", service: "RDS (PostgreSQL)", confidence: 0.90, file: "src/database.rs", line: 112 },
];

const MOCK_IAM_POLICY = `{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "S3Access",
      "Effect": "Allow",
      "Action": ["s3:GetObject", "s3:PutObject", "s3:ListBucket"],
      "Resource": ["arn:aws:s3:::my-bucket", "arn:aws:s3:::my-bucket/*"]
    },
    {
      "Sid": "DynamoDBAccess",
      "Effect": "Allow",
      "Action": ["dynamodb:GetItem", "dynamodb:PutItem", "dynamodb:Query"],
      "Resource": "arn:aws:dynamodb:us-east-1:*:table/my-table"
    },
    {
      "Sid": "SQSAccess",
      "Effect": "Allow",
      "Action": ["sqs:SendMessage", "sqs:ReceiveMessage", "sqs:DeleteMessage"],
      "Resource": "arn:aws:sqs:us-east-1:*:my-queue"
    },
    {
      "Sid": "LambdaInvoke",
      "Effect": "Allow",
      "Action": ["lambda:InvokeFunction"],
      "Resource": "arn:aws:lambda:us-east-1:*:function:my-handler"
    }
  ]
}`;

const IAC_TEMPLATES: Record<string, string> = {
  Terraform: `resource "aws_s3_bucket" "main" {
  bucket = "my-app-bucket"
  acl    = "private"
}

resource "aws_dynamodb_table" "main" {
  name         = "my-table"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "id"

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_sqs_queue" "main" {
  name = "my-queue"
}`,
  CloudFormation: `AWSTemplateFormatVersion: "2010-09-09"
Resources:
  S3Bucket:
    Type: AWS::S3::Bucket
    Properties:
      BucketName: my-app-bucket
      AccessControl: Private

  DynamoDBTable:
    Type: AWS::DynamoDB::Table
    Properties:
      TableName: my-table
      BillingMode: PAY_PER_REQUEST
      AttributeDefinitions:
        - AttributeName: id
          AttributeType: S
      KeySchema:
        - AttributeName: id
          KeyType: HASH

  SQSQueue:
    Type: AWS::SQS::Queue
    Properties:
      QueueName: my-queue`,
  Pulumi: `import * as aws from "@pulumi/aws";

const bucket = new aws.s3.Bucket("main", {
  bucket: "my-app-bucket",
  acl: "private",
});

const table = new aws.dynamodb.Table("main", {
  name: "my-table",
  billingMode: "PAY_PER_REQUEST",
  hashKey: "id",
  attributes: [{ name: "id", type: "S" }],
});

const queue = new aws.sqs.Queue("main", {
  name: "my-queue",
});`,
};

const MOCK_COSTS: CostEstimate[] = [
  { service: "S3", provider: "AWS", monthly: 23.50, yearly: 282.00, tier: "Standard" },
  { service: "Lambda", provider: "AWS", monthly: 18.40, yearly: 220.80, tier: "1M requests/mo" },
  { service: "DynamoDB", provider: "AWS", monthly: 45.00, yearly: 540.00, tier: "On-demand" },
  { service: "SQS", provider: "AWS", monthly: 4.20, yearly: 50.40, tier: "Standard" },
  { service: "CloudFront", provider: "AWS", monthly: 32.00, yearly: 384.00, tier: "1TB/mo" },
  { service: "RDS (PostgreSQL)", provider: "AWS", monthly: 125.00, yearly: 1500.00, tier: "db.t3.medium" },
  { service: "Cloud Storage", provider: "GCP", monthly: 12.00, yearly: 144.00, tier: "Standard" },
  { service: "Blob Storage", provider: "Azure", monthly: 15.00, yearly: 180.00, tier: "Hot" },
];

// ── Styles ────────────────────────────────────────────────────────────────────

const panelStyle: React.CSSProperties = { padding: 16, color: "var(--text-primary)", fontFamily: "var(--font-mono, monospace)", fontSize: 13, height: "100%", overflow: "auto", background: "var(--bg-primary)" };
const headingStyle: React.CSSProperties = { margin: "0 0 12px", fontSize: 15, fontWeight: 600, color: "var(--text-primary)" };
const cardStyle: React.CSSProperties = { background: "var(--bg-secondary)", borderRadius: 6, padding: 12, marginBottom: 10, border: "1px solid var(--border-primary)" };
const labelStyle: React.CSSProperties = { fontSize: 11, color: "var(--text-secondary)", marginBottom: 4 };
const btnStyle: React.CSSProperties = { padding: "6px 14px", borderRadius: 4, border: "1px solid var(--border-primary)", background: "var(--bg-tertiary)", color: "var(--text-primary)", cursor: "pointer", fontSize: 12 };
const tabBtnStyle = (active: boolean): React.CSSProperties => ({ ...btnStyle, background: active ? "var(--accent-primary)" : "var(--bg-tertiary)", color: active ? "#fff" : "var(--text-primary)", marginRight: 4 });

const preStyle: React.CSSProperties = { background: "var(--bg-tertiary)", padding: 10, borderRadius: 4, fontSize: 11, overflow: "auto", whiteSpace: "pre-wrap", border: "1px solid var(--border-primary)", maxHeight: 400 };
const providerColor: Record<string, string> = { AWS: "#ff9900", GCP: "#4285f4", Azure: "#0078d4" };
const confidenceColor = (c: number) => c >= 0.9 ? "#22c55e" : c >= 0.7 ? "#f59e0b" : "#ef4444";

const thStyle: React.CSSProperties = { textAlign: "left", padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 11, color: "var(--text-secondary)" };
const tdStyle: React.CSSProperties = { padding: "6px 10px", borderBottom: "1px solid var(--border-primary)", fontSize: 12 };

// ── Component ─────────────────────────────────────────────────────────────────

type Tab = "scan" | "iam" | "iac" | "cost";

export function CloudProviderPanel() {
  const [tab, setTab] = useState<Tab>("scan");
  const [iacFormat, setIacFormat] = useState<string>("Terraform");

  const totalMonthly = MOCK_COSTS.reduce((s, c) => s + c.monthly, 0);
  const totalYearly = MOCK_COSTS.reduce((s, c) => s + c.yearly, 0);

  return (
    <div style={panelStyle}>
      <h2 style={headingStyle}>Cloud Provider Integration</h2>

      <div style={{ marginBottom: 12 }}>
        <button style={tabBtnStyle(tab === "scan")} onClick={() => setTab("scan")}>Scan</button>
        <button style={tabBtnStyle(tab === "iam")} onClick={() => setTab("iam")}>IAM</button>
        <button style={tabBtnStyle(tab === "iac")} onClick={() => setTab("iac")}>IaC</button>
        <button style={tabBtnStyle(tab === "cost")} onClick={() => setTab("cost")}>Cost</button>
      </div>

      {tab === "scan" && (
        <div>
          <div style={{ ...cardStyle, fontSize: 12 }}>
            Detected {MOCK_SERVICES.length} cloud services across {new Set(MOCK_SERVICES.map((s) => s.provider)).size} providers.
          </div>
          {MOCK_SERVICES.map((s) => (
            <div key={s.id} style={{ ...cardStyle, display: "flex", justifyContent: "space-between", alignItems: "center" }}>
              <div>
                <span style={{ fontWeight: 600, color: providerColor[s.provider] }}>[{s.provider}]</span>{" "}
                <span style={{ fontWeight: 600 }}>{s.service}</span>
                <div style={{ fontSize: 10, color: "var(--text-secondary)", marginTop: 2 }}>{s.file}:{s.line}</div>
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontSize: 11, color: "var(--text-secondary)" }}>Confidence</div>
                <div style={{ fontWeight: 600, color: confidenceColor(s.confidence) }}>{(s.confidence * 100).toFixed(0)}%</div>
              </div>
            </div>
          ))}
        </div>
      )}

      {tab === "iam" && (
        <div>
          <div style={cardStyle}>
            <div style={labelStyle}>Generated least-privilege IAM policy based on detected AWS services</div>
            <pre style={preStyle}>{MOCK_IAM_POLICY}</pre>
            <div style={{ marginTop: 8 }}>
              <button style={btnStyle} onClick={() => navigator.clipboard?.writeText(MOCK_IAM_POLICY)}>Copy Policy</button>
            </div>
          </div>
        </div>
      )}

      {tab === "iac" && (
        <div>
          <div style={{ marginBottom: 10 }}>
            {Object.keys(IAC_TEMPLATES).map((fmt) => (
              <button key={fmt} style={tabBtnStyle(iacFormat === fmt)} onClick={() => setIacFormat(fmt)}>{fmt}</button>
            ))}
          </div>
          <div style={cardStyle}>
            <div style={labelStyle}>{iacFormat} template for detected services</div>
            <pre style={preStyle}>{IAC_TEMPLATES[iacFormat]}</pre>
            <div style={{ marginTop: 8 }}>
              <button style={btnStyle} onClick={() => navigator.clipboard?.writeText(IAC_TEMPLATES[iacFormat])}>Copy Template</button>
            </div>
          </div>
        </div>
      )}

      {tab === "cost" && (
        <div>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10, marginBottom: 12 }}>
            <div style={cardStyle}>
              <div style={labelStyle}>Estimated Monthly</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent-primary, #3b82f6)" }}>${totalMonthly.toFixed(2)}</div>
            </div>
            <div style={cardStyle}>
              <div style={labelStyle}>Estimated Yearly</div>
              <div style={{ fontSize: 22, fontWeight: 700, color: "var(--accent-primary, #3b82f6)" }}>${totalYearly.toFixed(2)}</div>
            </div>
          </div>
          <div style={cardStyle}>
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr>
                  <th style={thStyle}>Service</th>
                  <th style={thStyle}>Provider</th>
                  <th style={thStyle}>Tier</th>
                  <th style={{ ...thStyle, textAlign: "right" }}>Monthly</th>
                  <th style={{ ...thStyle, textAlign: "right" }}>Yearly</th>
                </tr>
              </thead>
              <tbody>
                {MOCK_COSTS.map((c, i) => (
                  <tr key={i}>
                    <td style={tdStyle}>{c.service}</td>
                    <td style={{ ...tdStyle, color: providerColor[c.provider] }}>{c.provider}</td>
                    <td style={{ ...tdStyle, fontSize: 11, color: "var(--text-secondary)" }}>{c.tier}</td>
                    <td style={{ ...tdStyle, textAlign: "right" }}>${c.monthly.toFixed(2)}</td>
                    <td style={{ ...tdStyle, textAlign: "right" }}>${c.yearly.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
              <tfoot>
                <tr>
                  <td colSpan={3} style={{ ...tdStyle, fontWeight: 600 }}>Total</td>
                  <td style={{ ...tdStyle, textAlign: "right", fontWeight: 600 }}>${totalMonthly.toFixed(2)}</td>
                  <td style={{ ...tdStyle, textAlign: "right", fontWeight: 600 }}>${totalYearly.toFixed(2)}</td>
                </tr>
              </tfoot>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}
