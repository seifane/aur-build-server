# Kubernetes

There is kubernetes manifests provided as a reference in `kubernetes` folder.

You will need to adjust the configuration and possibly add an ingress for the service.

## Prerequisites 

You will need to add a custom seccomp profile that this is provided in `./docker/seccomp.json`, instruction about installing custom seccomp profiles [here](https://kubernetes.io/docs/tutorials/security/seccomp/).
More information about the seccomp profile can be found in the [docker docs](docker.md).

If you want to sign your packages create the configmap from your GPG key using the following command and uncomment the relevant chunks in the server deployment.
```bash
kubectl create configmap sign-key -n aurbuild --from-file=key.asc
```

## Deploy

You will need to update the storage class in the PVC (`03-pvc-serve.yaml`) to match the one you have on your cluster.

You can deploy the manifests as usual with kubectl.
```bash
kubectl apply -f kubernetes
```