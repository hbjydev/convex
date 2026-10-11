# Convex Helm chart

Deploys the Convex issue tracker and its embedded frontend using [bjw-s' common
library](https://github.com/bjw-s-labs/helm-charts/tree/common-5.3.0/charts/library/common).
Requires Kubernetes 1.31 or newer and Helm 3 or 4.

## Install from this repository

Provision PostgreSQL separately, with a database and role that can apply Convex's
migrations. The application applies embedded migrations before opening its HTTP
listener. All durable application data lives in PostgreSQL; no application PVC is
needed. The chart does not install PostgreSQL or an identity provider.

Create a Secret in the release namespace through your usual secret-management
workflow. Its keys are runtime environment variable names:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: convex-runtime
  namespace: convex
type: Opaque
stringData:
  DATABASE_URL: postgres://convex:REPLACE_ME@postgres.database.svc:5432/convex
  CONVEX_SETUP_TOKEN: REPLACE_WITH_A_RANDOM_TOKEN_OF_AT_LEAST_32_CHARACTERS
```

Generate a setup token with `openssl rand -base64 32`. Keep credentials out of
checked-in values files. The setup token is needed to create the first
administrator; omit that key after setup and restart the deployment.

Save your overrides as `values.yaml`:

```yaml
convex:
  publicUrl: https://convex.example.com
  existingSecret: convex-runtime

controllers:
  main:
    containers:
      main:
        image:
          tag: main

ingress:
  main:
    className: nginx
    hosts:
      - host: convex.example.com
        paths:
          - path: /
            pathType: Prefix
            service:
              identifier: main
              port: http
    tls:
      - secretName: convex-tls
        hosts:
          - convex.example.com
```

The image tag defaults to the chart's `appVersion`. Select an available container
tag for your installation: `main` follows development builds, while published
releases use their version number. Pin a release tag or set
`controllers.main.containers.main.image.digest` for reproducible deployments.
Provide an ingress controller and the TLS Secret yourself, or use the common
library's `route` settings with an existing Gateway. TLS can terminate at either
proxy; `publicUrl` must still be the exact HTTPS browser origin, without a path or
query. Serve both the frontend and `/api` at that origin.

From the repository root:

```sh
helm dependency build charts/convex
helm upgrade --install convex ./charts/convex \
  --namespace convex --create-namespace --values values.yaml --wait
```

The default ClusterIP Service exposes port 80 and forwards to the container's HTTP
listener on port 8080. Ingress and Gateway routes are optional and disabled by
default. For local access, set `convex.publicUrl` to `http://localhost:8080` and run
`kubectl -n convex port-forward service/convex 8080:80`.

## Runtime configuration

`convex.publicUrl` generates `CONVEX_PUBLIC_URL`. `convex.port` generates
`CONVEX_BIND_ADDRESS=0.0.0.0:<port>` and the container's named `http` port. Set these
two chart values instead of overriding those environment variables. Additional
settings go in `convex.config`, using Convex's environment variable names:

```yaml
convex:
  config:
    RUST_LOG: info
    CONVEX_SESSION_TTL_SECONDS: "604800"
    CONVEX_OIDC_ISSUER_URL: https://identity.example.com/realms/convex
    CONVEX_OIDC_CLIENT_ID: convex
    CONVEX_OIDC_DISPLAY_NAME: Company login
    CONVEX_OIDC_ENABLED: true
    CONVEX_OIDC_REQUIRED: false
    CONVEX_SMTP_HOST: smtp.example.com
    CONVEX_SMTP_PORT: 587
    CONVEX_SMTP_FROM: Convex <convex@example.com>
    CONVEX_SMTP_USERNAME: convex
```

Add `CONVEX_OIDC_CLIENT_SECRET` and `CONVEX_SMTP_PASSWORD` to the existing Secret
when using these settings. Omit OIDC settings entirely when using local accounts.
SMTP uses STARTTLS by default; do not set `CONVEX_SMTP_TLS=plain` for a production
SMTP server. Register the OIDC callback at
`https://convex.example.com/api/v1/auth/oidc/callback`. See the repository's
[authentication documentation](../../docs/authentication.md) for provider setup,
session lifetime, invitations, and recovery.

`convex.existingSecret` imports every Secret key through `envFrom`. Alternatively,
reference individual keys through `convex.config` or native container `env`:

```yaml
convex:
  config:
    DATABASE_URL:
      valueFrom:
        secretKeyRef:
          name: convex-db-app
          key: uri
    CONVEX_SETUP_TOKEN:
      valueFrom:
        secretKeyRef:
          name: convex-setup
          key: token
```

This works with database operators that publish a connection URI in a Secret.
References must point to Secrets in the release namespace. The chart checks that
a database configuration source is present; it cannot check the contents of
existing Secrets or ConfigMaps during an offline render.

Native `controllers.main.containers.main.env` overrides `convex.config`, including
Secret references; both common-library map and list syntax are supported. Explicit
environment variables override `envFrom` values. The chart always sets the public
URL and bind address from `convex.publicUrl` and `convex.port`. The default session
lifetime and log level come from `convex.config`; remove a default with YAML
`null` when supplying it through `envFrom` instead.

## Workload customization and upgrades

Other top-level values are passed directly to the common library. Use
`controllers`, `defaultPodOptions`, `service`, `ingress`, `route`, `persistence`,
`configMaps`, `secrets`, and its other features as needed. Refer to the pinned
library's [values](https://github.com/bjw-s-labs/helm-charts/blob/common-5.3.0/charts/library/common/values.yaml)
and [documentation](https://bjw-s-labs.github.io/helm-charts/docs/).

The default deployment has one replica and uses `Recreate`: all old pods stop
before replacement pods start. Convex requires this when authentication settings
change, including session lifetime and OIDC configuration. You can increase
`controllers.main.replicas` with a shared PostgreSQL database. Keep every replica's
configuration identical. Changing the strategy to `RollingUpdate` requires
stopping all replicas separately before applying authentication changes.

Pods run as UID/GID 10001, with a read-only root filesystem, dropped capabilities,
and no mounted Kubernetes API token. Startup and readiness probes call
`/api/v1/health`, which checks the database, schema, and authentication policy.
The startup probe allows five minutes for migrations; increase its failure
threshold for longer migrations. Liveness checks the TCP listener. Resource
requests and limits are unset; size them for your workload through
`controllers.main.containers.main.resources`.

Updating an existing Secret does not automatically restart pods. Restart the
deployment to load new values. For authentication changes, stop every replica,
update the Secret and chart settings, and then start them together.

## Validate and package

```sh
mise run helm:deps
mise run check:helm
helm package charts/convex
```

The checks lint and render minimal configuration, OIDC/SMTP with TLS ingress,
and individual Secret references with a Gateway route. They also verify required
configuration failures and rendering from a packaged chart. `Chart.lock` pins the
dependency; downloaded dependency archives are ignored by Git. This repository
currently supports local installation and packaging, without publishing a Helm
repository.
