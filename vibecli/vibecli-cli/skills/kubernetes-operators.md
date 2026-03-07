---
triggers: ["k8s operator sdk", "kubebuilder", "operator pattern", "controller-runtime", "custom controller", "reconciliation loop"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["kubectl"]
category: devops
---

# Building Kubernetes Operators

When working with Kubernetes operators:

1. Scaffold with Kubebuilder for Go operators (`kubebuilder init --domain example.com && kubebuilder create api --group app --version v1 --kind MyResource`) — this generates the controller, CRD types, and RBAC markers out of the box.

2. Design the Reconcile loop to be idempotent and convergent — always compare desired state to actual state, never assume prior runs succeeded. Return `ctrl.Result{RequeueAfter: 30 * time.Second}` for periodic re-checks.

3. Use `controllerutil.CreateOrUpdate` to manage owned resources declaratively — set the desired state in the mutate function and let the helper decide whether to create or patch: `controllerutil.CreateOrUpdate(ctx, r.Client, deployment, func() error { /* set spec */ return ctrl.SetControllerReference(cr, deployment, r.Scheme) })`.

4. Set owner references on all child resources so they are garbage-collected when the parent CR is deleted. Use `ctrl.SetControllerReference()` and verify the reference appears in the child's `metadata.ownerReferences`.

5. Implement finalizers for cleanup of external resources (databases, cloud infra): add the finalizer on create, perform cleanup when `DeletionTimestamp` is set, then remove the finalizer to allow deletion to proceed.

6. Use status subresource to report state without triggering reconcile loops — update `.status` with `r.Status().Update(ctx, cr)` and define clear condition types: `Ready`, `Degraded`, `Progressing`.

7. Add RBAC markers above the Reconcile function for least-privilege access: `//+kubebuilder:rbac:groups=app.example.com,resources=myresources,verbs=get;list;watch;create;update;patch;delete` and `//+kubebuilder:rbac:groups=app.example.com,resources=myresources/status,verbs=get;update;patch`.

8. Use predicates to filter watch events and reduce unnecessary reconciliations: `builder.WithPredicates(predicate.GenerationChangedPredicate{})` skips reconciles triggered only by status updates.

9. Implement leader election for high availability — Kubebuilder enables it by default with `--leader-elect` flag. Never run multiple active controller instances that mutate the same resources.

10. Write integration tests using envtest (`sigs.k8s.io/controller-runtime/pkg/envtest`) which spins up a real API server and etcd — test the full reconcile loop including status updates, finalizers, and error handling.

11. Handle transient errors gracefully: return `ctrl.Result{}, err` to requeue with exponential backoff. For permanent errors, update status conditions with the failure reason and return `ctrl.Result{}, nil` to stop requeuing.

12. Version your CRD API properly — use `v1alpha1` for experimental, `v1beta1` for pre-stable, `v1` for GA. Implement conversion webhooks when evolving between versions and never remove fields without a full deprecation cycle.
