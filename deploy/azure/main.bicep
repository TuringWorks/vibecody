@description('Resource tier: lite (2vCPU/4GB), pro (4vCPU/8GB), max (8vCPU/16GB)')
@allowed(['lite', 'pro', 'max'])
param tier string = 'lite'

@description('Azure region')
param location string = resourceGroup().location

@description('Container image')
param image string = 'ghcr.io/turingworks/vibecody:latest'

var tierConfig = {
  lite: { cpu: '2.0', memory: '4.0Gi' }
  pro:  { cpu: '4.0', memory: '8.0Gi' }
  max:  { cpu: '8.0', memory: '16.0Gi' }
}

resource env 'Microsoft.App/managedEnvironments@2023-05-01' = {
  name: 'vibecody-env'
  location: location
  properties: {
    zoneRedundant: false
  }
}

resource app 'Microsoft.App/containerApps@2023-05-01' = {
  name: 'vibecody'
  location: location
  properties: {
    managedEnvironmentId: env.id
    configuration: {
      ingress: {
        external: true
        targetPort: 7878
        transport: 'http'
      }
    }
    template: {
      containers: [
        {
          name: 'vibecli'
          image: image
          resources: {
            cpu: json(tierConfig[tier].cpu)
            memory: tierConfig[tier].memory
          }
          env: [
            { name: 'VIBECLI_PROVIDER', value: 'ollama' }
            { name: 'OLLAMA_HOST', value: 'http://localhost:11434' }
            { name: 'RUST_LOG', value: 'info' }
          ]
          probes: [
            {
              type: 'Liveness'
              httpGet: { path: '/health', port: 7878 }
              periodSeconds: 30
            }
          ]
        }
        {
          name: 'ollama'
          image: 'ollama/ollama:latest'
          resources: {
            cpu: json(tierConfig[tier].cpu)
            memory: tierConfig[tier].memory
          }
        }
      ]
      scale: { minReplicas: 1, maxReplicas: 1 }
    }
  }
}

output url string = 'https://${app.properties.configuration.ingress.fqdn}'
