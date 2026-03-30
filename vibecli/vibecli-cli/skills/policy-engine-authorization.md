# Policy-as-Code Authorization Engine

Cerbos-inspired authorization engine providing RBAC and ABAC policy evaluation, derived roles, policy testing, audit trails, and conflict detection.

## Features
- **RBAC + ABAC**: Role-based and attribute-based access control
- **Derived Roles**: Dynamic role assignment based on conditions
- **14 Condition Operators**: Eq, NotEq, In, NotIn, Contains, StartsWith, EndsWith, Regex, Gt, Lt, Gte, Lte, And, Or, Not
- **Policy Testing**: Test suites with expected effects (pass/fail reporting)
- **YAML Policies**: Parse and generate YAML policy definitions
- **Audit Trail**: Full request/result/policy-chain logging
- **Conflict Detection**: Identifies overlapping rules with different effects
- **Coverage Analysis**: Reports which resources/actions are covered
- **Unused Rule Detection**: Finds rules never matched in audit log
- **Batch Evaluation**: Evaluate multiple requests efficiently
- **Policy Templates**: Generate starter policies for any resource

## Policy Types
- ResourcePolicy — policies attached to resources
- PrincipalPolicy — policies attached to principals
- DerivedRoles — dynamic role definitions
- ExportVariables — shared variables across policies

## Commands
- `/policy add <yaml>` — Add a policy
- `/policy check <principal> <resource> <action>` — Evaluate authorization
- `/policy test <suite>` — Run policy test suite
- `/policy list` — List all policies
- `/policy conflicts` — Detect policy conflicts
- `/policy coverage` — Show coverage report
- `/policy audit` — View audit trail
- `/policy template <resource>` — Generate starter policy

## Example YAML Policy
```yaml
apiVersion: api.cerbos.dev/v1
resourcePolicy:
  resource: "document"
  version: "1.0"
  rules:
    - actions: ["read", "list"]
      effect: ALLOW
      roles: ["viewer", "editor", "admin"]
    - actions: ["edit", "delete"]
      effect: ALLOW
      roles: ["editor", "admin"]
    - actions: ["delete"]
      effect: DENY
      conditions:
        - match: "resource.attr.protected"
          operator: "eq"
          value: true
```

## Example
```
/policy template document
/policy add document-policy.yaml
/policy check user:alice document:123 read
/policy test rbac-tests
/policy conflicts
```
