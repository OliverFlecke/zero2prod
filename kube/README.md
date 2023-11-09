# Kubernetes deployment

This document contains information to deploy the service to a kubernetes cluster.
It will allow you to deploy the service itself with a postgres and Redis cluster,
along with telemetry.

The easiest way to get started locally is with [minikube](https://minikube.sigs.k8s.io/docs/).

## Steps

### Deploy postgres

Create storage for postgres and install through a helm chart.

- `kubectl apply -f postgres-pv.yaml`
- `kubectl apply -f postgres-pvc.yaml`
- `helm install postgres bitnami/postgresql --set persistence.existingClaim postgresql-pv-claim --set volumePermissions.enabled=true`

The postgres instance should now be running. It will have been created with the name `postgres-postresql` (could be better named).
The password can be retreived with:

```sh
kubectl get secret postgres-postgresql -o jsonpath="{.data.postgres-password}" | base64 --decode
```

To expose the DB to your local machine, use a port forward:

```sh
kubectl port-forward service/postgres-postgresql 5432:5432
```

To create the database and apply migrations, use the `sqlx` cli:

```sh
export DATABASE_URL="postgres://postgres:$(kubectl get secret postgres-postgresql -o jsonpath="{.data.postgres-password}" | base64 --decode)@localhost:5432/newsletter"
sqlx database create
sqlx migrate run
```

### Deploy Redis

TODO

### Deploy service

To deploy the service, apply all the config files for the api:

```sh
kubectl apply -f api-deployment.yaml -f api-service.yaml -f api-ingress.yaml
```

Note for minikube: By default the ingress in not enabled locally. If you want to access it from your local machine, this has to be enabled.

```sh
minikube addons enable ingress
minikube tunnel # this has to stay open during testing
```

To access local images in `minikube` the `load` command in combination with a `imagePullPolicy: Never`.

```sh
minikube image load <image name>
```
