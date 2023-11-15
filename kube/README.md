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

### Deploying Postgres Operator

Seems like a better option for deploying a production ready version of Postgres is to use [CrunchyData's operator](https://access.crunchydata.com/documentation/postgres-operator/latest/tutorials/basic-setup).

The following is the commands used to install the operator and create a cluster with a working database (all commands should be run from the `kube/postgres-opertor` directory).

```sh
### Install the operator
kubectl apply --server-side -k kustomize/install/default
# Check the status of the pods
# kubectl get pods --selector=postgres-operator.crunchydata.com/control-plane=postgres-operator --field-selector=status.phase=Running

### Create the cluster
kubectl apply -k kustomize/postgres
# To check the status:
# kubectl describe postgresclusters.postgres-operator.crunchydata.com mail
# To see the instances
# kubectl get pods --selector=postgres-operator.crunchydata.com/cluster=mail,postgres-operator.crunchydata.com/instance

### Connecting to the service
# List the services
# kubectl get svc --selector=postgres-operator.crunchydata.com/cluster=mail

export PG_CLUSTER_USER_SECRET_NAME=mail-pguser-mail
export PGPASSWORD=$(kubectl get secrets -n zero2prod "${PG_CLUSTER_USER_SECRET_NAME}" -o go-template='{{.data.password | base64decode}}')
export PGUSER=$(kubectl get secrets -n zero2prod "${PG_CLUSTER_USER_SECRET_NAME}" -o go-template='{{.data.user | base64decode}}')
export PGDATABASE=$(kubectl get secrets -n zero2prod "${PG_CLUSTER_USER_SECRET_NAME}" -o go-template='{{.data.dbname | base64decode}}')
export DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@localhost:5432/${PGDATABASE}"

psql -h localhost
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
