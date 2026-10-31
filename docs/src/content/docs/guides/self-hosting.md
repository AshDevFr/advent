---
title: Self-hosting
description: "Run a calendar on your own machines: the static site on any web server, the company edition with Docker, Docker Compose or Kubernetes."
sidebar:
  order: 10
---

A calendar is published one of two ways, chosen with `advent init --edition`:

- **`static`** (the default): plain files rebuilt every day. The included workflow deploys them to Cloudflare Pages, but any web server works.
- **`company`**: `advent serve` in a Docker image, for invited players, with progress in SQLite. See [Company edition](/guides/company/).

## The static site on your own server

`advent build` writes `dist/`: HTML, JSON and images, nothing to run. What it contains depends on the date, since each day is only decrypted once it is due. So a self-hosted static site needs the same thing the included workflow does: **a build just after every local midnight** in December, on a machine that holds the release key.

1. On the server, install the engine version pinned in `advent.yaml` and clone your event repository.
2. Store the release key where only the build can read it, for example `/etc/advent/release.key` (mode `600`), and export it as `ADVENT_RELEASE_KEY` in the build script.
3. Build into a fresh folder and swap it in, so visitors never see half a site:

   ```sh
   #!/bin/sh
   # /usr/local/bin/advent-build: run by cron just after midnight
   set -eu
   export ADVENT_RELEASE_KEY="$(cat /etc/advent/release.key)"
   cd /srv/advent/repo
   git pull --quiet
   advent verify --release --released
   advent build --out /srv/advent/next
   rm -rf /srv/advent/previous
   [ -d /srv/advent/site ] && mv /srv/advent/site /srv/advent/previous
   mv /srv/advent/next /srv/advent/site
   ```

   ```cron
   # Five minutes past midnight, server time; set the server to the event's timezone.
   5 0 * * * advent /usr/local/bin/advent-build
   ```

4. Serve `/srv/advent/site` over **https**: browsers only allow the answer checking (WebCrypto) on secure pages. With Caddy, which gets certificates by itself:

   ```caddy
   advent.example.com {
       root * /srv/advent/site
       file_server
   }
   ```

   Or nginx:

   ```nginx
   server {
       listen 443 ssl;
       server_name advent.example.com;
       root /srv/advent/site;
       location / {
           try_files $uri $uri/ =404;
       }
   }
   ```

To serve the calendar under a path rather than a domain root, build with `--base /calendar/` and point that location at the folder. GitHub Pages and GitLab Pages work the same way: run the build on a schedule in their CI and publish `dist/` (as `public/` on GitLab).

## The company edition

Start the repository with `advent init --edition company`. Its `Dockerfile` builds an image holding the engine and your sealed files, and nothing else: see [Build the calendar image](/guides/company/#build-the-calendar-image). The image:

- runs as user `10001`, not root;
- listens on port `8080` and answers `GET /healthz` with `200` once the content has been checked;
- keeps everything players do in `/data/advent.db`, so `/data` must be a volume;
- needs the release key and other secrets as environment variables at run time. They are never in the image.

Every setting is listed in [Server settings](/reference/server/). The examples below use the ones nearly every deployment needs.

### Docker

```sh
docker volume create advent-data
docker run -d --name advent --restart unless-stopped \
  -p 8080:8080 \
  -v advent-data:/data \
  -e ADVENT_PUBLIC_URL=https://advent.example.com \
  -e ADVENT_TRUST_PROXY=true \
  --env-file /etc/advent/advent.env \
  ghcr.io/acme/advent-calendar:latest
```

with `/etc/advent/advent.env` (mode `600`):

```sh
ADVENT_RELEASE_KEY=AGE-SECRET-KEY-1...
ADVENT_ADMIN__PASSWORD=a-long-random-password
ADVENT_MAIL__SENDER=smtp
ADVENT_MAIL__FROM=Advent <advent@example.com>
ADVENT_MAIL__SMTP__HOST=smtp.example.com
ADVENT_MAIL__SMTP__PASSWORD=...
```

Put it behind your reverse proxy for https, and keep `ADVENT_TRUST_PROXY=true` only when a proxy sets `X-Forwarded-For`; otherwise the limits would trust whatever a visitor sends.

To update the content or the engine, build and pull the new image, then `docker rm -f advent` and run the same command again. Progress stays in the volume.

### Docker Compose

`docker-compose.example.yml` in a company repository is ready to adapt. Copy it to the server as `docker-compose.yml`, set `image` and the settings, put the secrets in `.env` next to it, and:

```sh
docker compose up -d
docker compose logs -f advent      # the log says when it serves, and prints emails with mail.sender: log
docker compose pull && docker compose up -d   # after pushing a new image
```

### Kubernetes

The server keeps its state in one SQLite file, so run **exactly one replica** with a `Recreate` strategy: two pods writing the same database file would corrupt it, and a rolling update would briefly run two.

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: advent
type: Opaque
stringData:
  ADVENT_RELEASE_KEY: AGE-SECRET-KEY-1...
  ADVENT_ADMIN__PASSWORD: a-long-random-password
  ADVENT_MAIL__SMTP__PASSWORD: "..."
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: advent-data
spec:
  accessModes: [ReadWriteOnce]
  resources:
    requests:
      storage: 1Gi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: advent
spec:
  replicas: 1
  strategy:
    type: Recreate
  selector:
    matchLabels: { app: advent }
  template:
    metadata:
      labels: { app: advent }
    spec:
      securityContext:
        runAsUser: 10001
        runAsGroup: 10001
        fsGroup: 10001
      containers:
        - name: advent
          image: ghcr.io/acme/advent-calendar:latest
          ports:
            - containerPort: 8080
          env:
            - { name: ADVENT_PUBLIC_URL, value: https://advent.example.com }
            - { name: ADVENT_TRUST_PROXY, value: "true" }
            - { name: ADVENT_MAIL__SENDER, value: smtp }
            - { name: ADVENT_MAIL__FROM, value: "Advent <advent@example.com>" }
            - { name: ADVENT_MAIL__SMTP__HOST, value: smtp.example.com }
          envFrom:
            - secretRef: { name: advent }
          volumeMounts:
            - { name: data, mountPath: /data }
          readinessProbe:
            httpGet: { path: /healthz, port: 8080 }
          livenessProbe:
            httpGet: { path: /healthz, port: 8080 }
            periodSeconds: 30
      volumes:
        - name: data
          persistentVolumeClaim: { claimName: advent-data }
---
apiVersion: v1
kind: Service
metadata:
  name: advent
spec:
  selector: { app: advent }
  ports:
    - port: 80
      targetPort: 8080
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: advent
spec:
  rules:
    - host: advent.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service: { name: advent, port: { number: 80 } }
  tls:
    - hosts: [advent.example.com]
      secretName: advent-tls
```

If the image lives in a private registry, add an `imagePullSecrets` entry. Most ingress controllers set `X-Forwarded-For`, which is what `ADVENT_TRUST_PROXY` relies on.

### Backups

Everything is in `/data/advent.db`, in SQLite's WAL mode, so copying the file alone while the server writes can miss recent changes. Either stop the server for a moment:

```sh
docker stop advent
docker run --rm -v advent-data:/data -v "$PWD":/backup alpine \
  cp /data/advent.db /backup/advent-$(date +%F).db
docker start advent
```

or, without stopping it, use the `sqlite3` tool on the host (the image does not include it) against the volume's folder, which takes a consistent copy:

```sh
sqlite3 "$(docker volume inspect -f '{{ .Mountpoint }}' advent-data)/advent.db" \
  ".backup '/backups/advent-$(date +%F).db'"
```

On Kubernetes, use your storage's volume snapshots, or scale to zero, copy, and scale back. To move the server, copy the database to the new volume and run the same image with the same secrets.
