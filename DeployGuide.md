Here's the exact step by step for the Render dashboard:

---

## Step 1 — Create New Web Service

Go to [render.com](https://render.com) → **New** → **Web Service**

---

## Step 2 — Connect Repository

- **Connect a repository** → select `ahoyaclyde/Archive`
- **Branch** → `CrimeBank`

---

## Step 3 — Basic Settings

| Field | Value |
|---|---|
| **Name** | `crimebank` |
| **Region** | Choose closest to your users |
| **Runtime** | `Docker` |
| **Dockerfile Path** | `./Dockerfile` |
| **Docker Context** | `.` (leave default) |
| **Instance Type** | `Standard` ($25/mo minimum — DeepFace needs RAM) |

Leave **Build Command** and **Start Command** blank — the Dockerfile handles everything via `ENTRYPOINT`.

---

## Step 4 — Environment Variables

Click **Add Environment Variable** for each one:

| Key | Value | Secret? |
|---|---|---|
| `RUST_LOG` | `info` | No |
| `PORT` | `8080` | No |
| `DATABASE_PATH` | `/app/data/flug_evidence.db` | No |
| `SESSION_SECRET` | click **Generate** | — |
| `STORJ_ACCESS_KEY` | your key | ✅ Yes |
| `STORJ_SECRET_KEY` | your key | ✅ Yes |
| `STORJ_ENDPOINT` | `https://gateway.storjshare.io` | No |
| `STORJ_SHARING_KEY` | `jwf3elu4346ewqioz2j7geg4ulta` | ✅ Yes |
| `FACE_SERVICE_URL` | `http://127.0.0.1:3001` | No |
| `FACE_THRESHOLD` | `0.55` | No |
| `ENCODINGS_DIR` | `/app/data/encodings` | No |
| `RESEND_API_KEY` | your key | ✅ Yes |
| `RESEND_FROM_EMAIL` | `onboarding@resend.dev` | No |
| `RESEND_FROM_NAME` | `FLUG Evidence` | No |

---

## Step 5 — Persistent Disk

Scroll to **Disks** → **Add Disk**:

| Field | Value |
|---|---|
| **Name** | `crimebank-data` |
| **Mount Path** | `/app/data` |
| **Size** | `5 GB` |

This single disk stores both your SQLite database and the encodings pickle. It survives every redeploy.

---

## Step 6 — Deploy

Click **Create Web Service**. Render will:

1. Pull your repo from the `CrimeBank` branch
2. Build the Docker image (first build takes ~8-12 minutes — FaceNet weights download during this)
3. Start the container via `start.sh`

Watch the logs for:
```
✅ Face sidecar ready
✅ Storj service initialized successfully
✅ Encodings bucket  : Ready
🚀 Listening on     : 0.0.0.0:8080
```

---

## Step 7 — After First Deploy

Once running, upload one target photo and confirm in logs:
```
🥒 [PICKLE] Spawning encoding update for 1 target(s)
🥒 [PICKLE] Pickle saved → /app/data/encodings/encodings.pickle
✅ [PICKLE] Storj copy updated: https://link.storjshare.io/.../encodings/encodings.pickle
```

That confirms the full pipeline is live in production.

---

**One thing to watch** — the first Docker build is slow because it downloads FaceNet weights (~92MB) and face-api.js models at build time. Every subsequent deploy uses Docker's layer cache and skips those downloads entirely, making redeploys much faster.