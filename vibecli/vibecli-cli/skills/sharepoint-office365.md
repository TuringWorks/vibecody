---
triggers: ["SharePoint", "Office 365", "Microsoft 365", "Power Platform", "Power Automate", "Power Apps", "Teams administration"]
tools_allowed: ["read_file", "write_file", "bash"]
category: enterprise
---

# SharePoint and Microsoft 365

When working with SharePoint, Microsoft 365, and the Power Platform:

1. Design SharePoint site architecture using hub sites for enterprise-wide navigation and consistent branding, team sites for departmental collaboration with Microsoft 365 Group integration, and communication sites for broadcasting news and showcasing content — plan your information architecture with a clear taxonomy, limit site depth to avoid navigation confusion, and use hub site associations to create logical groupings without rigid hierarchy.

2. Implement document management with structured metadata by creating content types that define document schemas, applying site columns for consistent metadata across libraries, configuring views with filtering and grouping by metadata columns, using managed metadata (term store) for controlled vocabularies, enabling versioning and check-out policies based on content sensitivity, and training users to tag documents consistently rather than relying solely on folder structures.

3. Build Power Automate workflows by starting with templates for common scenarios (approvals, notifications, data synchronization), designing flows with proper error handling and retry logic, using environment variables for connection references across dev/test/prod, implementing approval workflows with parallel and sequential patterns, leveraging the Common Data Service connector for complex business logic, and monitoring flow runs through the Power Platform admin center.

4. Create Power Apps low-code applications by choosing between canvas apps (pixel-perfect control) and model-driven apps (data-first with forms and views), connecting to SharePoint lists, Dataverse, or external data sources, implementing responsive layouts for mobile and tablet use, using component libraries for reusable UI elements, applying role-based visibility with User() function and security roles, and following the Power Apps coding standards for formula readability.

5. Build Power BI dashboards by connecting to organizational data sources through dataflows and datasets, designing reports with a clear visual hierarchy and storytelling approach, implementing row-level security for data access control, using DAX measures for calculated business metrics, publishing to Power BI workspaces aligned with your governance model, scheduling data refreshes through the Power BI gateway, and embedding reports in SharePoint pages and Teams tabs.

6. Govern Microsoft Teams by establishing a Teams creation policy (who can create, naming conventions, expiration), defining channel structure standards (general, project-specific, social), configuring external access and guest policies, managing app permissions and which third-party apps are allowed, implementing retention policies for compliance, archiving inactive teams rather than deleting them, and training users on when to use Teams channels versus SharePoint document libraries versus email.

7. Administer Microsoft 365 by managing user licensing and group-based license assignment, configuring multi-factor authentication and conditional access policies, monitoring service health and incident communications, managing Exchange Online mailbox policies and distribution groups, setting up OneDrive storage quotas and sharing policies, reviewing security and compliance scores regularly, and using the Microsoft 365 admin center reports for adoption insights.

8. Implement compliance controls using Data Loss Prevention (DLP) policies to detect and protect sensitive information, configuring retention policies and labels for records management, setting up eDiscovery cases for legal holds and content searches, enabling audit logging and reviewing the unified audit log, configuring sensitivity labels for document classification and encryption, implementing information barriers where required by regulation, and aligning policies with organizational compliance frameworks.

9. Develop SharePoint Framework (SPFx) solutions by setting up the development environment with Node.js and Yeoman generator, building web parts with React for modern page experiences, creating extensions (application customizers, field customizers, command sets) for platform-level customization, using the PnP JS library for simplified SharePoint API interaction, deploying solutions through the app catalog with tenant-wide or site-scoped installation, and testing across SharePoint Online environments before production deployment.

10. Plan and execute SharePoint migration by inventorying content from legacy platforms (file shares, on-premises SharePoint, other systems), assessing content for relevance and applying disposition rules before migrating, choosing migration tools (SharePoint Migration Tool, third-party solutions like ShareGate or AvePoint), mapping source permissions to target SharePoint groups and access levels, running pilot migrations with user validation, scheduling large migrations during off-peak hours, and verifying content integrity post-migration.

11. Configure permission management by using SharePoint groups aligned with organizational roles, inheriting permissions from parent sites where possible to reduce complexity, breaking inheritance only when business requirements demand it, avoiding granting permissions to individual users in favor of groups, reviewing sharing links and external access regularly, using access reviews in Azure AD for periodic certification, and documenting the permission model for each major site collection.

12. Integrate with third-party systems by using Power Automate premium connectors for SaaS applications, building custom connectors for REST APIs not covered by standard connectors, leveraging Azure Logic Apps for enterprise-grade integration patterns, using Microsoft Graph API for programmatic access to Microsoft 365 data, implementing webhooks for event-driven architectures, connecting to on-premises systems through the on-premises data gateway, and maintaining an integration inventory with data flow documentation.
