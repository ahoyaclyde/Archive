"""
flug_client.py — Python SDK for the FLUG Evidence Platform API (v1)
====================================================================

Covers every route defined in python_api.rs:

    Health          GET  /api/v1/health
    Auth            POST /api/v1/auth/login
    Evidence read   GET  /api/v1/evidence
                    GET  /api/v1/evidence/{id}
                    GET  /api/v1/evidence/by-location
    Evidence write  POST /api/v1/evidence/{id}/update
                    POST /api/v1/evidence/{id}/status
    Targets read    GET  /api/v1/targets/by-evidence/{evidence_id}
                    GET  /api/v1/targets/by-location
                    GET  /api/v1/targets/by-incident-type/{type}
                    GET  /api/v1/targets/{target_id}
                    GET  /api/v1/targets/{target_id}/encoding  → numpy .npy bytes
    Target flags    POST /api/v1/targets/{target_id}/flag
    POI             GET  /api/v1/poi
                    POST /api/v1/poi
                    POST /api/v1/poi/{id}/status
    Face match      POST /api/v1/feedback/face-match
    SSE events      GET  /api/v1/events

Quick start
-----------
    from flug_client import FlugClient, EvidenceUpdate, FlagTarget, FaceMatchFeedback

    client = FlugClient(
        base_url="https://your-app.onrender.com",
        api_key="your-PYTHON_API_KEY-value",
    )

    # ── list evidence ────────────────────────────────────────────────────────
    page = client.list_evidence(status="Submitted", limit=20)
    for item in page["items"]:
        print(item["id"], item["title"])

    # ── auto-paginate ────────────────────────────────────────────────────────
    for record in client.iter_all_evidence(status="Submitted"):
        process(record)

    # ── get with targets + encodings ────────────────────────────────────────
    ev = client.get_evidence_by_location(county="Nairobi", mode=2)

    # ── targets ─────────────────────────────────────────────────────────────
    targets = client.get_targets_by_evidence("ev_abc123")
    npy_bytes = client.get_target_encoding("tgt_xyz")          # feed to np.load
    client.flag_target("tgt_xyz", FlagTarget(flag_type="poi", reason="Seen at scene"))

    # ── POI management ───────────────────────────────────────────────────────
    poi = client.create_poi(display_name="Unknown Male", category="person",
                            pinned_by_user_id="usr_abc")
    client.update_poi_status(poi["poi_id"], "active")

    # ── close a case via face-match pipeline ────────────────────────────────
    client.submit_face_match(FaceMatchFeedback(
        source_target_id="tgt_src",
        matched_target_id="tgt_match",
        evidence_id_source="ev_src",
        evidence_id_matched="ev_match",
        confidence_pct=92.5,
        augmentation_name="DeepFace v0.3 — Nairobi sweep",
    ))

    # ── live audit-log stream (blocking) ────────────────────────────────────
    for event in client.stream_events():
        print(event["action_type"], event["target_id"])

Installation
------------
    pip install requests sseclient-py

    numpy is optional — required only if you call get_target_encoding() and want
    to decode the returned bytes automatically:
        import io, numpy as np
        arr = np.load(io.BytesIO(client.get_target_encoding(target_id)))

Environment variables (alternative to constructor args)
-------------------------------------------------------
    FLUG_BASE_URL   — e.g. https://your-app.onrender.com
    FLUG_API_KEY    — matches PYTHON_API_KEY on the Rust server
"""

from __future__ import annotations

import io
import json
import os
import time
from dataclasses import dataclass, field, asdict
from typing import Any, Generator, Iterator, List, Optional

import requests

try:
    import sseclient  # type: ignore
    _SSE_AVAILABLE = True
except ImportError:
    _SSE_AVAILABLE = False


# ─────────────────────────────────────────────────────────────────────────────
# Request dataclasses
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class EvidenceUpdate:
    """
    Patch payload for POST /api/v1/evidence/{id}/update.
    Only non-None fields are sent.
    """
    title:               Optional[str]  = None
    description:         Optional[str]  = None
    # red | orange | yellow | blue
    emergency_level:     Optional[str]  = None
    county:              Optional[str]  = None
    constituency:        Optional[str]  = None
    ward:                Optional[str]  = None
    landmark:            Optional[str]  = None
    suspect_description: Optional[str]  = None
    injuries:            Optional[str]  = None
    property_damage:     Optional[str]  = None
    needs_attention:     Optional[bool] = None
    police_case_id:      Optional[str]  = None
    police_station:      Optional[str]  = None

    def to_json(self) -> dict:
        return {k: v for k, v in asdict(self).items() if v is not None}


@dataclass
class FlagTarget:
    """
    Payload for POST /api/v1/targets/{id}/flag.
    flag_type must be one of: poi | watchlist | wanted | pin | takedown | flagged
    """
    flag_type:    str
    reason:       Optional[str] = None
    user_id:      Optional[str] = None   # FLUG user_id this flag is attributed to
    display_name: Optional[str] = None   # used when creating a POI record
    notes:        Optional[str] = None

    VALID_TYPES = frozenset({"poi", "watchlist", "pinned", "takedown", "flagged"})

    def __post_init__(self):
        if self.flag_type not in self.VALID_TYPES:
            raise ValueError(
                f"flag_type must be one of: {', '.join(sorted(self.VALID_TYPES))}"
            )

    def to_json(self) -> dict:
        return {k: v for k, v in asdict(self).items() if v is not None}


@dataclass
class FaceMatchFeedback:
    """
    Payload for POST /api/v1/feedback/face-match.

    On success the server will:
      1. Link the two evidence cases
      2. Flag matched evidence as needs_attention
      3. Write an audit log entry
      4. Notify the evidence uploader
    """
    source_target_id:    str
    matched_target_id:   str
    evidence_id_source:  str
    evidence_id_matched: str
    # 0.0–100.0
    confidence_pct:      float
    # e.g. "DeepFace v0.3 — Nairobi sweep"
    augmentation_name:   Optional[str] = None
    notes:               Optional[str] = None

    def __post_init__(self):
        if not (0.0 <= self.confidence_pct <= 100.0):
            raise ValueError("confidence_pct must be between 0 and 100")

    def to_json(self) -> dict:
        return {k: v for k, v in asdict(self).items() if v is not None}


@dataclass
class CreatePoi:
    """
    Payload for POST /api/v1/poi.
    category must be one of: person | vehicle | unknown
    """
    display_name:        str
    # person | vehicle | unknown
    category:            str
    pinned_by_user_id:   str
    notes:               Optional[str]       = None
    linked_evidence_ids: Optional[List[str]] = None

    VALID_CATEGORIES = frozenset({"person", "vehicle", "unknown"})

    def __post_init__(self):
        if self.category not in self.VALID_CATEGORIES:
            raise ValueError(
                f"category must be one of: {', '.join(sorted(self.VALID_CATEGORIES))}"
            )

    def to_json(self) -> dict:
        payload = {
            "display_name":      self.display_name,
            "category":          self.category,
            "pinned_by_user_id": self.pinned_by_user_id,
        }
        if self.notes is not None:
            payload["notes"] = self.notes
        if self.linked_evidence_ids is not None:
            payload["linked_evidence_ids"] = self.linked_evidence_ids
        return payload


# ─────────────────────────────────────────────────────────────────────────────
# Exceptions
# ─────────────────────────────────────────────────────────────────────────────

class FlugApiError(Exception):
    """Raised when the server returns a non-2xx response or success=false."""
    def __init__(self, status_code: int, message: str):
        self.status_code = status_code
        self.message     = message
        super().__init__(f"HTTP {status_code}: {message}")


class FlugAuthError(FlugApiError):
    """Raised on 401 — bad or missing API key."""


class FlugNotFoundError(FlugApiError):
    """Raised on 404."""


class FlugValidationError(FlugApiError):
    """Raised on 400 — invalid field values."""


# ─────────────────────────────────────────────────────────────────────────────
# Client
# ─────────────────────────────────────────────────────────────────────────────

class FlugClient:
    """
    Synchronous HTTP client for the FLUG Evidence Platform v1 API.

    Parameters
    ----------
    base_url : str
        Root URL of the Rust server, no trailing slash.
        Falls back to FLUG_BASE_URL environment variable.
    api_key : str
        Value of PYTHON_API_KEY set on the server.
        Falls back to FLUG_API_KEY environment variable.
    timeout : int
        Default request timeout in seconds (default 30).
        SSE connections ignore this and use an indefinite timeout.
    """

    def __init__(
        self,
        base_url: Optional[str] = None,
        api_key:  Optional[str] = None,
        timeout:  int           = 30,
    ) -> None:
        self.base_url = (base_url or os.environ.get("FLUG_BASE_URL", "")).rstrip("/")
        self._api_key = api_key or os.environ.get("FLUG_API_KEY", "")
        self.timeout  = timeout

        if not self.base_url:
            raise ValueError("base_url is required (or set FLUG_BASE_URL)")
        if not self._api_key:
            raise ValueError("api_key is required (or set FLUG_API_KEY)")

        self._session = requests.Session()
        self._session.headers.update({
            "X-API-Key":    self._api_key,
            "Content-Type": "application/json",
            "Accept":       "application/json",
        })

    # ── internal ─────────────────────────────────────────────────────────────

    def _url(self, path: str) -> str:
        return f"{self.base_url}{path}"

    def _raise_for(self, resp: requests.Response) -> dict:
        """Parse body and raise a typed exception on failure."""
        if resp.status_code == 400:
            body = self._safe_json(resp)
            raise FlugValidationError(400, body.get("error", resp.text[:200]))
        if resp.status_code == 401:
            raise FlugAuthError(401, "Invalid or missing API key")
        if resp.status_code == 404:
            body = self._safe_json(resp)
            raise FlugNotFoundError(404, body.get("error", "Not found"))
        if not resp.ok:
            body = self._safe_json(resp)
            raise FlugApiError(resp.status_code, body.get("error", resp.text[:200]))
        body = resp.json()
        if not body.get("success", True):
            raise FlugApiError(resp.status_code, body.get("error", "Unknown API error"))
        return body

    @staticmethod
    def _safe_json(resp: requests.Response) -> dict:
        try:
            return resp.json()
        except Exception:
            return {}

    @staticmethod
    def _strip_nones(d: dict) -> dict:
        return {k: v for k, v in d.items() if v is not None}

    # ═════════════════════════════════════════════════════════════════════════
    # HEALTH
    # ═════════════════════════════════════════════════════════════════════════

    def health(self) -> dict:
        """
        GET /api/v1/health   (no auth required)

        Returns::

            {"status": "ok", "timestamp": "…", "version": "v1"}
        """
        resp = self._session.get(self._url("/api/v1/health"), timeout=self.timeout)
        return self._raise_for(resp)

    # ═════════════════════════════════════════════════════════════════════════
    # AUTH
    # ═════════════════════════════════════════════════════════════════════════

    def login(self, email: str, password: str) -> dict:
        """
        POST /api/v1/auth/login   (no X-API-Key required)

        Verifies email + bcrypt password and returns the full user profile
        together with the api_key value so downstream code can store it.

        Returns::

            {
              "success": True,
              "api_key": "…",
              "user": {
                "id", "email", "is_verified", "account_type",
                "phone_number", "county", "wallet_address",
                "wallet_chain", "public_key", "geo_latitude",
                "geo_longitude", "is_profile_complete",
                "created_at", "updated_at", …
              }
            }

        Raises FlugAuthError on wrong credentials or unverified account.
        """
        resp = requests.post(
            self._url("/api/v1/auth/login"),
            json={"email": email, "password": password},
            timeout=self.timeout,
            headers={"Content-Type": "application/json", "Accept": "application/json"},
        )
        return self._raise_for(resp)

    # ═════════════════════════════════════════════════════════════════════════
    # EVIDENCE — READ
    # ═════════════════════════════════════════════════════════════════════════

    def list_evidence(
        self,
        page:            int           = 1,
        limit:           int           = 50,
        status:          Optional[str] = None,   # Draft|Submitted|Reported|UnderReview|Archived|Rejected
        emergency_level: Optional[str] = None,   # red|orange|yellow|blue
        incident_type:   Optional[str] = None,
        county:          Optional[str] = None,
        constituency:    Optional[str] = None,
        ward:            Optional[str] = None,
        query:           Optional[str] = None,
        sort_by:         Optional[str] = None,   # newest|oldest
        date_from:       Optional[str] = None,   # ISO date e.g. "2024-01-01"
        date_to:         Optional[str] = None,
        lat:             Optional[float] = None,
        lng:             Optional[float] = None,
        radius_km:       Optional[float] = None,
    ) -> dict:
        """
        GET /api/v1/evidence

        Returns::

            {
              "items":       [EvidenceSummary, …],
              "total":       int,
              "page":        int,
              "limit":       int,
              "total_pages": int,
            }
        """
        params = self._strip_nones({
            "page": page, "limit": limit,
            "status": status, "emergency_level": emergency_level,
            "incident_type": incident_type,
            "county": county, "constituency": constituency, "ward": ward,
            "query": query, "sort_by": sort_by,
            "date_from": date_from, "date_to": date_to,
            "lat": lat, "lng": lng, "radius_km": radius_km,
        })
        resp = self._session.get(
            self._url("/api/v1/evidence"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)["data"]

    def get_evidence(self, evidence_id: str) -> dict:
        """
        GET /api/v1/evidence/{id}

        Returns the full evidence detail dict.
        Raises FlugNotFoundError if the ID does not exist.
        """
        resp = self._session.get(
            self._url(f"/api/v1/evidence/{evidence_id}"),
            timeout=self.timeout,
        )
        return self._raise_for(resp)["data"]

    def get_evidence_by_location(
        self,
        county:            Optional[str]   = None,
        constituency:      Optional[str]   = None,
        ward:              Optional[str]   = None,
        country:           Optional[str]   = None,
        lat:               Optional[float] = None,
        lng:               Optional[float] = None,
        radius_km:         Optional[float] = None,
        incident_type:     Optional[str]   = None,
        status:            Optional[str]   = None,
        emergency_level:   Optional[str]   = None,
        mode:              int             = 1,
        include_encodings: Optional[bool]  = None,
        page:              int             = 1,
        limit:             int             = 50,
    ) -> dict:
        """
        GET /api/v1/evidence/by-location

        mode=1  → evidence summaries only (default, fast)
        mode=2  → evidence summaries + all targets + face encodings (slower)

        include_encodings defaults to True when mode=2.

        Returns::

            {
              "items":           [EvidenceSummary | EnrichedRecord, …],
              "total":           int,
              "page":            int,
              "limit":           int,
              "total_pages":     int,
              "location_filter": { … },
            }
        """
        params = self._strip_nones({
            "county": county, "constituency": constituency,
            "ward": ward, "country": country,
            "lat": lat, "lng": lng, "radius_km": radius_km,
            "incident_type": incident_type, "status": status,
            "emergency_level": emergency_level,
            "mode": mode, "include_encodings": include_encodings,
            "page": page, "limit": limit,
        })
        resp = self._session.get(
            self._url("/api/v1/evidence/by-location"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)["data"]

    def iter_all_evidence(
        self,
        page_size: int = 100,
        **list_kwargs,
    ) -> Generator[dict, None, None]:
        """
        Auto-paging generator over list_evidence().

        Example::

            for record in client.iter_all_evidence(status="Submitted"):
                process(record)
        """
        page = 1
        while True:
            result = self.list_evidence(page=page, limit=page_size, **list_kwargs)
            yield from result["items"]
            if page >= result["total_pages"]:
                break
            page += 1

    # ═════════════════════════════════════════════════════════════════════════
    # EVIDENCE — WRITE
    # ═════════════════════════════════════════════════════════════════════════

    def update_evidence(
        self,
        evidence_id: str,
        update:      EvidenceUpdate,
    ) -> dict:
        """
        POST /api/v1/evidence/{id}/update

        Returns::

            {"success": True, "message": "Updated", "id": "ev_…"}

        Raises FlugValidationError if no fields were supplied.
        Raises FlugNotFoundError if the ID does not exist.
        """
        payload = update.to_json()
        if not payload:
            raise ValueError("EvidenceUpdate has no fields set")
        resp = self._session.post(
            self._url(f"/api/v1/evidence/{evidence_id}/update"),
            json=payload,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def update_status(
        self,
        evidence_id: str,
        status:      str,
        reason:      Optional[str] = None,
    ) -> dict:
        """
        POST /api/v1/evidence/{id}/status

        Valid statuses: Draft | Submitted | Reported | UnderReview | Archived | Rejected

        Returns::

            {"success": True, "id": "ev_…", "new_status": "Archived"}
        """
        VALID = {"Draft", "Submitted", "Reported", "UnderReview", "Archived", "Rejected"}
        if status not in VALID:
            raise ValueError(f"status must be one of: {', '.join(sorted(VALID))}")
        payload: dict[str, Any] = {"status": status}
        if reason:
            payload["reason"] = reason
        resp = self._session.post(
            self._url(f"/api/v1/evidence/{evidence_id}/status"),
            json=payload,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def bulk_update_status(
        self,
        evidence_ids: List[str],
        status:       str,
        reason:       Optional[str] = None,
    ) -> List[dict]:
        """
        Call update_status for every ID.  Errors are captured inline rather
        than stopping the batch.

        Returns a list of result dicts, one per ID::

            [{"id": "ev_…", "success": True,  "result": …},
             {"id": "ev_…", "success": False, "error":  "…"}, …]
        """
        results = []
        for eid in evidence_ids:
            try:
                r = self.update_status(eid, status, reason)
                results.append({"id": eid, "success": True,  "result": r})
            except FlugApiError as exc:
                results.append({"id": eid, "success": False, "error": str(exc)})
        return results

    # ═════════════════════════════════════════════════════════════════════════
    # TARGETS — READ
    # ═════════════════════════════════════════════════════════════════════════

    def get_targets_by_evidence(
        self,
        evidence_id:       str,
        include_encodings: bool = True,
    ) -> List[dict]:
        """
        GET /api/v1/targets/by-evidence/{evidence_id}

        Returns a list of target objects.  Each target includes:
          - raw_image_url   (Storj CDN — download directly)
          - encodings[]     when include_encodings=True:
              face_index, detection_score, encoding_b64, encoding_path,
              phash, auto_generated, descriptor_dims (always 128)

        Decode an encoding in Python::

            import base64, numpy as np
            arr = np.frombuffer(
                base64.b64decode(t["encodings"][0]["encoding_b64"]),
                dtype=np.float32,
            )  # arr.shape == (128,)
        """
        resp = self._session.get(
            self._url(f"/api/v1/targets/by-evidence/{evidence_id}"),
            params={"include_encodings": str(include_encodings).lower()},
            timeout=self.timeout,
        )
        return self._raise_for(resp)["data"]

    def get_targets_by_location(
        self,
        county:            Optional[str]   = None,
        constituency:      Optional[str]   = None,
        ward:              Optional[str]   = None,
        country:           Optional[str]   = None,
        lat:               Optional[float] = None,
        lng:               Optional[float] = None,
        radius_km:         Optional[float] = None,
        incident_type:     Optional[str]   = None,
        category:          Optional[str]   = None,
        include_encodings: bool            = True,
        page:              int             = 1,
        limit:             int             = 100,
    ) -> dict:
        """
        GET /api/v1/targets/by-location

        Every target whose parent evidence matches the supplied location fields.
        Supports optional lat/lng/radius_km post-filter (Haversine distance).

        Returns::

            {
              "count":           int,
              "page":            int,
              "limit":           int,
              "location_filter": { … },
              "data":            [Target, …],
            }
        """
        params = self._strip_nones({
            "county": county, "constituency": constituency,
            "ward": ward, "country": country,
            "lat": lat, "lng": lng, "radius_km": radius_km,
            "incident_type": incident_type, "category": category,
            "include_encodings": include_encodings,
            "page": page, "limit": limit,
        })
        resp = self._session.get(
            self._url("/api/v1/targets/by-location"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def get_targets_by_incident_type(
        self,
        incident_type:     str,
        include_encodings: bool = True,
        page:              int  = 1,
        limit:             int  = 100,
    ) -> dict:
        """
        GET /api/v1/targets/by-incident-type/{type}

        e.g. incident_type="HitAndRun"

        Returns::

            {
              "incident_type": "…",
              "count":         int,
              "page":          int,
              "limit":         int,
              "data":          [Target, …],
            }
        """
        params = self._strip_nones({
            "include_encodings": include_encodings,
            "page": page,
            "limit": limit,
        })
        resp = self._session.get(
            self._url(f"/api/v1/targets/by-incident-type/{incident_type}"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def get_target(self, target_id: str) -> dict:
        """
        GET /api/v1/targets/{target_id}

        Returns the full target dict including face encodings.
        Raises FlugNotFoundError if the ID does not exist.
        """
        resp = self._session.get(
            self._url(f"/api/v1/targets/{target_id}"),
            timeout=self.timeout,
        )
        return self._raise_for(resp)["data"]

    def get_target_encoding(self, target_id: str) -> bytes:
        """
        GET /api/v1/targets/{target_id}/encoding

        Returns raw bytes of a numpy .npy file containing the 128-dim float32
        face encoding vector.

        Usage::

            import io, numpy as np
            arr = np.load(io.BytesIO(client.get_target_encoding("tgt_abc")))
            # arr.shape == (128,), arr.dtype == float32

        The server tries to serve a local .pkl file written by the face sidecar
        first; if not found it synthesises a valid .npy from the DB blob.
        The X-Encoding-Source response header will be "disk" or "database".

        Raises FlugNotFoundError if the target has no encoding.
        """
        resp = self._session.get(
            self._url(f"/api/v1/targets/{target_id}/encoding"),
            timeout=self.timeout,
        )
        if resp.status_code == 404:
            body = self._safe_json(resp)
            raise FlugNotFoundError(404, body.get("error", "No encoding for this target"))
        if resp.status_code == 401:
            raise FlugAuthError(401, "Invalid or missing API key")
        if not resp.ok:
            raise FlugApiError(resp.status_code, resp.text[:200])
        return resp.content

    def list_flagged_targets(
        self,
        flag_type: Optional[str] = None,   # poi|watchlist|pinned|takedown|flagged
        page:      int           = 1,
        limit:     int           = 100,
    ) -> dict:
        """
        GET /api/v1/targets/flagged

        target_flags stores flags as boolean columns (is_poi, is_watchlist,
        is_pinned, is_takedown, is_flagged) — one row per target, multiple
        flags per row. This is what the web UI writes to.

        Leave flag_type=None to get all flagged targets regardless of type.

        flag_type options: poi | watchlist | pinned | takedown | flagged

        Returns::

            {
              "count":  int,
              "page":   int,
              "limit":  int,
              "filter": "poi" | null,
              "counts_by_type": {
                "poi": 3, "watchlist": 2, "pinned": 0,
                "takedown": 1, "flagged": 0
              },
              "data": [
                {
                  "target_id": "tgt_…",
                  "flags": {
                    "is_poi": true, "is_watchlist": false, …,
                    "active": ["poi"]
                  },
                  "notes":      "…" | null,
                  "flagged_at": 1718000000,
                  "target":   { id, filename, raw_image_url, category, … },
                  "location": { county, constituency, ward, latitude, longitude },
                  "evidence": { incident_type, emergency_level, title, status, … },
                },
                …
              ]
            }
        """
        VALID = {"poi", "watchlist", "pinned", "takedown", "flagged"}
        if flag_type is not None and flag_type not in VALID:
            raise ValueError(f"flag_type must be one of: {', '.join(sorted(VALID))}")
        params = self._strip_nones({
            "flag_type": flag_type,
            "page":      page,
            "limit":     limit,
        })
        resp = self._session.get(
            self._url("/api/v1/targets/flagged"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    # ── convenience wrappers ──────────────────────────────────────────────────

    def list_poi_targets(self, page: int = 1, limit: int = 100) -> dict:
        """Shorthand for list_flagged_targets(flag_type='poi')."""
        return self.list_flagged_targets(flag_type="poi", page=page, limit=limit)

    def list_watchlist_targets(self, page: int = 1, limit: int = 100) -> dict:
        """Shorthand for list_flagged_targets(flag_type='watchlist')."""
        return self.list_flagged_targets(flag_type="watchlist", page=page, limit=limit)

    def list_takedown_targets(self, page: int = 1, limit: int = 100) -> dict:
        """Shorthand for list_flagged_targets(flag_type='takedown')."""
        return self.list_flagged_targets(flag_type="takedown", page=page, limit=limit)

    # ═════════════════════════════════════════════════════════════════════════
    # TARGETS — WRITE (FLAGS)
    # ═════════════════════════════════════════════════════════════════════════

    def flag_target(
        self,
        target_id: str,
        flag:      FlagTarget,
    ) -> dict:
        """
        POST /api/v1/targets/{target_id}/flag

        flag.flag_type must be one of:
            poi | watchlist | wanted | pin | takedown | flagged

        For poi / wanted the server also upserts a persons_of_interest row
        and fires a notification to the evidence uploader.

        Returns::

            {
              "success":     True,
              "flag_id":     "flag_…",
              "target_id":   "tgt_…",
              "evidence_id": "ev_…",
              "flag_type":   "poi",
              "poi_id":      "poi_…" | null,
              "uploader_notified": {"user_id": "…", "email": "…"},
            }
        """
        resp = self._session.post(
            self._url(f"/api/v1/targets/{target_id}/flag"),
            json=flag.to_json(),
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    # ═════════════════════════════════════════════════════════════════════════
    # PERSONS OF INTEREST
    # ═════════════════════════════════════════════════════════════════════════

    def list_poi(
        self,
        status: Optional[str] = None,   # watching | active | resolved | archived
        page:   int           = 1,
        limit:  int           = 50,
    ) -> dict:
        """
        GET /api/v1/poi

        Returns::

            {
              "count": int,
              "page":  int,
              "limit": int,
              "data":  [PoiRecord, …],
            }

        Each PoiRecord includes:
            id, poi_number, display_name, category, status, linked_cases,
            linked_evidence (JSON string), notes, pinned_by,
            created_at, last_seen_at, resolved_at
        """
        params = self._strip_nones({"status": status, "page": page, "limit": limit})
        resp = self._session.get(
            self._url("/api/v1/poi"),
            params=params,
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def create_poi(
        self,
        display_name:        str,
        category:            str,
        pinned_by_user_id:   str,
        notes:               Optional[str]       = None,
        linked_evidence_ids: Optional[List[str]] = None,
    ) -> dict:
        """
        POST /api/v1/poi

        category must be one of: person | vehicle | unknown

        Returns::

            {"success": True, "poi_id": "poi_…", "poi_number": "POI-XXXXXXXX"}
        """
        poi = CreatePoi(
            display_name=display_name,
            category=category,
            pinned_by_user_id=pinned_by_user_id,
            notes=notes,
            linked_evidence_ids=linked_evidence_ids,
        )
        resp = self._session.post(
            self._url("/api/v1/poi"),
            json=poi.to_json(),
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    def update_poi_status(self, poi_id: str, status: str) -> dict:
        """
        POST /api/v1/poi/{id}/status

        status must be one of: watching | active | resolved | archived

        Returns::

            {"success": True, "poi_id": "poi_…", "new_status": "active"}
        """
        VALID = {"watching", "active", "resolved", "archived"}
        if status not in VALID:
            raise ValueError(f"status must be one of: {', '.join(sorted(VALID))}")
        resp = self._session.post(
            self._url(f"/api/v1/poi/{poi_id}/status"),
            json={"status": status},
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    # ═════════════════════════════════════════════════════════════════════════
    # FACE MATCH FEEDBACK
    # ═════════════════════════════════════════════════════════════════════════

    def submit_face_match(self, feedback: FaceMatchFeedback) -> dict:
        """
        POST /api/v1/feedback/face-match

        Called by the Python intelligence pipeline when it finds a confirmed
        face match between two targets.  The server will:
          1. Link the two evidence cases (linked_cases table)
          2. Set needs_attention=1 on the matched evidence record
          3. Write a detailed audit log entry
          4. Notify the evidence uploader

        Returns::

            {
              "success":           True,
              "link_id":           "…",
              "notification_id":   "…",
              "augmentation":      "DeepFace v0.3 — Nairobi sweep",
              "confidence_pct":    92.5,
              "evidence_matched":  "ev_…",
              "evidence_source":   "ev_…",
              "uploader_notified": {"user_id": "…", "email": "…"},
            }
        """
        resp = self._session.post(
            self._url("/api/v1/feedback/face-match"),
            json=feedback.to_json(),
            timeout=self.timeout,
        )
        return self._raise_for(resp)

    # ═════════════════════════════════════════════════════════════════════════
    # SSE EVENT STREAM
    # ═════════════════════════════════════════════════════════════════════════

    def stream_events(
        self,
        retry_on_disconnect: bool = True,
        retry_delay:         int  = 5,
    ) -> Iterator[dict]:
        """
        GET /api/v1/events  (Server-Sent Events, blocking generator)

        The server tails audit_log and emits new rows every 3 seconds.
        Between events it sends ``: ping`` keepalive frames — these are
        filtered out and never yielded.

        Yields audit-log event dicts::

            {
              "id":            "audit_…",
              "action_type":   "evidence_updated",
              "action_target": "evidence",
              "target_id":     "ev_…",
              "details":       "{…}",       # JSON string — parse as needed
              "created_at":    1718000000,
            }

        Parameters
        ----------
        retry_on_disconnect : bool
            Automatically reconnect on network errors (default True).
        retry_delay : int
            Seconds to wait before reconnecting (default 5).

        Requires:  pip install sseclient-py

        Example::

            for event in client.stream_events():
                details = json.loads(event["details"])
                print(event["action_type"], event["target_id"], details)
        """
        if not _SSE_AVAILABLE:
            raise ImportError(
                "sseclient-py is required for stream_events(). "
                "Install with:  pip install sseclient-py"
            )

        url = self._url("/api/v1/events")
        headers = {
            "X-API-Key": self._api_key,
            "Accept":    "text/event-stream",
        }

        while True:
            try:
                resp = self._session.get(
                    url,
                    headers=headers,
                    stream=True,
                    timeout=None,   # indefinite — SSE is a long-lived connection
                )
                if resp.status_code == 401:
                    raise FlugAuthError(401, "Invalid API key")
                if not resp.ok:
                    raise FlugApiError(
                        resp.status_code,
                        f"SSE connect failed: {resp.text[:200]}",
                    )

                client = sseclient.SSEClient(resp)
                for event in client.events():
                    if not event.data or not event.data.strip():
                        continue   # ping / heartbeat frame
                    try:
                        yield json.loads(event.data)
                    except json.JSONDecodeError:
                        continue   # silently skip malformed frames

            except (requests.ConnectionError, requests.Timeout) as exc:
                if not retry_on_disconnect:
                    raise
                print(
                    f"[FlugClient] SSE disconnected ({exc}). "
                    f"Retrying in {retry_delay}s…"
                )
                time.sleep(retry_delay)


# ─────────────────────────────────────────────────────────────────────────────
# Smoke test — python flug_client.py
# ─────────────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import sys

    base = os.environ.get("FLUG_BASE_URL", "http://localhost:8080")
    key  = os.environ.get("FLUG_API_KEY",  "30b5d4a5a4204886ac2b5d791f8f113b359c8752e7ab79e6162b2400f0c057d2")

    if not key:
        print("Set FLUG_API_KEY before running the smoke test.")
        sys.exit(1)

    c = FlugClient(base_url=base, api_key=key)

    print("── health ──────────────────────────────────────────────────────")
    print(c.health())

    print("\n── list_evidence (first 5) ─────────────────────────────────────")
    page = c.list_evidence(limit=5)
    print(f"Total: {page['total']}  |  Pages: {page['total_pages']}")
    for item in page["items"]:
        print(" ", item.get("id"), item.get("title", "")[:60])

    print("\n── list_poi (persons_of_interest table) ────────────────────────")
    poi_page = c.list_poi(limit=5)
    print(f"POI count: {poi_page['count']}")
    for poi in poi_page["data"]:
        print(" ", poi.get("poi_number"), poi.get("display_name"))


    print("\n── list_flagged_targets (target_flags table — web UI source) ───")
    flags = c.list_flagged_targets()
    print(f"Counts by type: {flags['counts_by_type']}")
    for item in flags["data"][:5]:
        print(
            f"  {item['flags']['active']}",
            item["target"].get("filename", "")[:40],
            "—", item["evidence"].get("title", "")[:40],
        )

    print("\nSmoke test complete.")