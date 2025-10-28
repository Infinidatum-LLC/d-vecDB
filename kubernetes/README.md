# d-vecDB Kubernetes Deployment

This directory contains Kubernetes manifests for deploying d-vecDB in various environments.

## Directory Structure

```
kubernetes/
├── base/                          # Base Kubernetes resources
│   ├── deployment.yaml            # StatefulSet for d-vecDB
│   ├── service.yaml               # Services (REST, gRPC, metrics)
│   ├── configmap.yaml             # Configuration
│   ├── pdb.yaml                   # Pod Disruption Budget
│   ├── hpa.yaml                   # Horizontal Pod Autoscaler
│   ├── servicemonitor.yaml        # Prometheus ServiceMonitor
│   └── kustomization.yaml         # Kustomize base configuration
└── overlays/                      # Environment-specific configurations
    ├── development/               # Dev environment
    │   └── kustomization.yaml
    ├── staging/                   # Staging environment
    │   └── kustomization.yaml
    └── production/                # Production environment
        └── kustomization.yaml
```

## Prerequisites

1. **Kubernetes Cluster** (1.19+)
   - EKS, GKE, AKS, or any Kubernetes cluster
   - kubectl configured

2. **Kustomize** (optional, built into kubectl)
   ```bash
   # Verify kubectl supports kustomize
   kubectl version --client
   ```

3. **Storage Class**
   - Ensure your cluster has a StorageClass
   - Default: `standard`
   - Production: `fast-ssd` (configure in production overlay)

4. **Prometheus Operator** (optional, for metrics)
   - Required for ServiceMonitor to work
   - Install: https://prometheus-operator.dev/docs/prologue/quick-start/

## Quick Start

### Deploy to Development

```bash
# Create namespace
kubectl create namespace dvecdb-dev

# Deploy using kustomize
kubectl apply -k kubernetes/overlays/development/

# Verify deployment
kubectl get pods -n dvecdb-dev
kubectl get svc -n dvecdb-dev

# Check logs
kubectl logs -f -n dvecdb-dev -l app=dvecdb

# Port forward for local access
kubectl port-forward -n dvecdb-dev svc/dev-dvecdb 8080:8080
```

### Deploy to Staging

```bash
kubectl create namespace dvecdb-staging
kubectl apply -k kubernetes/overlays/staging/

# Watch rollout
kubectl rollout status statefulset/staging-dvecdb -n dvecdb-staging
```

### Deploy to Production

```bash
kubectl create namespace dvecdb-prod
kubectl apply -k kubernetes/overlays/production/

# Monitor deployment
kubectl get pods -n dvecdb-prod -w
```

## Configuration

### Environment Variables

Configure via ConfigMap in `base/configmap.yaml` or overlay-specific kustomizations:

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Server bind address |
| `REST_PORT` | `8080` | REST API port |
| `GRPC_PORT` | `9090` | gRPC port |
| `METRICS_PORT` | `9091` | Prometheus metrics port |
| `LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `TOKIO_WORKER_THREADS` | `4` | Tokio async runtime threads |

### Resource Requirements

#### Development
- CPU: 250m request, 2 cores limit
- Memory: 1Gi request, 4Gi limit
- Storage: 20Gi

#### Staging
- CPU: 500m request, 4 cores limit
- Memory: 2Gi request, 8Gi limit
- Storage: 50Gi

#### Production
- CPU: 1 core request, 8 cores limit
- Memory: 4Gi request, 16Gi limit
- Storage: 200Gi

## Health Checks

The deployment includes three types of probes:

### Liveness Probe
```yaml
httpGet:
  path: /health/live
  port: 8080
initialDelaySeconds: 10
periodSeconds: 10
```

### Readiness Probe
```yaml
httpGet:
  path: /ready
  port: 8080
initialDelaySeconds: 5
periodSeconds: 5
```

### Startup Probe
```yaml
httpGet:
  path: /health/live
  port: 8080
failureThreshold: 30
periodSeconds: 10
```

## Services

### ClusterIP (Internal)
- **Name**: `dvecdb` / `{env}-dvecdb`
- **Ports**: 8080 (REST), 9090 (gRPC), 9091 (metrics)
- **Usage**: Internal cluster access

### LoadBalancer (External)
- **Name**: `dvecdb-rest`, `dvecdb-grpc`
- **Type**: LoadBalancer (NLB on AWS)
- **Usage**: External API access

### Headless Service
- **Name**: `dvecdb-headless`
- **Type**: ClusterIP (None)
- **Usage**: StatefulSet DNS

## Scaling

### Manual Scaling

```bash
# Scale up
kubectl scale statefulset/prod-dvecdb -n dvecdb-prod --replicas=5

# Scale down (be careful with StatefulSets!)
kubectl scale statefulset/prod-dvecdb -n dvecdb-prod --replicas=3
```

### Auto-Scaling (HPA)

Horizontal Pod Autoscaler is configured in `base/hpa.yaml`:

- **Min Replicas**: 1 (dev), 2 (staging), 3 (prod)
- **Max Replicas**: 10
- **Metrics**: CPU (70%), Memory (80%)

```bash
# Check HPA status
kubectl get hpa -n dvecdb-prod

# Describe HPA
kubectl describe hpa prod-dvecdb-hpa -n dvecdb-prod
```

## Persistence

### Volume Claims

Each pod gets a persistent volume via `volumeClaimTemplates`:

```bash
# List PVCs
kubectl get pvc -n dvecdb-prod

# Describe PVC
kubectl describe pvc data-prod-dvecdb-0 -n dvecdb-prod
```

### Storage Classes

Configure storage class in overlays:

```yaml
# For production (high performance)
storageClassName: fast-ssd

# For development (standard)
storageClassName: standard
```

## Monitoring

### Prometheus Integration

If Prometheus Operator is installed:

```bash
# Apply ServiceMonitor
kubectl apply -f kubernetes/base/servicemonitor.yaml -n dvecdb-prod

# Verify ServiceMonitor
kubectl get servicemonitor -n dvecdb-prod
```

### Metrics Endpoint

Access metrics directly:

```bash
# Port forward
kubectl port-forward -n dvecdb-prod pod/prod-dvecdb-0 9091:9091

# Fetch metrics
curl http://localhost:9091/metrics
```

### Grafana Dashboard

Import the Grafana dashboard from `monitoring/grafana-dashboard.json`

## Backup & Disaster Recovery

### Using Velero

Annotations are added for Velero backup:

```yaml
backup.velero.io/backup-volumes: data
```

```bash
# Install Velero
velero install --provider aws --bucket my-backup-bucket

# Create backup
velero backup create dvecdb-backup --include-namespaces dvecdb-prod

# Restore
velero restore create --from-backup dvecdb-backup
```

### Manual Backup

```bash
# Exec into pod
kubectl exec -it -n dvecdb-prod prod-dvecdb-0 -- /bin/sh

# Create backup (if backup tool is available)
dvecdb backup create --output /data/backup-$(date +%Y%m%d).tar.gz

# Copy backup out
kubectl cp dvecdb-prod/prod-dvecdb-0:/data/backup.tar.gz ./backup.tar.gz
```

## Troubleshooting

### Pod Not Starting

```bash
# Check pod events
kubectl describe pod prod-dvecdb-0 -n dvecdb-prod

# Check logs
kubectl logs prod-dvecdb-0 -n dvecdb-prod

# Check previous logs if crashed
kubectl logs prod-dvecdb-0 -n dvecdb-prod --previous
```

### Health Check Failures

```bash
# Test health endpoint
kubectl exec -it prod-dvecdb-0 -n dvecdb-prod -- curl http://localhost:8080/health/check

# Check readiness
kubectl exec -it prod-dvecdb-0 -n dvecdb-prod -- curl http://localhost:8080/ready
```

### Storage Issues

```bash
# Check PVC status
kubectl get pvc -n dvecdb-prod

# Check PV
kubectl get pv

# Describe to see events
kubectl describe pvc data-prod-dvecdb-0 -n dvecdb-prod
```

### Performance Issues

```bash
# Check resource usage
kubectl top pod -n dvecdb-prod

# Check HPA
kubectl get hpa -n dvecdb-prod

# Check metrics
kubectl port-forward -n dvecdb-prod prod-dvecdb-0 9091:9091
curl http://localhost:9091/metrics | grep dvecdb
```

## Rolling Updates

```bash
# Update image
kubectl set image statefulset/prod-dvecdb dvecdb=dvecdb/server:0.1.8 -n dvecdb-prod

# Watch rollout
kubectl rollout status statefulset/prod-dvecdb -n dvecdb-prod

# Pause rollout
kubectl rollout pause statefulset/prod-dvecdb -n dvecdb-prod

# Resume rollout
kubectl rollout resume statefulset/prod-dvecdb -n dvecdb-prod

# Rollback
kubectl rollout undo statefulset/prod-dvecdb -n dvecdb-prod
```

## Security

### Network Policies

Example network policy (add to base/ if needed):

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: dvecdb-netpol
spec:
  podSelector:
    matchLabels:
      app: dvecdb
  policyTypes:
  - Ingress
  - Egress
  ingress:
  - from:
    - podSelector: {}
    ports:
    - protocol: TCP
      port: 8080
    - protocol: TCP
      port: 9090
  egress:
  - to:
    - podSelector: {}
```

### Pod Security

Security context is configured in deployment:

- `runAsNonRoot: true`
- `runAsUser: 1000`
- `allowPrivilegeEscalation: false`
- `readOnlyRootFilesystem: true`

## Clean Up

```bash
# Delete development
kubectl delete namespace dvecdb-dev

# Delete staging
kubectl delete namespace dvecdb-staging

# Delete production (be careful!)
kubectl delete namespace dvecdb-prod

# Or delete specific resources
kubectl delete -k kubernetes/overlays/production/
```

## Best Practices

1. **Use Kustomize Overlays**: Keep environment-specific configs in overlays
2. **Resource Limits**: Always set requests and limits
3. **Pod Disruption Budget**: Ensure availability during voluntary disruptions
4. **Health Checks**: Configure proper liveness, readiness, and startup probes
5. **Monitoring**: Enable Prometheus metrics and alerts
6. **Backups**: Regular backups using Velero or custom scripts
7. **Security**: Run as non-root, use network policies
8. **Testing**: Test in development/staging before production
9. **Version Control**: Keep manifests in Git
10. **Documentation**: Document custom configurations

## Support

- **Documentation**: https://github.com/rdmurugan/d-vecDB/tree/main/docs
- **Issues**: https://github.com/rdmurugan/d-vecDB/issues
- **Discussions**: https://github.com/rdmurugan/d-vecDB/discussions

---

Generated for d-vecDB v0.1.7
