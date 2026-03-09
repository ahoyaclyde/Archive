// src/media_routes.rs - FIXED VERSION
use actix_web::{web, HttpResponse, HttpRequest};
use actix_session::Session;
use serde::Serialize;  // ADD THIS LINE
use std::collections::HashMap;
use serde_json::json;
use crate::auth::AuthService;
use crate::models::ApiResponse;
use crate::models::*;
use actix_multipart::Multipart;
use crate::evidence_service::EvidenceService;
use std::fs;
 use chrono::Local;
// Add at the top of media_routes.rs
// Add at the top of media_routes.rs
use futures_util::StreamExt;  // Add this line
use chrono::{DateTime, Utc};
use crate::models::{
    ProfileUpdateForm, 
    PasswordChangeForm, 
    WalletDisconnectForm
};
use crate::models::ChartData;
use serde::Deserialize;  // for JSON body structs

/// JSON payload sent by the evidence_view.html police-report modal.
/// Field names match exactly what the frontend submits.
#[derive(Deserialize)]
struct PoliceReportJson {
    report_number:    String,
    police_station:   String,
    officer_name:     Option<String>,
    report_date:      Option<String>,
    additional_notes: Option<String>,
}

// Now this will work
#[derive(Serialize)]
struct SimpleApiResponse {
    success: bool,
    message: String,
}
// ==================== HELPER FUNCTIONS ====================

/// Escapes user-supplied strings for safe insertion into HTML content and attributes.
/// Prevents XSS by replacing the five HTML special characters with their entity equivalents.
/// Must be applied to every piece of user-controlled data before it is interpolated into
/// an HTML format string.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#x27;")
}

fn render_evidence_card(evidence: &EvidenceSummary) -> String {
    // Emergency badge — uses lc-emerg-badge + lc-emerg-dot (matches linked cases)
    let (emerg_dot_color, emerg_label, emerg_badge_cls) = match evidence.emergency_level {
        EmergencyLevel::Red    => ("#ef4444", "Red",    "lc-badge-critical"),
        EmergencyLevel::Orange => ("#f97316", "Orange", "lc-badge-high"),
        EmergencyLevel::Yellow => ("#eab308", "Yellow", "lc-badge-medium"),
        EmergencyLevel::Blue   => ("#3b82f6", "Blue",   "lc-badge-low"),
    };
    // Status badge — same pattern
    let (status_dot, status_label, status_badge_cls) = match evidence.status {
        EvidenceStatus::Draft       => ("#94a3b8", "Draft",        "lc-badge-low"),
        EvidenceStatus::Submitted   => ("#3b82f6", "Submitted",    "lc-badge-medium"),
        EvidenceStatus::Reported    => ("#22c55e", "Reported",     "lc-badge-medium"),
        EvidenceStatus::UnderReview => ("#f59e0b", "Under Review", "lc-badge-high"),
        EvidenceStatus::Archived    => ("#8b5cf6", "Archived",     "lc-badge-low"),
        EvidenceStatus::Rejected    => ("#ef4444", "Rejected",     "lc-badge-critical"),
    };

    let incident_type_display = format!("{:?}", evidence.incident_type);
    let short_title = if evidence.title.len() > 36 { format!("{}…", &evidence.title[..36]) } else { evidence.title.clone() };
    let id_short = evidence.id[..8.min(evidence.id.len())].to_uppercase();

    format!(r##"<tr>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;width:44px;">
    <div style="width:32px;height:32px;border-radius:9px;background:rgba(59,130,246,.09);border:1px solid rgba(59,130,246,.18);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" stroke="#3b82f6" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/><polyline points="14 2 14 8 20 8" stroke="#3b82f6" stroke-width="1.8" stroke-linecap="round"/></svg>
    </div>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;">
    <a href="/evidence/view/{}" class="lc-case-link">{}</a>
    <span style="display:block;margin-top:3px;font-size:.58rem;font-weight:700;letter-spacing:.06em;font-family:var(--mono,monospace);color:var(--text-3);text-transform:uppercase;">#{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span class="lc-ltype-manual">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span class="lc-emerg-badge {}"><span class="lc-emerg-dot" style="background:{};"></span>{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span class="lc-emerg-badge {}"><span class="lc-emerg-dot" style="background:{};"></span>{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span style="font-size:.72rem;color:var(--text-2);font-family:var(--mono,monospace);">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;text-align:center;">
    <div style="display:inline-flex;align-items:center;gap:5px;">
      <a href="/evidence/view/{}" class="lc-act-btn" title="View scene">
        <svg width="13" height="13" viewBox="0 0 20 20" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M10.875 13.862C8.108 13.862 5.743 12.137 4.798 9.702C5.743 7.268 8.108 5.543 10.875 5.543C13.641 5.543 16.007 7.268 16.952 9.702C16.007 12.137 13.641 13.862 10.875 13.862ZM10.866 7.844C9.84 7.844 9.008 8.676 9.008 9.702C9.008 10.729 9.84 11.561 10.866 11.561H10.881C11.907 11.561 12.739 10.729 12.739 9.702C12.739 8.676 11.907 7.844 10.881 7.844H10.866Z" fill="currentColor"/></svg>
      </a>
      <button onclick="deleteEvidence(&#39;{}&#39;)" class="lc-act-btn purple" style="background:none;cursor:pointer;" title="Delete">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6l-1 14a2 2 0 01-2 2H8a2 2 0 01-2-2L5 6"/><path d="M10 11v6M14 11v6"/></svg>
      </button>
    </div>
  </td>
</tr>"##,
    html_escape(&evidence.id), short_title, id_short,
    incident_type_display,
    emerg_badge_cls, emerg_dot_color, emerg_label,
    status_badge_cls, status_dot, status_label,
    evidence.incident_time.format("%d %b %Y").to_string(),
    html_escape(&evidence.id),
    html_escape(&evidence.id),
    )
}
fn render_evidence_card_my(evidence: &EvidenceSummary) -> String {
    // ── Emergency level ──────────────────────────────────────────────────────
    let (emg_str, emg_dot_class, emg_level_class) = match evidence.emergency_level {
        EmergencyLevel::Red    => ("RED", "lvl-red", "card-poi"),
        EmergencyLevel::Orange => ("ORANGE", "lvl-orange", "card-watch"),
        EmergencyLevel::Yellow => ("YELLOW", "lvl-yellow", "card-pinned"),
        EmergencyLevel::Blue   => ("BLUE", "lvl-blue", "card-default"),
    };

    // ── Status ───────────────────────────────────────────────────────────────
    let status_str = match evidence.status {
        EvidenceStatus::Submitted   => "Submitted",
        EvidenceStatus::Reported    => "Reported",
        EvidenceStatus::UnderReview => "Under Review",
        EvidenceStatus::Archived    => "Archived",
        EvidenceStatus::Rejected    => "Rejected",
        EvidenceStatus::Draft       => "Draft",
    };

    // Determine ribbon class and icon based on needs_attention
    let (ribbon_class, ribbon_icon) = if evidence.needs_attention {
        ("ic-ribbon-poi", "🎯")
    } else {
        match evidence.emergency_level {
            EmergencyLevel::Red => ("ic-ribbon-poi", "🎯"),
            EmergencyLevel::Orange => ("ic-ribbon-watch", "👁"),
            EmergencyLevel::Yellow => ("ic-ribbon-pinned", "📌"),
            EmergencyLevel::Blue => ("", ""),
        }
    };

    // Determine card status class
    let card_status_class = if evidence.needs_attention {
        "card-poi"
    } else {
        match evidence.emergency_level {
            EmergencyLevel::Red => "card-poi",
            EmergencyLevel::Orange => "card-watch",
            EmergencyLevel::Yellow => "card-pinned",
            EmergencyLevel::Blue => "card-default",
        }
    };

    let incident_type_str = format!("{:?}", evidence.incident_type);
    let location_display  = if evidence.county.is_empty() { "Unknown".to_string() } else { evidence.county.clone() };
    let date_display      = evidence.incident_time.format("%d %b %Y").to_string();
    let police_str        = if evidence.reported_to_police { "true" } else { "false" };
    let has_media_str     = if evidence.has_media { "true" } else { "false" };
    let attention_str     = if evidence.needs_attention { "true" } else { "false" };
    let is_signed         = false; // You'll need to add this to your EvidenceSummary if needed

    let hash_display = format!("{:x}",
        evidence.id.as_bytes().iter()
            .fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64))
    );

    let title_short = if evidence.title.len() > 52 {
        format!("{}…", &evidence.title[..49])
    } else {
        evidence.title.clone()
    };

    let image_src = format!("/evidence/image/{}", html_escape(&evidence.id));

    // Confidence score (default to 75 if not available - you may want to add this to your model)
    let confidence_score = 75;
    
    // Confidence color classes
    let confidence_color = if confidence_score >= 80 { 
        "text-green-600 dark:text-green-400".to_string() 
    } else if confidence_score >= 60 { 
        "text-amber-600 dark:text-amber-400".to_string() 
    } else { 
        "text-red-500".to_string() 
    };
    
    let confidence_bar_color = if confidence_score >= 80 { 
        "bg-green-500".to_string() 
    } else if confidence_score >= 60 { 
        "bg-amber-400".to_string() 
    } else { 
        "bg-red-500".to_string() 
    };

    // Build conditional HTML strings with proper types
    let attention_pill = if evidence.needs_attention {
        r#"<span class="pill pill-poi">ATTENTION</span>"#.to_string()
    } else {
        String::new()
    };
    
    let signed_pill = if is_signed {
        r#"<span class="pill pill-signed">⛓ SIGNED</span>"#.to_string()
    } else {
        String::new()
    };
    
    let signed_badge = if is_signed {
        r#"<div class="ic-signed-badge">⛓ SIGNED</div>"#.to_string()
    } else {
        String::new()
    };
    
    let attention_pulse = if evidence.needs_attention {
        r#"<div class="ic-pulse"><span></span><span></span></div>"#.to_string()
    } else {
        String::new()
    };
    
    let ribbon_style = if ribbon_class.is_empty() { 
        "display:none".to_string() 
    } else { 
        String::new() 
    };

    format!(r#"
<article class="evidence-item intel-card {card_status_class}"
     data-id="{id}"
     data-title="{title_esc}"
     data-status="{status}"
     data-emergency="{emergency}"
     data-type="{inc_type}"
     data-date="{date}"
     data-location="{loc_esc}"
     data-evnum="{evnum}"
     data-hash="{hash_full}"
     data-police="{police}"
     data-size=""
     data-media="{media_bool}"
     data-attention="{attn}"
     data-policecase=""
     data-search="{search_blob}"
     data-confidence="{confidence}"
     data-report-count="0"
     data-linked-cases="0"
     data-signed="{signed}">

  <!-- Image -->
  <div class="ic-img" onclick="EV.openLightbox?.({{
    imageUrl: '{image_src}',
    description: '{title_esc}',
    evidenceNumber: '{evnum}'
  }})">
    <img src="{image_src}" alt="{title_esc}" onerror="this.style.display='none'; document.getElementById('no-img-{id}').style.display='flex';">
    <div class="ic-img-overlay">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" class="text-white/80 drop-shadow">
        <path d="M15 3h6v6M9 21H3v-6M21 3l-7 7M3 21l7-7" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </div>

    <!-- No image placeholder -->
    <div class="ic-no-img" id="no-img-{id}" style="display: none;">
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none" class="text-gray-300 dark:text-gray-600">
        <circle cx="12" cy="8" r="4" stroke="currentColor" stroke-width="1.5"/>
        <path d="M4 20c0-4 3.582-7 8-7s8 3 8 7" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
      </svg>
      <span class="text-[9px] text-gray-300 dark:text-gray-600" style="font-family:var(--font-mono)">NO IMAGE</span>
    </div>

    <!-- Status ribbon corner -->
    <div class="ic-ribbon {ribbon_class}" id="ribbon-{id}" style="{ribbon_style}">
    </div>
    
    <!-- Ribbon icon -->
    <span class="ic-ribbon-icon" id="ribbon-icon-{id}">{ribbon_icon}</span>

    <!-- Bottom pills -->
    <div class="ic-pills">
      {attention_pill}
      {signed_pill}
    </div>

    <!-- Wallet signed badge -->
    {signed_badge}

    <!-- POI pulse (if needs attention) -->
    {attention_pulse}
  </div>

  <!-- Kebab menu -->
  <div class="ic-kebab" x-data="{{ddOpen: false}}" @click.outside="ddOpen = false">
    <button @click.stop="ddOpen = !ddOpen" class="ic-kebab-btn">
      <svg width="12" height="12" viewBox="0 0 20 20" fill="none">
        <circle cx="10" cy="4.5" r="1.5" fill="currentColor"/>
        <circle cx="10" cy="10" r="1.5" fill="currentColor"/>
        <circle cx="10" cy="15.5" r="1.5" fill="currentColor"/>
      </svg>
    </button>
    <div x-show="ddOpen" class="ic-dropdown" @click.outside="ddOpen = false">
      <button @click="EV.openDrawer('{id}'); ddOpen = false"><i class="fas fa-eye w-3 opacity-60"></i>View Details</button>
      <button @click="console.log('Link to case', '{id}'); ddOpen = false"><i class="fas fa-link w-3 opacity-60"></i>Link to Case</button>
      <button @click="console.log('Report intel', '{id}'); ddOpen = false"><i class="fas fa-shield-halved w-3 opacity-60"></i>Report Intel</button>
      <div class="ic-dropdown-divider"></div>
      <button @click="EV.promptTakedown('{id}'); ddOpen = false" class="danger"><i class="fas fa-triangle-exclamation w-3"></i>File Complaint</button>
      <button @click="EV.exportRecord('{id}'); ddOpen = false"><i class="fas fa-download w-3 opacity-60"></i>Export Record</button>
    </div>
  </div>

  <!-- Body -->
  <div class="ic-body">
    <p class="ic-title" onclick="EV.openDrawer('{id}')">{title_short}</p>
    <p class="ic-evid">{evnum}</p>

    <div class="ic-meta">
      <span class="ic-tag">{inc_type}</span>
      <span class="ic-tag">{location}</span>
      <span class="ml-auto flex items-center gap-1.5">
        <span class="ic-level-dot {emg_dot_class}"></span>
        <span class="text-[9px] font-mono font-bold text-gray-400">{emergency}</span>
      </span>
    </div>

    <!-- Confidence -->
    <div class="ic-conf">
      <div class="ic-conf-row">
        <span class="ic-conf-label">Confidence</span>
        <span class="ic-conf-val {confidence_color}">{confidence}%</span>
      </div>
      <div class="ic-conf-track">
        <div class="ic-conf-fill {confidence_bar_color}" style="width: {confidence}%"></div>
      </div>
    </div>

    <!-- Stats row -->
    <div class="ic-stats">
      <span class="ic-stat">
        <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
        </svg>
        <span>0 rpts</span>
      </span>
      <span class="text-gray-200 dark:text-gray-700">·</span>
      <span class="ic-stat">
        <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/>
          <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/>
        </svg>
        <span>0 linked</span>
      </span>
      <span class="ic-stat ml-auto">{date}</span>
    </div>
  </div>

  <!-- Footer actions -->
  <div class="ic-footer">
    <button @click="console.log('Link case', '{id}')" class="ic-btn ic-btn-link">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71"/>
        <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71"/>
      </svg>
      Link
    </button>
    <button @click="console.log('Report', '{id}')" class="ic-btn ic-btn-rep">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/>
      </svg>
      Report
    </button>
    <button @click="console.log('Complaint', '{id}')" class="ic-btn ic-btn-cmp">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      CMP
    </button>
    <a href="/evidence/view/{id}" class="ic-btn ic-btn-view">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/>
        <polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/>
      </svg>
      View
    </a>
    <button @click="EV.exportRecord('{id}')" class="ic-btn ic-btn-exp ml-auto" title="Export">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/>
        <polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
      </svg>
    </button>
  </div>
</article>

<script>
// Hide no-image placeholder if image loads successfully
(function() {{
    var img = document.querySelector('img[src="{image_src}"]');
    if (img && img.complete && img.naturalHeight === 0) {{
        document.getElementById('no-img-{id}').style.display = 'flex';
    }}
}})();
</script>
    "#,
    id                  = html_escape(&evidence.id),
    title_esc           = html_escape(&evidence.title),
    title_short         = html_escape(&title_short),
    status              = status_str,
    emergency           = emg_str,
    inc_type            = html_escape(&incident_type_str),
    date                = html_escape(&date_display),
    loc_esc             = html_escape(&location_display),
    location            = html_escape(&location_display),
    evnum               = html_escape(&evidence.evidence_number),
    hash_full           = html_escape(&hash_display),
    police              = police_str,
    media_bool          = has_media_str,
    attn                = attention_str,
    signed              = if is_signed { "true" } else { "false" },
    confidence          = confidence_score,
    card_status_class   = card_status_class,
    emg_dot_class       = emg_dot_class,
    ribbon_class        = ribbon_class,
    ribbon_style        = ribbon_style,
    ribbon_icon         = ribbon_icon,
    attention_pill      = attention_pill,
    signed_pill         = signed_pill,
    signed_badge        = signed_badge,
    attention_pulse     = attention_pulse,
    confidence_color    = confidence_color,
    confidence_bar_color = confidence_bar_color,
    image_src           = image_src,
    search_blob         = html_escape(&format!("{} {} {} {} {} {}",
        evidence.title, evidence.evidence_number,
        incident_type_str, emg_str, status_str, location_display
    )),
    )
}

fn render_linked_case_row(
    my_number: &str,
    my_id: &str,
    my_title: &str,
    linked_number: &str,
    linked_id: &str,
    linked_title: &str,
    link_type: &str,
    link_reason: &str,
    confidence: i32,
    county: &str,
    created_at: &str,
    ownership: &str, // "Mine" | "Other"
) -> String {
    let confidence_pct = confidence;
    let confidence_color = if confidence_pct >= 80 {
        "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
    } else if confidence_pct >= 50 {
        "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400"
    } else {
        "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400"
    };

    // NOTE: We bind the fallback String first so it lives long enough for the match borrow.
    let link_type_badge_fallback = format!(
        r#"<span class="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400 rounded">{}</span>"#,
        link_type
    );
    let link_type_badge: &str = match link_type.to_lowercase().as_str() {
        "target_match" => r#"<span class="px-2 py-0.5 text-xs font-medium bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400 rounded">Target Match</span>"#,
        "location"     => r#"<span class="px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400 rounded">Location</span>"#,
        "manual"       => r#"<span class="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-400 rounded">Manual</span>"#,
        _              => &link_type_badge_fallback,
    };

    let ownership_badge = if ownership == "Mine" {
        r#"<span class="px-2 py-0.5 text-xs font-medium bg-brand-50 text-brand-600 dark:bg-brand-900/20 dark:text-brand-400 rounded-full">Owner</span>"#
    } else {
        r#"<span class="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-500 dark:bg-gray-800 dark:text-gray-400 rounded-full">Linked</span>"#
    };

    let short_reason = if link_reason.len() > 40 {
        format!("{}...", &link_reason[..40])
    } else {
        link_reason.to_string()
    };

    let my_title_short = if my_title.len() > 22 { format!("{}...", &my_title[..22]) } else { my_title.to_string() };
    let linked_title_short = if linked_title.len() > 22 { format!("{}...", &linked_title[..22]) } else { linked_title.to_string() };

    // Escape all user-controlled fields before HTML interpolation
    let my_id_e            = html_escape(my_id);
    let my_number_e        = html_escape(my_number);
    let my_title_short_e   = html_escape(&my_title_short);
    let linked_id_e        = html_escape(linked_id);
    let linked_number_e    = html_escape(linked_number);
    let linked_title_short_e = html_escape(&linked_title_short);
    let county_e           = html_escape(county);
    let link_reason_e      = html_escape(link_reason);
    let short_reason_e     = html_escape(&short_reason);

    format!(r#"
      <div class="grid grid-cols-12 border-t border-gray-100 px-6 py-[16px] dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/50 transition-colors">
        <!-- My Case -->
        <div class="col-span-3 flex items-center gap-2">
          <div class="flex flex-col">
            <a href="/evidence/view/{}" class="text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-brand-500 dark:hover:text-brand-400 transition-colors">
              #{} — {}
            </a>
            <div class="mt-0.5">{}</div>
          </div>
        </div>
        <!-- Linked Case -->
        <div class="col-span-3 flex items-center gap-2">
          <div class="flex flex-col">
            <a href="/evidence/view/{}" class="text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-brand-500 dark:hover:text-brand-400 transition-colors">
              #{} — {}
            </a>
            <span class="text-xs text-gray-400 dark:text-gray-500 mt-0.5">{}</span>
          </div>
        </div>
        <!-- Link Type & Reason -->
        <div class="col-span-2 flex flex-col justify-center gap-1">
          {}
          <span class="text-xs text-gray-400 dark:text-gray-500 truncate" title="{}">{}</span>
        </div>
        <!-- Confidence -->
        <div class="col-span-2 flex items-center">
          <span class="px-2 py-0.5 text-xs font-semibold rounded {}">{} %</span>
        </div>
        <!-- Date Linked -->
        <div class="col-span-1 flex items-center">
          <span class="text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
        </div>
        <!-- Actions -->
        <div class="col-span-1 flex items-center justify-center gap-2">
          <a href="/evidence/view/{}" class="text-gray-400 hover:text-brand-500 dark:hover:text-brand-400 transition-colors" title="View My Case">
            <svg class="fill-current" width="18" height="18" viewBox="0 0 21 20" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path fill-rule="evenodd" clip-rule="evenodd" d="M10.8749 13.8619C8.10837 13.8619 5.74279 12.1372 4.79804 9.70241C5.74279 7.26761 8.10837 5.54297 10.8749 5.54297C13.6415 5.54297 16.0071 7.26762 16.9518 9.70243C16.0071 12.1372 13.6415 13.8619 10.8749 13.8619ZM10.8749 4.04297C7.35666 4.04297 4.36964 6.30917 3.29025 9.4593C3.23626 9.61687 3.23626 9.78794 3.29025 9.94552C4.36964 13.0957 7.35666 15.3619 10.8749 15.3619C14.3932 15.3619 17.3802 13.0957 18.4596 9.94555C18.5136 9.78797 18.5136 9.6169 18.4596 9.45932C17.3802 6.30919 14.3932 4.04297 10.8749 4.04297Z" fill=""/>
            </svg>
          </a>
          <a href="/evidence/view/{}" class="text-gray-400 hover:text-purple-500 dark:hover:text-purple-400 transition-colors" title="View Linked Case">
            <svg class="fill-current" width="18" height="18" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path fill-rule="evenodd" clip-rule="evenodd" d="M13.5 3.5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 1 1-2 0V5.914l-8.293 8.293a1 1 0 0 1-1.414-1.414L18.086 4.5H14.5a1 1 0 0 1-1-1Zm-10 3a1 1 0 0 1 1-1h6a1 1 0 1 1 0 2H5.5v11h11v-5a1 1 0 1 1 2 0v6a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1v-13Z" fill=""/>
            </svg>
          </a>
        </div>
      </div>
    "#,
    my_id_e, my_number_e, my_title_short_e,
    ownership_badge,
    linked_id_e, linked_number_e, linked_title_short_e,
    county_e,
    link_type_badge,
    link_reason_e, short_reason_e,
    confidence_color, confidence_pct,
    created_at,
    my_id_e,
    linked_id_e,
    )
}

fn get_wallet_status_html(wallet_address: Option<&String>, wallet_chain: Option<&String>) -> String {
    match (wallet_address, wallet_chain) {
        (Some(addr), Some(chain)) => {
            let chain_badge = match chain.as_str() {
                "ethereum" => "bg-purple-600",
                "base" => "bg-blue-600",
                "avalanche" => "bg-red-600",
                "stellar" => "bg-purple-800",
                _ => "bg-gray-600",
            };
            
            format!(r#"
                <div class="mb-6 p-4 bg-green-900/20 border border-green-700 rounded-lg">
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-3">
                            <div class="w-10 h-10 rounded-full bg-green-600 flex items-center justify-center">
                                <i class="fas fa-wallet"></i>
                            </div>
                            <div>
                                <div class="font-semibold flex items-center">
                                    Wallet Connected
                                    <span class="ml-2 px-2 py-1 text-xs rounded {}">{}</span>
                                </div>
                                <div class="text-sm text-gray-300 font-mono">
                                    {}...{}
                                </div>
                            </div>
                        </div>
                        <div class="flex space-x-2">
                            <a href="/connect-wallet" class="text-sm bg-gray-700 px-3 py-1 rounded hover:bg-gray-600">
                                <i class="fas fa-sync mr-1"></i>Change
                            </a>
                            <form method="POST" action="/disconnect-wallet" class="inline">
                                <button type="submit" 
                                        class="text-sm bg-red-600 px-3 py-1 rounded hover:bg-red-700">
                                    <i class="fas fa-unlink mr-1"></i>Disconnect
                                </button>
                            </form>
                        </div>
                    </div>
                    <div class="mt-3 text-sm text-green-300">
                        <i class="fas fa-check-circle mr-1"></i>
                        Your evidence will be cryptographically signed and verified on-chain.
                    </div>
                </div>
            "#, chain_badge, chain, &addr[..6], &addr[addr.len()-4..])
        }
        _ => {
            format!(r#"
<div class="rounded-2xl border border-gray-200 bg-white dark:border-gray-800 dark:bg-white/[0.03] mb-2">
<div class="flex flex-col gap-4 border-b border-gray-200 px-4 py-4 sm:px-5 lg:flex-row lg:items-center lg:justify-between dark:border-gray-800">
    <div class="flex-shrink-0">
      <h3 class="text-lg font-semibold text-gray-800 dark:text-white/90">
        No Wallet Connected
      </h3>
      <p class="text-sm text-gray-500 dark:text-gray-400">
           Connect your wallet to cryptographically sign and monetize  your content .
      </p>
    </div>

    <div class="flex flex-col gap-4 lg:flex-row lg:items-center">
      <!-- Tab Navigation -->
     

      <!-- Filter Controls -->
      <div>
        <div class="relative">
      <button class="text-theme-sm shadow-theme-xs inline-flex items-center gap-2 rounded-lg border border-gray-300 bg-white px-4 py-2.5 font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
        Learn More
      </button>
        
       <a  href="/connect-wallet" class="text-theme-sm shadow-theme-xs inline-flex items-center gap-2 rounded-lg border border-gray-300 bg-white px-4 py-2.5 font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
        
        
          Connect Wallet
        </a>
        </div>
      </div>
    </div>
</div>
</div>


            "#)
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.2} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

// ==================== TEMPLATE RENDERER ====================

fn render_template(template_name: &str, context: &HashMap<&str, String>) -> String {
    let template_path = format!("static/templates/{}.html", template_name);
    match fs::read_to_string(&template_path) {
        Ok(mut template) => {
            // Simple template replacement
            for (key, value) in context {
                let placeholder = format!("{{{{ {} }}}}", key);
                template = template.replace(&placeholder, value);
            }
            template
        }
        Err(e) => {
            println!("Error reading template {}: {}", template_name, e);
            format!("<h1>Error loading template: {}</h1>", template_name)
        }
    }
}

// ==================== PAGE ROUTES ====================

pub async fn evidence_dashboard(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🎬 EVIDENCE_DASHBOARD: Starting dashboard check");
    
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🎬 EVIDENCE_DASHBOARD: User email found in session: {}", email);
        
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("🎬 EVIDENCE_DASHBOARD: Profile NOT complete - redirecting to /profile/complete");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
            }
            Ok(None) => {
                println!("🎬 EVIDENCE_DASHBOARD: User NOT found in database - clearing session");
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
            Err(e) => {
                println!("🎬 EVIDENCE_DASHBOARD: Error getting user: {} - clearing session", e);
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        let user_id = session.get::<String>("user_id").unwrap_or(None);
        let wallet_address = session.get::<String>("wallet_address").unwrap_or(None);
        let wallet_chain = session.get::<String>("wallet_chain").unwrap_or(None);
        
        let stats_result = if let Some(ref uid) = user_id {
            evidence_service.get_dashboard_stats(uid).await
        } else {
            Ok(DashboardStats {
                total_evidence: 0,
                urgent_count: 0,
                reported_count: 0,
                needs_attention_count: 0,
                today_count: 0,
                by_county: Vec::new(),
                by_type: Vec::new(),
            })
        };
        
        let stats = match stats_result {
            Ok(stats) => stats,
            Err(e) => {
                println!("🎬 EVIDENCE_DASHBOARD: Error getting stats: {}", e);
                DashboardStats {
                    total_evidence: 0,
                    urgent_count: 0,
                    reported_count: 0,
                    needs_attention_count: 0,
                    today_count: 0,
                    by_county: Vec::new(),
                    by_type: Vec::new(),
                }
            }
        };
        
        let recent_evidence = if let Some(ref uid) = user_id {
            evidence_service.get_user_evidence(uid).await.unwrap_or_default()
        } else {
            Vec::new()
        };
        
        let wallet_status = get_wallet_status_html(wallet_address.as_ref(), wallet_chain.as_ref());
        
        let recent_html = if recent_evidence.is_empty() {
            r#"<tr><td colspan="9" class="tg-table-cell" style="padding:40px;text-align:center;color:var(--text-3);font-size:13px;">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" style="margin:0 auto 10px;display:block;opacity:.35;"><path fill-rule="evenodd" clip-rule="evenodd" d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4z"/></svg>
              No evidence uploaded yet —
              <a href="/evidence/upload" style="color:#dc2626;font-weight:600;text-decoration:none;">Upload your first scene</a>
            </td></tr>"#.to_string()
        } else {
            let mut html = String::new();

            for evidence in recent_evidence.iter().take(6) {
                html.push_str(&render_evidence_card(evidence));
            }

            html
        };
        
        let county_stats_html = if stats.by_county.is_empty() {
            String::new()
        } else {
            let mut html = String::new();
            html.push_str(r#"<div class="space-y-2">"#);
            
            for county_stat in stats.by_county.iter().take(5) {
                let percentage = if stats.total_evidence > 0 {
                    (county_stat.count as f64 / stats.total_evidence as f64 * 100.0) as i32
                } else {
                    0
                };
                
                html.push_str(&format!(r#"
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <div class="w-2 h-2 rounded-full bg-red-500"></div>
                            <span class="text-sm">{}</span>
                        </div>
                        <div class="flex items-center space-x-3">
                            <span class="text-xs text-gray-400">{} cases</span>
                            <div class="w-16 bg-gray-700 rounded-full h-2">
                                <div class="bg-red-500 h-2 rounded-full" style="width: {}%"></div>
                            </div>
                        </div>
                    </div>
                "#, html_escape(&county_stat.county), county_stat.count, percentage));
            }
            
            html.push_str("</div>");
            html
        };
        
        let type_stats_html = if stats.by_type.is_empty() {
            String::new()
        } else {
            let mut html = String::new();
            html.push_str(r#"<div class="space-y-2">"#);
            
            for type_stat in stats.by_type.iter() {
                let percentage = if stats.total_evidence > 0 {
                    (type_stat.count as f64 / stats.total_evidence as f64 * 100.0) as i32
                } else {
                    0
                };
                
                let type_label = match type_stat.incident_type {
                    IncidentType::HitAndRun      => "Hit & Run",
                    IncidentType::Assault        => "Assault",
                    IncidentType::ThreatToLife   => "Threat to Life",
                    IncidentType::PropertyDamage => "Property Damage",
                    IncidentType::Theft          => "Theft",
                    IncidentType::Other          => "Other",
                };
                
                html.push_str(&format!(r#"
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <div class="w-2 h-2 rounded-full bg-blue-500"></div>
                            <span class="text-sm">{}</span>
                        </div>
                        <div class="flex items-center space-x-3">
                            <span class="text-xs text-gray-400">{} cases</span>
                            <div class="w-16 bg-gray-700 rounded-full h-2">
                                <div class="bg-blue-500 h-2 rounded-full" style="width: {}%"></div>
                            </div>
                        </div>
                    </div>
                "#, type_label, type_stat.count, percentage));
            }
            
            html.push_str("</div>");
            html
        };
        // Get chart data
        let chart_data = if let Some(ref uid) = user_id {
            match evidence_service.get_chart_data(uid).await {
                Ok(data) => data,
                Err(e) => {
                    println!("🎬 EVIDENCE_DASHBOARD: Error getting chart data: {}", e);
                    ChartData {
                        collated: 0,
                        reported: 0,
                        submitted: 0,
                        draft: 0,
                        urgent: 0,
                        signed: 0,
                        others: 0,
                    }
                }
            }
        } else {
            ChartData {
                collated: 0,
                reported: 0,
                submitted: 0,
                draft: 0,
                urgent: 0,
                signed: 0,
                others: 0,
            }
        };


             // Get storage statistics for polar area chart
        let storage_stats = if let Some(ref uid) = user_id {
            match evidence_service.get_storage_stats(uid).await {
                Ok(stats) => stats,
                Err(e) => {
                    println!("🎬 EVIDENCE_DASHBOARD: Error getting storage stats: {}", e);
                    MediaStorageStats {
                        media: 0,
                        scenes: 0,
                        profiles: 0,
                        evidence: 0,
                        target: 0,
                    }
                }
            }
        } else {
            MediaStorageStats {
                media: 0,
                scenes: 0,
                profiles: 0,
                evidence: 0,
                target: 0,
            }
        };
        
        let mut context = HashMap::new();
        context.insert("email", email.clone());
        context.insert("wallet_status", wallet_status);
        context.insert("stats_total_evidence", stats.total_evidence.to_string());
        context.insert("stats_urgent_count", stats.urgent_count.to_string());
        context.insert("stats_reported_count", stats.reported_count.to_string());
        context.insert("stats_needs_attention_count", stats.needs_attention_count.to_string());
        context.insert("county_stats_html", county_stats_html);
        context.insert("type_stats_html", type_stats_html);
        context.insert("recent_html", recent_html);
        // Add chart data to context
        context.insert("chart_collated", chart_data.collated.to_string());
        context.insert("chart_reported", chart_data.reported.to_string());
        context.insert("chart_submitted", chart_data.submitted.to_string());
        context.insert("chart_draft", chart_data.draft.to_string());
        context.insert("chart_urgent", chart_data.urgent.to_string());
        context.insert("chart_signed", chart_data.signed.to_string());
        context.insert("chart_others", chart_data.others.to_string());


         // Create JSON array for radar chart
        let chart_data_json = format!(
            "[{}, {}, {}, {}, {}, {}, {}]",
            chart_data.collated,
            chart_data.reported,
            chart_data.submitted,
            chart_data.draft,
            chart_data.urgent,
            chart_data.signed,
            chart_data.others
        );
        context.insert("chart_data_json", chart_data_json);
        
        // Add storage stats to context
        context.insert("storage_media", storage_stats.media.to_string());
        context.insert("storage_scenes", storage_stats.scenes.to_string());
        context.insert("storage_profiles", storage_stats.profiles.to_string());
        context.insert("storage_evidence", storage_stats.evidence.to_string());
        context.insert("storage_target", storage_stats.target.to_string());
        
        // Create JSON array for polar area chart
        let storage_data_json = format!(
            "[{}, {}, {}, {}, {}]",
            storage_stats.media,
            storage_stats.scenes,
            storage_stats.profiles,
            storage_stats.evidence,
            storage_stats.target
        );

        context.insert("storage_data_json", storage_data_json);

        let wallet_address = session.get::<String>("wallet_address").unwrap_or_default();
        let wallet_address_str = wallet_address.clone().unwrap_or_default();
        context.insert("wallet_address", wallet_address_str);
                
if wallet_address.is_some() {
    context.insert("wallet_badge", r#"<span class="ml-2 px-2 py-1 bg-green-900 text-xs rounded">Wallet Connected</span>"#.to_string());
} else {
    context.insert("wallet_badge", "".to_string());
}

// Just add this to your dashboard route after getting user_id
let signed_count = if let Some(ref uid) = user_id {
    // Create a temporary connection (assuming pool is accessible)
    // Or better, add a method to EvidenceService
    match evidence_service.database.pool.get() {
        Ok(conn) => {
            conn.query_row(
                "SELECT COUNT(*) FROM evidence WHERE uploader_id = ? AND wallet_signature IS NOT NULL AND wallet_signature != ''",
                [uid],
                |row| row.get::<_, i64>(0)
            ).unwrap_or(0)
        }
        Err(_) => 0
    }
} else {
    0
};

context.insert("signed_count", signed_count.to_string());

        // ── Linked Cases Table ────────────────────────────────────────────────
        let mut linked_count: i64 = 0;
        let linked_cases_html = if let Some(ref uid) = user_id {
            let user_ev = evidence_service.get_user_evidence(uid).await.unwrap_or_default();
            let mut rows = String::new();
            let mut seen_pairs = std::collections::HashSet::new();

            for ev in &user_ev {
                let links = evidence_service.database.get_linked_evidence(&ev.id).await.unwrap_or_default();
                for link in links {
                    let other_id = if link.evidence_id_1 == ev.id { link.evidence_id_2.clone() } else { link.evidence_id_1.clone() };
                    let pair = if ev.id < other_id { format!("{}_{}", ev.id, other_id) } else { format!("{}_{}", other_id, ev.id) };
                    if seen_pairs.contains(&pair) { continue; }
                    seen_pairs.insert(pair);

                    let other_opt = evidence_service.get_evidence(&other_id, false).await.unwrap_or(None);
                    let current_opt = evidence_service.get_evidence(&ev.id, false).await.unwrap_or(None);

                    if let (Some(cur), Some(other)) = (current_opt, other_opt) {
                        let ownership = if cur.uploader_id == *uid { "Mine" } else { "Linked" };
                        let created_str = link.created_at.format("%d %b %Y").to_string();
                        let county = other.location.county.clone();
                        rows.push_str(&render_linked_case_row(
                            &cur.evidence_number,
                            &cur.id,
                            &cur.title,
                            &other.evidence_number,
                            &other.id,
                            &other.title,
                            &link.link_type,
                            &link.link_reason,
                            link.confidence_score,
                            &county,
                            &created_str,
                            ownership,
                        ));
                    }
                }
            }

            linked_count = seen_pairs.len() as i64;

            if rows.is_empty() {
                r#"
                <div class="text-center py-12">
                    <svg class="mx-auto mb-3 text-gray-300 dark:text-gray-600" width="48" height="48" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M13.5 3.5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v6a1 1 0 1 1-2 0V5.914l-8.293 8.293a1 1 0 0 1-1.414-1.414L18.086 4.5H14.5a1 1 0 0 1-1-1Zm-10 3a1 1 0 0 1 1-1h6a1 1 0 1 1 0 2H5.5v11h11v-5a1 1 0 1 1 2 0v6a1 1 0 0 1-1 1h-13a1 1 0 0 1-1-1v-13Z" fill="currentColor"/>
                    </svg>
                    <p class="text-gray-400 dark:text-gray-500">No linked cases found</p>
                    <p class="text-xs text-gray-400 dark:text-gray-600 mt-1">Cases are linked automatically when matching targets are detected</p>
                </div>
                "#.to_string()
            } else {
                rows
            }
        } else {
            String::new()
        };
        context.insert("linked_cases_html", linked_cases_html);

        // ── Derived stats for stat-card percentages ───────────────────────────
        let total_f = stats.total_evidence as f64;
        let pct = |n: i64| -> String {
            if total_f > 0.0 { format!("{}", (n as f64 / total_f * 100.0).round() as u64) }
            else { "0".to_string() }
        };
        context.insert("stats_signed_pct",    pct(signed_count));
        context.insert("stats_urgent_pct",    pct(stats.urgent_count as i64));
        context.insert("stats_linked_pct",    pct(linked_count));
        context.insert("stats_reported_pct",  pct(stats.reported_count as i64));
        context.insert("stats_attention_pct", pct(stats.needs_attention_count as i64));
        context.insert("linked_count",         linked_count.to_string());
        context.insert("stats_today_count",    stats.today_count.to_string());

        // ── Storage file-count totals ─────────────────────────────────────────
        let storage_total_files = storage_stats.media
            + storage_stats.scenes
            + storage_stats.profiles
            + storage_stats.evidence
            + storage_stats.target;
        context.insert("storage_total_files", storage_total_files.to_string());

        // ── Incident-type bar-chart JSON ──────────────────────────────────────
        // Produces {"labels":["Hit & Run","Assault",...], "data":[3,7,...]}
        let (type_labels_vec, type_data_vec): (Vec<String>, Vec<i64>) = stats.by_type.iter()
            .map(|t| {
                let label = match t.incident_type {
                    IncidentType::HitAndRun      => "Hit & Run",
                    IncidentType::Assault        => "Assault",
                    IncidentType::ThreatToLife   => "Threat to Life",
                    IncidentType::PropertyDamage => "Property Damage",
                    IncidentType::Theft          => "Theft",
                    IncidentType::Other          => "Other",
                };
                (label.to_string(), t.count as i64)
            })
            .unzip();

        let type_chart_json = if type_labels_vec.is_empty() {
            r#"{"labels":["No Data"],"data":[0]}"#.to_string()
        } else {
            let labels_json = serde_json::to_string(&type_labels_vec).unwrap_or_default();
            let data_json   = serde_json::to_string(&type_data_vec).unwrap_or_default();
            format!(r#"{{"labels":{},"data":{}}}"#, labels_json, data_json)
        };
        context.insert("type_chart_json", type_chart_json);

        // ── Platform-wide evidence count (for Crime Billboard card) ───────────
        let platform_total_evidence: i64 = match evidence_service.database.pool.get() {
            Ok(conn) => conn.query_row(
                "SELECT COUNT(*) FROM evidence WHERE status != 'Draft'",
                rusqlite::params![],
                |row| row.get::<_, i64>(0)
            ).unwrap_or(0),
            Err(_) => 0,
        };
        context.insert("platform_total_evidence", platform_total_evidence.to_string());

        // ── Last Submitted / Last Indexed / Last Linked timestamps ────────────
        let last_submitted_at: String = if let Some(ref uid) = user_id {
            match evidence_service.database.pool.get() {
                Ok(conn) => conn.query_row(
                    "SELECT created_at FROM evidence WHERE uploader_id = ? AND status != 'Draft' ORDER BY created_at DESC LIMIT 1",
                    [uid],
                    |row| row.get::<_, String>(0)
                ).ok()
                  .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()
                      .map(|dt| dt.format("%d %b %Y").to_string()))
                  .unwrap_or_else(|| "—".to_string()),
                Err(_) => "—".to_string(),
            }
        } else { "—".to_string() };

        let last_indexed_at: String = if let Some(ref uid) = user_id {
            let user_ev_idx = evidence_service.get_user_evidence(uid).await.unwrap_or_default();
            let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;
            for ev in &user_ev_idx {
                if let Ok(targets) = evidence_service.get_evidence_targets(&ev.id).await {
                    for t in targets {
                        if latest.map_or(true, |l| t.created_at > l) {
                            latest = Some(t.created_at);
                        }
                    }
                }
            }
            latest.map(|dt| dt.format("%d %b %Y").to_string()).unwrap_or_else(|| "—".to_string())
        } else { "—".to_string() };

        let last_linked_at: String = if let Some(ref uid) = user_id {
            match evidence_service.database.pool.get() {
                Ok(conn) => conn.query_row(
                    "SELECT lc.created_at FROM linked_cases lc                      JOIN evidence e ON (e.id = lc.evidence_id_1 OR e.id = lc.evidence_id_2)                      WHERE e.uploader_id = ? ORDER BY lc.created_at DESC LIMIT 1",
                    [uid],
                    |row| row.get::<_, String>(0)
                ).ok()
                  .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()
                      .map(|dt| dt.format("%d %b %Y").to_string()))
                  .unwrap_or_else(|| "—".to_string()),
                Err(_) => "—".to_string(),
            }
        } else { "—".to_string() };

        context.insert("last_submitted_at", last_submitted_at);
        context.insert("last_indexed_at",   last_indexed_at);
        context.insert("last_linked_at",    last_linked_at);

        // ── Target Profiles table (latest 5 rows) ─────────────────────────────
        let target_profiles_html: String = if let Some(ref uid) = user_id {
            let user_ev_tp = evidence_service.get_user_evidence(uid).await.unwrap_or_default();
            let mut rows: Vec<(chrono::DateTime<chrono::Utc>, String)> = Vec::new();

            for ev in &user_ev_tp {
                if let Ok(targets) = evidence_service.get_evidence_targets(&ev.id).await {
                    for target in targets {
                        let confidence_css_class = if target.confidence_score >= 80 {
                            "tg-conf-span-high"
                        } else if target.confidence_score >= 50 {
                            "tg-conf-span-medium"
                        } else {
                            "tg-conf-span-low"
                        };
                        let confidence_bar_color = if target.confidence_score >= 80 {
                            "#22c55e"
                        } else if target.confidence_score >= 50 {
                            "#f59e0b"
                        } else {
                            "#ef4444"
                        };
                        let category_text = match target.category {
                            TargetCategory::Person   => "Person",
                            TargetCategory::Vehicle  => "Vehicle",
                            TargetCategory::Object   => "Object",
                            TargetCategory::Location => "Location",
                            TargetCategory::Other    => "Other",
                        };
                        let desc = target.description.as_deref().unwrap_or("—");
                        let short_desc = if desc.len() > 50 { format!("{}…", &desc[..50]) } else { desc.to_string() };
                        let ev_num_short = if ev.evidence_number.len() > 12 {
                            format!("…{}", &ev.evidence_number[ev.evidence_number.len()-10..])
                        } else { ev.evidence_number.clone() };
                        let county = &ev.county;
                        let incident = format!("{:?}", ev.incident_type);
                        let indexed_on = target.created_at.format("%d %b %Y").to_string();
                        let ev_id = html_escape(&ev.id);

                        // badge class based on confidence level
                        let conf_badge_cls = if target.confidence_score >= 80 { "lc-badge-low" }  // green-ish → reuse low = blue, or use custom
                            else if target.confidence_score >= 50 { "lc-badge-medium" }
                            else { "lc-badge-critical" };
                        let conf_dot_color = confidence_bar_color;

                        // category badge colors
                        let (cat_bg, cat_color, cat_border) = match target.category {
                            TargetCategory::Person   => ("rgba(59,130,246,.1)",  "#2563eb", "rgba(59,130,246,.25)"),
                            TargetCategory::Vehicle  => ("rgba(34,197,94,.1)",   "#15803d", "rgba(34,197,94,.25)"),
                            TargetCategory::Object   => ("rgba(249,115,22,.1)",  "#c2410c", "rgba(249,115,22,.25)"),
                            TargetCategory::Location => ("rgba(20,184,166,.1)",  "#0f766e", "rgba(20,184,166,.25)"),
                            TargetCategory::Other    => ("rgba(148,163,184,.1)", "#475569", "rgba(148,163,184,.25)"),
                        };

                        let row_html = format!(
                            r##"<tr>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;width:44px;">
    <div style="width:32px;height:32px;border-radius:9px;background:rgba(249,115,22,.09);border:1px solid rgba(249,115,22,.2);display:flex;align-items:center;justify-content:center;flex-shrink:0;">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="#f97316" stroke-width="1.8"><circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 3.6-7 8-7s8 3 8 7"/></svg>
    </div>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;">
    <span class="lc-case-link" style="cursor:default;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:100%;display:block;" title="{}">{}</span>
    <span style="display:block;margin-top:3px;font-size:.6rem;font-weight:700;letter-spacing:.06em;font-family:var(--mono,monospace);color:var(--text-3);text-transform:uppercase;">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span style="display:inline-flex;align-items:center;gap:4px;padding:3px 9px;border-radius:9999px;font-size:.65rem;font-weight:700;letter-spacing:.04em;background:{};color:{};border:1px solid {};font-family:var(--mono,monospace);">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <a href="/evidence/view/{}" style="display:inline-flex;align-items:center;gap:4px;padding:3px 9px;border-radius:7px;background:rgba(59,130,246,.1);border:1px solid rgba(59,130,246,.22);font-size:.65rem;font-weight:700;color:#2563eb;text-decoration:none;font-family:var(--mono,monospace);">{}</a>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span style="font-size:.72rem;color:var(--text-2);font-family:var(--mono,monospace);">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span class="lc-county-chip">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <div style="display:flex;align-items:center;gap:7px;">
      <span class="lc-emerg-badge {}"><span class="lc-emerg-dot" style="background:{};"></span>{}%</span>
      <div style="flex:1;min-width:48px;height:4px;border-radius:999px;background:var(--border);overflow:hidden;">
        <div style="height:100%;border-radius:999px;width:{}%;background:{};transition:width .65s ease;"></div>
      </div>
    </div>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;white-space:nowrap;">
    <span style="font-size:.72rem;color:var(--text-2);font-family:var(--mono,monospace);">{}</span>
  </td>
  <td style="padding:12px 16px;vertical-align:middle;overflow:hidden;text-align:center;">
    <a href="/targets" class="lc-act-btn" title="View target profile">
      <svg width="13" height="13" viewBox="0 0 20 20" fill="none"><path fill-rule="evenodd" clip-rule="evenodd" d="M10.875 13.862C8.108 13.862 5.743 12.137 4.798 9.702C5.743 7.268 8.108 5.543 10.875 5.543C13.641 5.543 16.007 7.268 16.952 9.702C16.007 12.137 13.641 13.862 10.875 13.862ZM10.866 7.844C9.84 7.844 9.008 8.676 9.008 9.702C9.008 10.729 9.84 11.561 10.866 11.561H10.881C11.907 11.561 12.739 10.729 12.739 9.702C12.739 8.676 11.907 7.844 10.881 7.844H10.866Z" fill="currentColor"/></svg>
    </a>
  </td>
</tr>"##,
                            short_desc, short_desc,
                            category_text,
                            cat_bg, cat_color, cat_border, category_text,
                            ev_id, ev_num_short,
                            incident,
                            county,
                            conf_badge_cls, conf_dot_color, target.confidence_score,
                            target.confidence_score, confidence_bar_color,
                            indexed_on,
                        );
                        rows.push((target.created_at, row_html));
                    }
                }
            }

            rows.sort_by(|a, b| b.0.cmp(&a.0));
            rows.truncate(5);

            if rows.is_empty() {
                r#"<tr><td colspan="9" style="padding:40px;text-align:center;color:var(--text-3);font-size:13px;font-family:var(--mono);">No target profiles indexed yet</td></tr>"#.to_string()
            } else {
                rows.into_iter().map(|(_, html)| html).collect::<Vec<_>>().join("\n")
            }
        } else {
            r#"<tr><td colspan="9" style="padding:40px;text-align:center;color:var(--text-3);font-size:13px;font-family:var(--mono);">Please log in to view target profiles</td></tr>"#.to_string()
        };

        context.insert("target_profiles_html", target_profiles_html);

        let html = render_template("evidence_dashboard", &context);
        HttpResponse::Ok().body(html)
    } else {
        println!("🎬 EVIDENCE_DASHBOARD: No session - redirecting to /login");
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}



pub async fn evidence_browse_page(
    session: Session,
    auth_service: web::Data<AuthService>,
    evidence_service: web::Data<EvidenceService>,
    req: HttpRequest,
) -> HttpResponse {
    println!("🌐 EVIDENCE_BROWSE_PAGE: Loading browse page");
    
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🌐 EVIDENCE_BROWSE_PAGE: User email: {}", email);
        
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("🌐 EVIDENCE_BROWSE_PAGE: Profile not complete, redirecting");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
            }
            _ => {
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        let user_id = session.get::<String>("user_id").unwrap_or(None);
        let query_str = req.query_string();
        println!("🌐 EVIDENCE_BROWSE_PAGE: Query string: {}", query_str);
        
        let mut query_params = HashMap::new();
        for param in query_str.split('&') {
            if param.is_empty() { continue; }
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 {
                query_params.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        
        println!("🌐 EVIDENCE_BROWSE_PAGE: Query params: {:?}", query_params);
        
        let filters = EvidenceSearchFilters {
            query: query_params.get("q").cloned(),
            incident_type: query_params.get("incident_type").cloned(),
            county: query_params.get("county").cloned(),
            emergency_level: query_params.get("emergency_level").cloned(),
            status: query_params.get("status").cloned(),
            reported_to_police: query_params.get("reported_to_police").map(|v| v == "true"),
            needs_attention: query_params.get("needs_attention").map(|v| v == "true"),
            signed_only: query_params.get("signed_only").map(|v| v == "true"),
            uploader_id: if query_params.get("mine").map(|v| v == "true").unwrap_or(false) {
                user_id.clone()
            } else {
                None
            },
            date_from: query_params.get("date_from").cloned(),
            date_to: query_params.get("date_to").cloned(),
            start_date: query_params.get("start_date").cloned(),
            end_date: query_params.get("end_date").cloned(),
            sort_by: Some(query_params.get("sort_by").cloned().unwrap_or_else(|| "newest".to_string())),
            page: 1,
            limit: 100,
        };
        
        println!("🌐 EVIDENCE_BROWSE_PAGE: Search filters: {:?}", filters);
        
        let search_result = match evidence_service.search_evidence_with_filters(&filters, user_id.as_deref().unwrap_or("")).await {
            Ok(result) => {
                println!("🌐 EVIDENCE_BROWSE_PAGE: Search successful, found {} items", result.total);
                result
            }
            Err(e) => {
                println!("🌐 EVIDENCE_BROWSE_PAGE: Error searching evidence: {}", e);
                EvidenceSearchResponse {
                    evidence: Vec::new(),
                    summaries: Vec::new(),
                    total: 0,
                    page: 1,
                    total_pages: 1,
                }
            }
        };
        
        let kenya_counties = vec![
            "Nairobi", "Mombasa", "Kisumu", "Nakuru", "Eldoret", "Thika", "Malindi", "Kitale",
            "Garissa", "Kakamega", "Kisii", "Meru", "Nyeri", "Machakos", "Kiambu", "Kilifi",
            "Bungoma", "Busia", "Embu", "Homa Bay", "Isiolo", "Kajiado", "Kericho", "Kirinyaga",
            "Kitui", "Kwale", "Laikipia", "Lamu", "Mandera", "Marsabit", "Migori", "Murang'a",
            "Nyamira", "Nyandarua", "Narok", "Samburu", "Siaya", "Taita Taveta", "Tana River",
            "Trans Nzoia", "Turkana", "Uasin Gishu", "Vihiga", "Wajir", "West Pokot"
        ];
        
        let incident_types = vec![
            "HitAndRun", "Assault", "ThreatToLife", "PropertyDamage", "Theft", "Other"
        ];
        
        let emergency_levels = vec![
            ("red", "Red - Emergency"),
            ("orange", "Orange - High"),
            ("yellow", "Yellow - Medium"),
            ("blue", "Blue - Low")
        ];
        
        let statuses = vec![
            ("draft", "Draft"),
            ("submitted", "Submitted"),
            ("reported", "Reported to Police"),
            ("under_review", "Under Review"),
            ("archived", "Archived"),
            ("rejected", "Rejected")
        ];
        
        // Generate Kanban cards instead of table rows
        let evidence_cards = if search_result.summaries.is_empty() {
            println!("🌐 EVIDENCE_BROWSE_PAGE: No evidence found, showing empty state");
            r#"
            <div class="col-span-3 px-6 py-12 text-center">
                <div class="inline-block p-8 bg-gray-900/50 rounded-lg">
                    <i class="fas fa-file-alt text-4xl text-gray-700 mb-4"></i>
                    <h3 class="text-xl font-bold mb-2">No Evidence Found</h3>
                    <p class="text-gray-400 mb-4">No evidence has been uploaded yet.</p>
                    <a href="/evidence/upload" class="inline-flex items-center bg-red-600 px-6 py-3 rounded-lg hover:bg-red-700">
                        <i class="fas fa-upload mr-2"></i>
                        Upload Your First Evidence
                    </a>
                </div>
            </div>
            "#.to_string()
        } else {
            println!("🌐 EVIDENCE_BROWSE_PAGE: Generating kanban cards with {} items", search_result.summaries.len());
            
            // Categorize evidence for the 3 swim lanes
            let mut urgent_evidence: Vec<&EvidenceSummary> = Vec::new();
            let mut in_progress_evidence: Vec<&EvidenceSummary> = Vec::new();
            let mut resolved_evidence: Vec<&EvidenceSummary> = Vec::new();
            
            for evidence in search_result.summaries.iter().take(15) {
                // Categorize based on emergency level and status
                match evidence.emergency_level {
                    EmergencyLevel::Red | EmergencyLevel::Orange => {
                        urgent_evidence.push(evidence);
                    }
                    _ => {
                        match evidence.status {
                            EvidenceStatus::Submitted | EvidenceStatus::UnderReview | EvidenceStatus::Draft => {
                                in_progress_evidence.push(evidence);
                            }
                            EvidenceStatus::Reported | EvidenceStatus::Archived | EvidenceStatus::Rejected => {
                                resolved_evidence.push(evidence);
                            }
                            _ => {
                                // Default to in-progress for unknown status
                                in_progress_evidence.push(evidence);
                            }
                        }
                    }
                }
            }
            
            // Limit each lane to max 5 items
            urgent_evidence.truncate(5);
            in_progress_evidence.truncate(5);
            resolved_evidence.truncate(5);
            
            let mut cards_html = String::new();
            
            // Lane 1: Urgent (Red/Orange emergency level)
            cards_html.push_str(&format!(r#"
                <div class="swim-lane flex flex-col gap-5 border-x border-gray-200 p-4 dark:border-gray-800 xl:p-6">
                    <div class="mb-1 flex items-center justify-between">
                        <h3 class="flex items-center gap-3 text-base font-medium text-gray-800 dark:text-white/90">
                            Urgent Cases
                            <span class="inline-flex rounded-full bg-red-50 px-2 py-0.5 text-theme-xs font-medium text-red-700 dark:bg-red-500/15 dark:text-red-400">
                                {}
                            </span>
                        </h3>

                        <div x-data="{{openDropDown: false}}" class="relative">
                            <button @click="openDropDown = !openDropDown" class="text-gray-700 dark:text-gray-400">
                                <svg class="fill-current" width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                                    <path fill-rule="evenodd" clip-rule="evenodd" d="M5.99902 10.2451C6.96552 10.2451 7.74902 11.0286 7.74902 11.9951V12.0051C7.74902 12.9716 6.96552 13.7551 5.99902 13.7551C5.03253 13.7551 4.24902 12.9716 4.24902 12.0051V11.9951C4.24902 11.0286 5.03253 10.2451 5.99902 10.2451ZM17.999 10.2451C18.9655 10.2451 19.749 11.0286 19.749 11.9951V12.0051C19.749 12.9716 18.9655 13.7551 17.999 13.7551C17.0325 13.7551 16.249 12.9716 16.249 12.0051V11.9951C16.249 11.0286 17.0325 10.2451 17.999 10.2451ZM13.749 11.9951C13.749 11.0286 12.9655 10.2451 11.999 10.2451C11.0325 10.2451 10.249 11.0286 10.249 11.9951V12.0051C10.249 12.9716 11.0325 13.7551 11.999 13.7551C12.9655 13.7551 13.749 12.9716 13.749 12.0051V11.9951Z"></path>
                                </svg>
                            </button>
                            <div x-show="openDropDown" @click.outside="openDropDown = false" class="absolute right-0 top-full z-40 w-[140px] space-y-1 rounded-2xl border border-gray-200 bg-white p-2 shadow-theme-md dark:border-gray-800 dark:bg-gray-dark" style="display: none;">
                                <button class="flex w-full rounded-lg px-3 py-2 text-left text-theme-xs font-medium text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-white/5 dark:hover:text-gray-300">
                                    Filter
                                </button>
                                <button class="flex w-full rounded-lg px-3 py-2 text-left text-theme-xs font-medium text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-white/5 dark:hover:text-gray-300">
                                    Sort
                                </button>
                            </div>
                        </div>
                    </div>
            "#, urgent_evidence.len()));
            
            // Urgent cards
            for evidence in urgent_evidence {
                let emergency_badge = match evidence.emergency_level {
                    EmergencyLevel::Red => r#"<span class="inline-flex rounded-full bg-red-50 px-2 py-0.5 text-theme-xs font-medium text-red-700 dark:bg-red-500/15 dark:text-red-400">
                        RED ALERT
                    </span>"#,
                    EmergencyLevel::Orange => r#"<span class="inline-flex rounded-full bg-orange-50 px-2 py-0.5 text-theme-xs font-medium text-orange-700 dark:bg-orange-500/15 dark:text-orange-400">
                        HIGH PRIORITY
                    </span>"#,
                    _ => ""
                };
                
                let date_display = evidence.incident_time.format("%b %d, %Y").to_string();
                
                cards_html.push_str(&format!(r#"
                    <!-- urgent evidence card -->
                    <div draggable="true" class="task rounded-xl border border-gray-200 bg-white p-5 shadow-theme-sm dark:border-gray-800 dark:bg-white/5">
                        <div class="flex items-start justify-between gap-6">
                            <div class="flex-1">
                                <div class="mb-3">
                                    {}
                                </div>
                                <h4 class="mb-3 text-base text-gray-800 dark:text-white/90">
                                    {}
                                </h4>
                                
                                <div class="flex items-center gap-3 mb-3">
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="fill-current" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path fill-rule="evenodd" clip-rule="evenodd" d="M5.33329 1.0835C5.74751 1.0835 6.08329 1.41928 6.08329 1.8335V2.25016L9.91663 2.25016V1.8335C9.91663 1.41928 10.2524 1.0835 10.6666 1.0835C11.0808 1.0835 11.4166 1.41928 11.4166 1.8335V2.25016L12.3333 2.25016C13.2998 2.25016 14.0833 3.03366 14.0833 4.00016V6.00016L14.0833 12.6668C14.0833 13.6333 13.2998 14.4168 12.3333 14.4168L3.66663 14.4168C2.70013 14.4168 1.91663 13.6333 1.91663 12.6668L1.91663 6.00016L1.91663 4.00016C1.91663 3.03366 2.70013 2.25016 3.66663 2.25016L4.58329 2.25016V1.8335C4.58329 1.41928 4.91908 1.0835 5.33329 1.0835ZM5.33329 3.75016L3.66663 3.75016C3.52855 3.75016 3.41663 3.86209 3.41663 4.00016V5.25016L12.5833 5.25016V4.00016C12.5833 3.86209 12.4714 3.75016 12.3333 3.75016L10.6666 3.75016L5.33329 3.75016ZM12.5833 6.75016L3.41663 6.75016L3.41663 12.6668C3.41663 12.8049 3.52855 12.9168 3.66663 12.9168L12.3333 12.9168C12.4714 12.9168 12.5833 12.8049 12.5833 12.6668L12.5833 6.75016Z"></path>
                                        </svg>
                                        {}
                                    </span>
                                    
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="stroke-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path d="M9 15.6343C12.6244 15.6343 15.5625 12.6961 15.5625 9.07178C15.5625 5.44741 12.6244 2.50928 9 2.50928C5.37563 2.50928 2.4375 5.44741 2.4375 9.07178C2.4375 10.884 3.17203 12.5246 4.35961 13.7122L2.4375 15.6343H9Z" stroke="" stroke-width="1.5" stroke-linejoin="round"></path>
                                        </svg>
                                        #{}
                                    </span>
                                </div>
                                
                                <span class="inline-flex rounded-full bg-gray-100 px-2 py-0.5 text-theme-xs font-medium text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                                    {}
                                </span>
                            </div>
                            
                            <div class="flex flex-col items-end gap-2">
                                <a href="/evidence/view/{}" class="p-1.5 text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors">
                                    <svg class="w-4 h-4 fill-current" viewBox="0 0 20 20">
                                        <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"/>
                                        <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"/>
                                    </svg>
                                </a>
                            </div>
                        </div>
                    </div>
                "#, emergency_badge, 
                   html_escape(&if evidence.title.len() > 60 { format!("{}...", &evidence.title[..57]) } else { evidence.title.clone() }),
                   date_display,
                   html_escape(&evidence.evidence_number),
                   html_escape(&evidence.county),
                   html_escape(&evidence.id)));
            }
            
            cards_html.push_str("</div>");
            
            // Lane 2: In Progress (Submitted/Under Review/Draft)
            cards_html.push_str(&format!(r#"
                <div class="swim-lane flex flex-col gap-5 border-x border-gray-200 p-4 dark:border-gray-800 xl:p-6">
                    <div class="mb-1 flex items-center justify-between">
                        <h3 class="flex items-center gap-3 text-base font-medium text-gray-800 dark:text-white/90">
                            In Progress
                            <span class="inline-flex rounded-full bg-blue-50 px-2 py-0.5 text-theme-xs font-medium text-blue-700 dark:bg-blue-500/15 dark:text-blue-400">
                                {}
                            </span>
                        </h3>
                    </div>
            "#, in_progress_evidence.len()));
            
            // In Progress cards
            for evidence in in_progress_evidence {
                let status_badge = match evidence.status {
                    EvidenceStatus::Submitted => r#"<span class="inline-flex rounded-full bg-blue-50 px-2 py-0.5 text-theme-xs font-medium text-blue-700 dark:bg-blue-500/15 dark:text-blue-400">
                        SUBMITTED
                    </span>"#,
                    EvidenceStatus::UnderReview => r#"<span class="inline-flex rounded-full bg-yellow-50 px-2 py-0.5 text-theme-xs font-medium text-yellow-700 dark:bg-yellow-500/15 dark:text-yellow-400">
                        UNDER REVIEW
                    </span>"#,
                    EvidenceStatus::Draft => r#"<span class="inline-flex rounded-full bg-gray-100 px-2 py-0.5 text-theme-xs font-medium text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                        DRAFT
                    </span>"#,
                    _ => ""
                };
                
                let date_display = evidence.incident_time.format("%b %d, %Y").to_string();
                
                // Media files are not available on EvidenceSummary — full URLs only
                // exist on the complete Evidence object. The view page (one click away)
                // shows all media properly, so we keep cards clean here.
                let media_section = String::new();
                
                cards_html.push_str(&format!(r#"
                    <!-- in progress evidence card -->
                    <div draggable="true" class="task rounded-xl border border-gray-200 bg-white p-5 shadow-theme-sm dark:border-gray-800 dark:bg-white/5">
                        <div>
                            <div class="mb-3">
                                {}
                            </div>
                            <h4 class="mb-2 text-base text-gray-800 dark:text-white/90">
                                {}
                            </h4>
                            
                            {}
                            
                            <div class="flex items-start justify-between gap-6">
                                <div class="flex items-center gap-3">
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="fill-current" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path fill-rule="evenodd" clip-rule="evenodd" d="M5.33329 1.0835C5.74751 1.0835 6.08329 1.41928 6.08329 1.8335V2.25016L9.91663 2.25016V1.8335C9.91663 1.41928 10.2524 1.0835 10.6666 1.0835C11.0808 1.0835 11.4166 1.41928 11.4166 1.8335V2.25016L12.3333 2.25016C13.2998 2.25016 14.0833 3.03366 14.0833 4.00016V6.00016L14.0833 12.6668C14.0833 13.6333 13.2998 14.4168 12.3333 14.4168L3.66663 14.4168C2.70013 14.4168 1.91663 13.6333 1.91663 12.6668L1.91663 6.00016L1.91663 4.00016C1.91663 3.03366 2.70013 2.25016 3.66663 2.25016L4.58329 2.25016V1.8335C4.58329 1.41928 4.91908 1.0835 5.33329 1.0835ZM5.33329 3.75016L3.66663 3.75016C3.52855 3.75016 3.41663 3.86209 3.41663 4.00016V5.25016L12.5833 5.25016V4.00016C12.5833 3.86209 12.4714 3.75016 12.3333 3.75016L10.6666 3.75016L5.33329 3.75016ZM12.5833 6.75016L3.41663 6.75016L3.41663 12.6668C3.41663 12.8049 3.52855 12.9168 3.66663 12.9168L12.3333 12.9168C12.4714 12.9168 12.5833 12.8049 12.5833 12.6668L12.5833 6.75016Z"></path>
                                        </svg>
                                        {}
                                    </span>
                                    
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="stroke-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path d="M9 15.6343C12.6244 15.6343 15.5625 12.6961 15.5625 9.07178C15.5625 5.44741 12.6244 2.50928 9 2.50928C5.37563 2.50928 2.4375 5.44741 2.4375 9.07178C2.4375 10.884 3.17203 12.5246 4.35961 13.7122L2.4375 15.6343H9Z" stroke="" stroke-width="1.5" stroke-linejoin="round"></path>
                                        </svg>
                                        #{}
                                    </span>
                                </div>
                                
                                <div class="flex items-center gap-1">
                                    <a href="/evidence/view/{}" class="p-1.5 text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors">
                                        <svg class="w-4 h-4 fill-current" viewBox="0 0 20 20">
                                            <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"/>
                                            <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"/>
                                        </svg>
                                    </a>
                                </div>
                            </div>
                        </div>
                    </div>
                "#, status_badge,
                   html_escape(&if evidence.title.len() > 60 { format!("{}...", &evidence.title[..57]) } else { evidence.title.clone() }),
                   media_section,
                   date_display,
                   html_escape(&evidence.evidence_number),
                   html_escape(&evidence.id)));
            }
            
            cards_html.push_str("</div>");
            
            // Lane 3: Resolved (Reported/Archived/Rejected)
            cards_html.push_str(&format!(r#"
                <div class="swim-lane flex flex-col gap-5 border-x border-gray-200 p-4 dark:border-gray-800 xl:p-6">
                    <div class="mb-1 flex items-center justify-between">
                        <h3 class="flex items-center gap-3 text-base font-medium text-gray-800 dark:text-white/90">
                            Resolved Cases
                            <span class="inline-flex rounded-full bg-green-50 px-2 py-0.5 text-theme-xs font-medium text-green-700 dark:bg-green-500/15 dark:text-green-400">
                                {}
                            </span>
                        </h3>
                    </div>
            "#, resolved_evidence.len()));
            
            // Resolved cards
            for evidence in resolved_evidence {
                let resolved_badge = match evidence.status {
                    EvidenceStatus::Reported => r#"<span class="inline-flex rounded-full bg-green-50 px-2 py-0.5 text-theme-xs font-medium text-green-700 dark:bg-green-500/15 dark:text-green-400">
                        REPORTED TO POLICE
                    </span>"#,
                    EvidenceStatus::Archived => r#"<span class="inline-flex rounded-full bg-gray-100 px-2 py-0.5 text-theme-xs font-medium text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                        ARCHIVED
                    </span>"#,
                    EvidenceStatus::Rejected => r#"<span class="inline-flex rounded-full bg-red-50 px-2 py-0.5 text-theme-xs font-medium text-red-700 dark:bg-red-500/15 dark:text-red-400">
                        REJECTED
                    </span>"#,
                    _ => ""
                };
                
                let date_display = evidence.incident_time.format("%b %d, %Y").to_string();
                let police_status = if evidence.reported_to_police {
                    evidence.police_case_id.as_ref()
                        .map(|id| format!("Case #{}", id))
                        .unwrap_or_else(|| "Police Notified".to_string())
                } else {
                    "Not Reported".to_string()
                };
                
                cards_html.push_str(&format!(r#"
                    <!-- resolved evidence card -->
                    <div draggable="true" class="task rounded-xl border border-gray-200 bg-white p-5 shadow-theme-sm dark:border-gray-800 dark:bg-white/5">
                        <div class="flex items-start justify-between gap-6">
                            <div class="flex-1">
                                <div class="mb-3">
                                    {}
                                </div>
                                <h4 class="mb-3 text-base text-gray-800 dark:text-white/90">
                                    {}
                                </h4>
                                
                                <div class="flex items-center gap-3 mb-3">
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="fill-current" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path fill-rule="evenodd" clip-rule="evenodd" d="M5.33329 1.0835C5.74751 1.0835 6.08329 1.41928 6.08329 1.8335V2.25016L9.91663 2.25016V1.8335C9.91663 1.41928 10.2524 1.0835 10.6666 1.0835C11.0808 1.0835 11.4166 1.41928 11.4166 1.8335V2.25016L12.3333 2.25016C13.2998 2.25016 14.0833 3.03366 14.0833 4.00016V6.00016L14.0833 12.6668C14.0833 13.6333 13.2998 14.4168 12.3333 14.4168L3.66663 14.4168C2.70013 14.4168 1.91663 13.6333 1.91663 12.6668L1.91663 6.00016L1.91663 4.00016C1.91663 3.03366 2.70013 2.25016 3.66663 2.25016L4.58329 2.25016V1.8335C4.58329 1.41928 4.91908 1.0835 5.33329 1.0835ZM5.33329 3.75016L3.66663 3.75016C3.52855 3.75016 3.41663 3.86209 3.41663 4.00016V5.25016L12.5833 5.25016V4.00016C12.5833 3.86209 12.4714 3.75016 12.3333 3.75016L10.6666 3.75016L5.33329 3.75016ZM12.5833 6.75016L3.41663 6.75016L3.41663 12.6668C3.41663 12.8049 3.52855 12.9168 3.66663 12.9168L12.3333 12.9168C12.4714 12.9168 12.5833 12.8049 12.5833 12.6668L12.5833 6.75016Z"></path>
                                        </svg>
                                        {}
                                    </span>
                                    
                                    <span class="flex cursor-pointer items-center gap-1 text-sm text-gray-500 dark:text-gray-400">
                                        <svg class="fill-current" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                            <path fill-rule="evenodd" clip-rule="evenodd" d="M2.66667 1.8335C2.66667 1.41928 3.00246 1.0835 3.41667 1.0835H12.5833C12.9975 1.0835 13.3333 1.41928 13.3333 1.8335V14.1668C13.3333 14.581 12.9975 14.9168 12.5833 14.9168H3.41667C3.00246 14.9168 2.66667 14.581 2.66667 14.1668V1.8335ZM4.25 2.5835V13.4168H11.75V2.5835H4.25ZM6.41667 4.25016C6.41667 3.83595 6.75246 3.50016 7.16667 3.50016H8.83333C9.24755 3.50016 9.58333 3.83595 9.58333 4.25016V5.91683C9.58333 6.33104 9.24755 6.66683 8.83333 6.66683H7.16667C6.75246 6.66683 6.41667 6.33104 6.41667 5.91683V4.25016Z"></path>
                                        </svg>
                                        {}
                                    </span>
                                </div>
                                
                                <span class="inline-flex rounded-full bg-gray-100 px-2 py-0.5 text-theme-xs font-medium text-gray-700 dark:bg-gray-800 dark:text-gray-300">
                                    {}
                                </span>
                            </div>
                            
                            <div class="flex flex-col items-end gap-2">
                                <a href="/evidence/view/{}" class="p-1.5 text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors">
                                    <svg class="w-4 h-4 fill-current" viewBox="0 0 20 20">
                                        <path d="M10 12a2 2 0 100-4 2 2 0 000 4z"/>
                                        <path fill-rule="evenodd" d="M.458 10C1.732 5.943 5.522 3 10 3s8.268 2.943 9.542 7c-1.274 4.057-5.064 7-9.542 7S1.732 14.057.458 10zM14 10a4 4 0 11-8 0 4 4 0 018 0z" clip-rule="evenodd"/>
                                    </svg>
                                </a>
                            </div>
                        </div>
                    </div>
                "#, resolved_badge,
                   html_escape(&if evidence.title.len() > 60 { format!("{}...", &evidence.title[..57]) } else { evidence.title.clone() }),
                   date_display,
                   police_status,
                   match evidence.incident_type {
                       IncidentType::HitAndRun      => "Hit & Run",
                       IncidentType::Assault        => "Assault",
                       IncidentType::ThreatToLife   => "Threat to Life",
                       IncidentType::PropertyDamage => "Property Damage",
                       IncidentType::Theft          => "Theft",
                       IncidentType::Other          => "Other",
                   },
                   html_escape(&evidence.id)));
            }
            
            cards_html.push_str("</div>");
            
            cards_html
        };
        
        let search_query = query_params.get("q").unwrap_or(&String::new()).clone();
        let selected_county = query_params.get("county").unwrap_or(&String::new()).clone();
        let selected_incident_type = query_params.get("incident_type").unwrap_or(&String::new()).clone();
        let selected_emergency_level = query_params.get("emergency_level").unwrap_or(&String::new()).clone();
        let selected_status = query_params.get("status").unwrap_or(&String::new()).clone();
        
        let total_evidence = search_result.total;
        let urgent_count = search_result.summaries.iter().filter(|e| 
            matches!(e.emergency_level, EmergencyLevel::Red | EmergencyLevel::Orange)
        ).count();
        let reported_count = search_result.summaries.iter().filter(|e| e.reported_to_police).count();
        let needs_attention_count = search_result.summaries.iter().filter(|e| e.needs_attention).count();
        
        let county_options: String = kenya_counties.iter().map(|county| {
            if selected_county == *county {
                format!("<option value=\"{}\" selected>{}</option>", county, county)
            } else {
                format!("<option value=\"{}\">{}</option>", county, county)
            }
        }).collect();
        
        let incident_type_options: String = incident_types.iter().map(|inc_type| {
            if selected_incident_type == *inc_type {
                format!("<option value=\"{}\" selected>{}</option>", inc_type, inc_type)
            } else {
                format!("<option value=\"{}\">{}</option>", inc_type, inc_type)
            }
        }).collect();
        
        let emergency_level_options: String = emergency_levels.iter().map(|(value, label)| {
            if selected_emergency_level == *value {
                format!("<option value=\"{}\" selected>{}</option>", value, label)
            } else {
                format!("<option value=\"{}\">{}</option>", value, label)
            }
        }).collect();
        
        let status_options: String = statuses.iter().map(|(value, label)| {
            if selected_status == *value {
                format!("<option value=\"{}\" selected>{}</option>", value, label)
            } else {
                format!("<option value=\"{}\">{}</option>", value, label)
            }
        }).collect();
        
        let mut context = HashMap::new();
        context.insert("email", email);
        context.insert("total_evidence", total_evidence.to_string());
        context.insert("search_query", search_query);
        context.insert("urgent_count", urgent_count.to_string());
        context.insert("reported_count", reported_count.to_string());
        context.insert("needs_attention_count", needs_attention_count.to_string());
        context.insert("county_options", county_options);
        context.insert("incident_type_options", incident_type_options);
        context.insert("emergency_level_options", emergency_level_options);
        context.insert("status_options", status_options);
        context.insert("evidence_cards", evidence_cards);
        
        let html = render_template("evidence_browse", &context);
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

pub async fn evidence_upload_page(
    session: Session,
    auth_service: web::Data<AuthService>,
    _evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        let user = auth_service.get_session_user(&email).await.ok().flatten();
        let has_wallet = user.as_ref().map(|u| u.has_wallet).unwrap_or(false);
        
        let wallet_status = if has_wallet {
            format!(r#"
            <div class="flex flex-col justify-between gap-6 rounded-2xl border border-gray-200 bg-white px-6 py-5 sm:flex-row sm:items-center dark:border-gray-800 dark:bg-white/3">
                <div class="flex flex-col gap-2.5 divide-gray-300 sm:flex-row sm:divide-x dark:divide-gray-700">
                  <div class="flex items-center gap-2 sm:pr-3">
                    <span class="text-base font-medium text-gray-700 dark:text-gray-400">
                      Blockchain Wallet
                      </span>
                    <span class="bg-success-50 text-success-600 dark:bg-success-500/15 dark:text-success-500 inline-flex items-center justify-center gap-1 rounded-full px-2.5 py-0.5 text-sm font-medium"> Connected </span>
                  </div>
                  <p class="text-sm text-gray-500 sm:pl-3 dark:text-gray-400">
                    Your evidence will be cryptographically signed.
                  </p>
                </div>
                <div class="flex gap-3">
                  <button class="bg-brand-500 shadow-theme-xs hover:bg-brand-600 inline-flex items-center justify-center gap-2 rounded-lg px-4 py-3 text-sm font-medium text-white transition">
                    Connect 
                  </button>
                  <button class="shadow-theme-xs inline-flex items-center justify-center gap-2 rounded-lg bg-white px-4 py-3 text-sm font-medium text-gray-700 ring-1 ring-gray-300 transition hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-400 dark:ring-gray-700 dark:hover:bg-white/[0.03]">
                    Hide
                  </button>
                </div>
            </div> 
            "#)
        } else {
            format!(r#"
   <div class="flex flex-col justify-between gap-6 rounded-2xl border border-gray-200 bg-white px-6 py-5 sm:flex-row sm:items-center dark:border-gray-800 dark:bg-white/3">
                <div class="flex flex-col gap-2.5 divide-gray-300 sm:flex-row sm:divide-x dark:divide-gray-700">
                  <div class="flex items-center gap-2 sm:pr-3">
                    <span class="text-base font-medium text-gray-700 dark:text-gray-400">
                      Connect Your Blockchain Wallet 
                      </span>
                    <span class="bg-warning-50 text-warning-600 dark:bg-warning-500/15 dark:text-warning-500 inline-flex items-center justify-center gap-1 rounded-full px-2.5 py-0.5 text-sm font-medium"> Not Connected </span>
                  </div>
                  <p class="text-sm text-gray-500 sm:pl-3 dark:text-gray-400">
                    Validated crime scene(s) require cryptographic signing for privacy , anonymity and monetization 
                  </p>
                </div>
              
                <div class="flex gap-3">
                  <a href="/connect-wallet">
                  <button class="bg-brand-500 shadow-theme-xs hover:bg-brand-600 inline-flex items-center justify-center gap-2 rounded-lg px-4 py-3 text-sm font-medium text-white transition">
                    Connect 
                  </button>
                  </a>
                  <button class="shadow-theme-xs inline-flex items-center justify-center gap-2 rounded-lg bg-white px-4 py-3 text-sm font-medium text-gray-700 ring-1 ring-gray-300 transition hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-400 dark:ring-gray-700 dark:hover:bg-white/[0.03]">
                    Learn More 
                  </button>
                </div>
    </div> 

  
               
            "#)
        };
        
        let upload_js          = include_str!("../static/js/evidence_upload.js");
        let frame_extractor_js = include_str!("../static/js/frame_extractor.js");
        
        let mut context = HashMap::new();
        context.insert("email", email);
        context.insert("wallet_status", wallet_status);
        context.insert("has_wallet", if has_wallet { "true" } else { "false" }.to_string());
        context.insert("upload_js", upload_js.to_string());
        context.insert("frame_extractor_js", frame_extractor_js.to_string());
        
        let html = render_template("evidence_upload", &context);
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}



pub async fn evidence_complete_page(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let email = session.get::<String>("user_email").unwrap_or(None);
    
    println!("📝 EVIDENCE_COMPLETE_PAGE: Loading evidence ID: {}", evidence_id);
    
    if let Some(email) = email {
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(_user)) => {
                println!("📝 User found: {}", _user.id);
            }
            Ok(None) => {
                println!("📝 User not found in DB");
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
            Err(e) => {
                println!("📝 Error getting user: {}", e);
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        let evidence_result = evidence_service.get_evidence(&evidence_id, false).await;
        let evidence = match evidence_result {
            Ok(Some(evidence)) => {
                println!("✅ Found evidence: {}", evidence.evidence_number);
                evidence
            },
            Ok(None) => {
                println!("❌ Evidence not found: {}", evidence_id);
                return HttpResponse::NotFound().body("Evidence not found");
            },
            Err(e) => {
                println!("❌ Error loading evidence: {}", e);
                return HttpResponse::InternalServerError().body("Failed to load evidence");
            },
        };
        
        let user_id = session.get::<String>("user_id").unwrap_or(None);
        if user_id.as_ref().map_or(false, |uid| uid != &evidence.uploader_id) {
            println!("❌ User doesn't own this evidence");
            return HttpResponse::Forbidden().body("You don't own this evidence");
        }
        
        if evidence.status != EvidenceStatus::Draft {
            println!("⚠️ Evidence already submitted, redirecting to view");
            return HttpResponse::SeeOther()
                .append_header(("Location", format!("/evidence/view/{}", evidence_id)))
                .finish();
        }
        
        let media_preview_html = if evidence.media_files.is_empty() {
            String::new()
        } else {
            let mut tabs_html = String::new();
            let mut content_html = String::new();
            
            // Generate a unique ID for this tab component to avoid Alpine.js conflicts
            let tab_component_id = format!("media_tabs_{}", evidence_id);
            
            for (i, media) in evidence.media_files.iter().enumerate() {
                let tab_id = format!("media_{}", i);
                
                let is_active = i == 0;
                let tab_name = if evidence.media_files.len() == 1 {
                    media.filename.clone()
                } else {
                    format!("Video {}", i + 1)
                };
                
                // Shorten filename for tab display (keep first 20 chars)
                let display_name = if tab_name.len() > 30 {
                    format!("{}...", &tab_name[..30])
                } else {
                    tab_name.clone()
                };
                
                // Tab button
                tabs_html.push_str(&format!(r#"
                    <button class="inline-flex items-center border-b-2 px-2.5 py-2 text-sm font-medium transition-colors duration-200 ease-in-out {}" 
                            x-bind:class="activeTab === '{}' ? 'text-brand-500 dark:text-brand-400 border-brand-500 dark:border-brand-400' : 'bg-transparent text-gray-500 border-transparent hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'" 
                            x-on:click="activeTab = '{}'">
                        {}
                    </button>
                "#, 
                if is_active { "text-brand-500 dark:text-brand-400 border-brand-500 dark:border-brand-400" } else { "bg-transparent text-gray-500 border-transparent hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200" },
                tab_id,
                tab_id,
                display_name
                ));
                
                // Determine if it's a video or other file type
                let is_video = media.mime_type.starts_with("video/") || 
                            media.filename.ends_with(".mp4") || 
                            media.filename.ends_with(".webm") ||
                            media.filename.ends_with(".mov");
                
                let media_content = if is_video {
                    format!(r#"
                        <div class="aspect-video bg-black rounded-lg overflow-hidden">
                            <video src="{}" controls crossorigin="anonymous" class="w-full h-full object-contain"></video>
                        </div>
                        <div class="mt-3 p-3 bg-gray-50 dark:bg-gray-900 rounded-lg">
                            <div class="text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">File Information</div>
                            <div class="text-xs text-gray-500 dark:text-gray-400 space-y-1">
                                <div class="flex justify-between">
                                    <span>Filename:</span>
                                    <span class="font-mono truncate max-w-xs">{}</span>
                                </div>
                                <div class="flex justify-between">
                                    <span>Type:</span>
                                    <span>Video ({})</span>
                                </div>
                            </div>
                        </div>
                    "#, html_escape(&media.storj_url), html_escape(&media.filename), html_escape(&media.mime_type))
                } else {
                    format!(r#"
                        <div class="aspect-video bg-gray-100 dark:bg-gray-900 rounded-lg flex flex-col items-center justify-center p-6">
                            <i class="fas fa-file text-4xl text-gray-400 dark:text-gray-600 mb-3"></i>
                            <div class="text-lg font-medium text-gray-700 dark:text-gray-300 mb-1">File Attachment</div>
                            <div class="text-sm text-gray-500 dark:text-gray-400 text-center">
                                <div class="font-mono break-all">{}</div>
                                <div class="mt-1">Type: {}</div>
                            </div>
                        </div>
                    "#, html_escape(&media.filename), html_escape(&media.mime_type))
                };
                
                // Tab content
                content_html.push_str(&format!(r#"
                    <div class="space-y-4" x-show="activeTab === '{}'" {} >
                        {}
                    </div>
                "#,
                tab_id,
                if !is_active { "style=\"display: none;\"" } else { "" },
                media_content
                ));
            }
            
            format!(r#"
                <div class="rounded-xl border border-gray-200 p-6 dark:border-gray-800" 
                    x-data="{{ activeTab: '{}' }}" 
                    id="{}">
                    <div class="border-b border-gray-200 dark:border-gray-800">
                        <nav class="-mb-px flex space-x-2 overflow-x-auto [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-gray-200 dark:[&::-webkit-scrollbar-thumb]:bg-gray-600 dark:[&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar]:h-1.5">
                            {}
                        </nav>
                    </div>
                    
                    <div class="pt-6 dark:border-gray-800">
                        {}
                    </div>
                </div>
            "#, 
            "media_0",
            tab_component_id,
            tabs_html,
            content_html
            )
        };


        let kenya_counties = vec![
            "Nairobi", "Mombasa", "Kisumu", "Nakuru", "Eldoret", "Thika", "Malindi", "Kitale",
            "Garissa", "Kakamega", "Kisii", "Meru", "Nyeri", "Machakos", "Kiambu", "Kilifi",
            "Bungoma", "Busia", "Embu", "Homa Bay", "Isiolo", "Kajiado", "Kericho", "Kirinyaga",
            "Kitui", "Kwale", "Laikipia", "Lamu", "Mandera", "Marsabit", "Migori", "Murang'a",
            "Nyamira", "Nyandarua", "Narok", "Samburu", "Siaya", "Taita Taveta", "Tana River",
            "Trans Nzoia", "Turkana", "Uasin Gishu", "Vihiga", "Wajir", "West Pokot"
        ];
        
        let county_options: String = kenya_counties.iter().map(|county| {
            format!("<option value=\"{}\">{}</option>", county, county)
        }).collect();
        
        let location_javascript  = include_str!("../static/js/location_functions.js");
        let evidence_complete_js = include_str!("../static/js/evidence_complete.js");
        
        let mut context = HashMap::new();
        context.insert("email", email);
        context.insert("evidence_id", evidence_id.clone());
        context.insert("evidence_number", evidence.evidence_number.clone());
        context.insert("media_files_count", evidence.media_files.len().to_string());
        context.insert("media_preview_html", media_preview_html);
        context.insert("county_options", county_options);
        context.insert("today", chrono::Utc::now().format("%Y-%m-%d").to_string());
        context.insert("current_time", chrono::Utc::now().format("%H:%M").to_string());
        context.insert("location_javascript", location_javascript.to_string());
        context.insert("evidence_complete_js", evidence_complete_js.to_string());
        
        let html = render_template("evidence_complete", &context);
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

// Fix the evidence_view_page function - the issue is at the end of the function


pub async fn evidence_view_page(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    let email = session.get::<String>("user_email").unwrap_or(None);
    
    println!("🎥 EVIDENCE_VIEW_PAGE: Loading evidence ID: {}", evidence_id);
    
    let evidence_result = evidence_service.get_evidence(&evidence_id, true).await;
    let evidence = match evidence_result {
        Ok(Some(evidence)) => {
            println!("✅ Found evidence: {}", evidence.evidence_number);
            evidence
        },
        Ok(None) => {
            println!("❌ Evidence not found: {}", evidence_id);
            return HttpResponse::NotFound().body("Evidence not found");
        },
        Err(e) => {
            println!("❌ Error loading evidence: {}", e);
            return HttpResponse::InternalServerError().body("Failed to load evidence");
        },
    };
    
    let is_owner = user_id.as_ref().map_or(false, |uid| uid == &evidence.uploader_id);
    if evidence.status == EvidenceStatus::Draft && is_owner {
        println!("📋 Evidence is draft - redirecting to completion form for ID: {}", evidence_id);
        let redirect_html = format!(r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Complete Evidence - FLUG Evidence</title>
                <script src="https://cdn.tailwindcss.com"></script>
                <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">
                <style>
                    @keyframes fadeIn {{
                        from {{ opacity: 0; transform: translateY(10px); }}
                        to {{ opacity: 1; transform: translateY(0); }}
                    }}
                    .fade-in {{
                        animation: fadeIn 0.5s ease-out;
                    }}
                </style>
            </head>
            <body class="bg-gray-900 text-white min-h-screen flex items-center justify-center">
                <div class="max-w-md w-full p-8 fade-in">
                    <div class="text-center mb-8">
                        <div class="w-24 h-24 bg-yellow-600 rounded-full flex items-center justify-center mx-auto mb-6">
                            <i class="fas fa-edit text-4xl"></i>
                        </div>
                        <h1 class="text-2xl font-bold mb-2">Complete Your Evidence</h1>
                        <p class="text-gray-400">
                            This evidence is still in draft mode and needs to be completed before viewing.
                        </p>
                    </div>
                    
                    <div class="bg-gray-800 rounded-lg p-6 mb-6">
                        <div class="space-y-3">
                            <div class="flex items-center">
                                <i class="fas fa-hashtag text-yellow-400 mr-3"></i>
                                <div>
                                    <div class="text-sm text-gray-400">Case #</div>
                                    <div class="font-semibold">{}</div>
                                </div>
                            </div>
                            <div class="flex items-center">
                                <i class="fas fa-heading text-yellow-400 mr-3"></i>
                                <div>
                                    <div class="text-sm text-gray-400">Title</div>
                                    <div class="font-semibold">{}</div>
                                </div>
                            </div>
                            <div class="flex items-center">
                                <i class="fas fa-calendar text-yellow-400 mr-3"></i>
                                <div>
                                    <div class="text-sm text-gray-400">Created</div>
                                    <div class="font-semibold">{}</div>
                                </div>
                            </div>
                        </div>
                    </div>
                    
                    <div class="space-y-4">
                        <a href="/evidence/complete/{}" 
                           class="block w-full bg-gradient-to-r from-yellow-600 to-orange-600 hover:from-yellow-700 hover:to-orange-700 py-4 rounded-lg font-bold text-lg text-center transition-all duration-300 hover:scale-[1.02] active:scale-[0.98]">
                            <i class="fas fa-edit mr-2"></i>
                            Go to Completion Form
                        </a>
                        
                        <a href="/evidence/dashboard" 
                           class="block w-full bg-gray-700 hover:bg-gray-600 py-3 rounded-lg font-medium text-center transition-colors">
                            <i class="fas fa-arrow-left mr-2"></i>
                            Back to Dashboard
                        </a>
                    </div>
                    
                    <div class="mt-8 text-center text-sm text-gray-500">
                        <i class="fas fa-info-circle mr-1"></i>
                        You can view this evidence after completing all required details
                    </div>
                </div>
                
                <script>
                    document.addEventListener('DOMContentLoaded', function() {{
                        console.log('⏱️ Auto-redirecting in 5 seconds...');
                        setTimeout(function() {{
                            window.location.href = '/evidence/complete/{}';
                        }}, 5000);
                    }});
                </script>
            </body>
            </html>
        "#,
        html_escape(&evidence.evidence_number),
        html_escape(&evidence.title),
        evidence.created_at.format("%B %d, %Y %H:%M"),
        html_escape(&evidence_id),
        html_escape(&evidence_id)
        );
        
        return HttpResponse::Ok().body(redirect_html);
    }
    
    let target_photos_result = evidence_service.get_evidence_targets(&evidence_id).await;
    let target_photos = match target_photos_result {
        Ok(photos) => {
            println!("🎯 Found {} target photos for evidence {}", photos.len(), evidence_id);
            photos
        },
        Err(e) => {
            println!("⚠️ Error getting target photos: {}", e);
            Vec::new()
        }
    };
    
    let _user = if let Some(ref email) = email {
        auth_service.get_session_user(email).await.ok().flatten()
    } else {
        None
    };
  

    let media_files_html = if evidence.media_files.is_empty() {
    format!(r#"
        <div class="text-center py-8 bg-gray-800/50 rounded-lg">
            <i class="fas fa-file-video text-4xl text-gray-600 mb-4"></i>
            <p class="text-gray-400">No media files attached</p>
        </div>
    "#)
} else {
    // Generate a unique ID for this tab component
    let tab_component_id = format!("media_tabs_{}", evidence_id);
    
    let mut tabs_html = String::new();
    let mut content_html = String::new();
    
    // Create tabs for each media file
    for (i, media) in evidence.media_files.iter().enumerate() {
        let tab_id = format!("media_{}", i);
        let is_active = i == 0;
        
        // Shorten filename for tab display (keep first 20 chars)
        let display_name = if media.filename.len() > 20 {
            format!("{}...", &media.filename[..20])
        } else {
            media.filename.clone()
        };
        
        // Determine media type for icon
        let is_image = media.mime_type.starts_with("image/");
        let is_video = media.mime_type.starts_with("video/") 
            || media.filename.ends_with(".mp4") 
            || media.filename.ends_with(".mov") 
            || media.filename.ends_with(".avi") 
            || media.filename.ends_with(".webm");
        let is_audio = media.mime_type.starts_with("audio/");
        
        let icon = if is_video {
            "fas fa-video"
        } else if is_image {
            "fas fa-image"
        } else if is_audio {
            "fas fa-music"
        } else {
            "fas fa-file"
        };
        
        // Tab button
        tabs_html.push_str(&format!(r#"
            <button class="inline-flex items-center border-b-2 px-2.5 py-2 text-sm font-medium transition-colors duration-200 ease-in-out {}" 
                    x-bind:class="activeTab === '{}' ? 'text-brand-500 dark:text-brand-400 border-brand-500 dark:border-brand-400' : 'bg-transparent text-gray-500 border-transparent hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'" 
                    x-on:click="activeTab = '{}'">
                <i class="{} mr-2"></i>
                {}
            </button>
        "#, 
        if is_active { "text-brand-500 dark:text-brand-400 border-brand-500 dark:border-brand-400" } else { "bg-transparent text-gray-500 border-transparent hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200" },
        tab_id,
        tab_id,
        icon,
        display_name
        ));
        
        // Tab content
        let media_content = if is_video {
            format!(r#"
                <div class="aspect-video bg-black rounded-lg overflow-hidden mb-4">
                    <video controls 
                        preload="metadata"
                        playsinline
                        webkit-playsinline
                        class="w-full h-full object-contain"
                        style="background-color: #000;">
                        <source src="{}" type="{}">
                        Your browser does not support the video tag.
                    </video>
                </div>
                <div class="bg-gray-800/50 rounded-lg p-4">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <h4 class="font-medium text-white/90 mb-1">{}</h4>
                            <div class="text-sm text-gray-400">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Video</span>
                            </div>
                        </div>
                        <button onclick="downloadFile('{}', '{}')" 
                                class="text-blue-400 hover:text-blue-300 ml-2"
                                title="Download">
                            <i class="fas fa-download"></i>
                        </button>
                    </div>
                </div>
            "#,
            media.storj_url,
            media.mime_type,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.filename)
        } else if is_image {
            format!(r#"
                <div class="aspect-square bg-black rounded-lg overflow-hidden flex items-center justify-center p-4 cursor-pointer mb-4"
                     onclick="openMediaModal('{}', 'image')">
                    <img src="{}" alt="{}" 
                        class="max-h-full max-w-full object-contain rounded-lg">
                </div>
                <div class="bg-gray-800/50 rounded-lg p-4">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <h4 class="font-medium text-white/90 mb-1">{}</h4>
                            <div class="text-sm text-gray-400">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Image</span>
                            </div>
                        </div>
                        <div class="flex space-x-2">
                            <button onclick="openMediaModal('{}', 'image')" 
                                    class="text-green-400 hover:text-green-300"
                                    title="View Fullscreen">
                                <i class="fas fa-expand"></i>
                            </button>
                            <button onclick="downloadFile('{}', '{}')" 
                                    class="text-blue-400 hover:text-blue-300"
                                    title="Download">
                                <i class="fas fa-download"></i>
                            </button>
                        </div>
                    </div>
                </div>
            "#, 
            media.storj_url,
            media.storj_url, 
            media.filename,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.storj_url,
            media.filename)
        } else if is_audio {
            format!(r#"
                <div class="bg-gray-800/50 rounded-lg p-6 mb-4">
                    <div class="flex items-center space-x-6">
                        <div class="w-24 h-24 bg-purple-600/20 rounded-lg flex items-center justify-center flex-shrink-0 border border-purple-500/30">
                            <i class="fas fa-music text-3xl text-purple-400"></i>
                        </div>
                        <div class="flex-1 min-w-0">
                            <h4 class="font-medium text-white/90 mb-2">{}</h4>
                            <div class="text-sm text-gray-400 mb-4">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Audio</span>
                            </div>
                            <audio controls class="w-full">
                                <source src="{}" type="{}">
                                Your browser does not support the audio element.
                            </audio>
                        </div>
                    </div>
                </div>
                <div class="flex justify-center">
                    <button onclick="downloadFile('{}', '{}')" 
                            class="inline-flex items-center px-4 py-2 bg-brand-500 hover:bg-brand-600 text-white rounded-lg transition-colors">
                        <i class="fas fa-download mr-2"></i>
                        Download Audio File
                    </button>
                </div>
            "#, 
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.mime_type,
            media.storj_url,
            media.filename)
        } else {
            // Get file icon based on extension
            let file_icon = if media.filename.ends_with(".pdf") {
                "fas fa-file-pdf"
            } else if media.filename.ends_with(".doc") || media.filename.ends_with(".docx") {
                "fas fa-file-word"
            } else if media.filename.ends_with(".xls") || media.filename.ends_with(".xlsx") {
                "fas fa-file-excel"
            } else if media.filename.ends_with(".txt") {
                "fas fa-file-alt"
            } else if media.filename.ends_with(".zip") || media.filename.ends_with(".rar") {
                "fas fa-file-archive"
            } else {
                "fas fa-file"
            };
            
            let file_color = if media.filename.ends_with(".pdf") {
                "bg-red-600/20 border-red-500/30 text-red-400"
            } else if media.filename.ends_with(".doc") || media.filename.ends_with(".docx") {
                "bg-blue-600/20 border-blue-500/30 text-blue-400"
            } else if media.filename.ends_with(".xls") || media.filename.ends_with(".xlsx") {
                "bg-green-600/20 border-green-500/30 text-green-400"
            } else if media.filename.ends_with(".zip") || media.filename.ends_with(".rar") {
                "bg-yellow-600/20 border-yellow-500/30 text-yellow-400"
            } else {
                "bg-gray-600/20 border-gray-500/30 text-gray-400"
            };
            
            format!(r#"
                <div class="bg-gray-800/50 rounded-lg p-8 mb-4">
                    <div class="flex flex-col items-center justify-center text-center">
                        <div class="w-32 h-32 {} rounded-lg flex items-center justify-center flex-shrink-0 border mb-6">
                            <i class="{} text-4xl"></i>
                        </div>
                        <h4 class="font-medium text-white/90 mb-2 text-xl">{}</h4>
                        <div class="text-sm text-gray-400 mb-6">
                            <div class="mb-1">{} • {}</div>
                            <span class="px-2 py-1 bg-gray-700 rounded text-xs">Document</span>
                        </div>
                        <button onclick="downloadFile('{}', '{}')" 
                                class="inline-flex items-center px-6 py-3 bg-brand-500 hover:bg-brand-600 text-white rounded-lg transition-colors text-lg">
                            <i class="fas fa-download mr-2"></i>
                            Download File
                        </button>
                    </div>
                </div>
                <div class="text-center text-sm text-gray-400">
                    This document can be downloaded to your device for offline viewing.
                </div>
            "#, 
            file_color,
            file_icon,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.filename)
        };
        
        content_html.push_str(&format!(r#"
            <div x-show="activeTab === '{}'" {} >
                <h3 class="mb-4 text-xl font-medium text-gray-800 dark:text-white/90">
                    {}
                </h3>
                {}
            </div>
        "#,
        tab_id,
        if !is_active { "style=\"display: none;\"" } else { "" },
        media.filename,
        media_content
        ));
    }
    
    // If only one media file, show it directly without tabs
    if evidence.media_files.len() == 1 {
        let media = &evidence.media_files[0];
        let is_video = media.mime_type.starts_with("video/") 
            || media.filename.ends_with(".mp4") 
            || media.filename.ends_with(".mov") 
            || media.filename.ends_with(".avi") 
            || media.filename.ends_with(".webm");
        let is_image = media.mime_type.starts_with("image/");
        let is_audio = media.mime_type.starts_with("audio/");
        
        let media_content = if is_video {
            format!(r#"
                <div class="aspect-video bg-black rounded-lg overflow-hidden mb-4">
                    <video controls 
                        preload="metadata"
                        playsinline
                        webkit-playsinline
                        class="w-full h-full object-contain"
                        style="background-color: #000;">
                        <source src="{}" type="{}">
                        Your browser does not support the video tag.
                    </video>
                </div>
                <div class="bg-gray-800/50 rounded-lg p-4">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <h4 class="font-medium text-white/90 mb-1">{}</h4>
                            <div class="text-sm text-gray-400">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Video</span>
                            </div>
                        </div>
                        <button onclick="downloadFile('{}', '{}')" 
                                class="text-blue-400 hover:text-blue-300 ml-2"
                                title="Download">
                            <i class="fas fa-download"></i>
                        </button>
                    </div>
                </div>
            "#,
            media.storj_url,
            media.mime_type,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.filename)
        } else if is_image {
            format!(r#"
                <div class="aspect-square bg-black rounded-lg overflow-hidden flex items-center justify-center p-4 cursor-pointer mb-4"
                     onclick="openMediaModal('{}', 'image')">
                    <img src="{}" alt="{}" 
                        class="max-h-full max-w-full object-contain rounded-lg">
                </div>
                <div class="bg-gray-800/50 rounded-lg p-4">
                    <div class="flex items-start justify-between">
                        <div class="flex-1">
                            <h4 class="font-medium text-white/90 mb-1">{}</h4>
                            <div class="text-sm text-gray-400">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Image</span>
                            </div>
                        </div>
                        <div class="flex space-x-2">
                            <button onclick="openMediaModal('{}', 'image')" 
                                    class="text-green-400 hover:text-green-300"
                                    title="View Fullscreen">
                                <i class="fas fa-expand"></i>
                            </button>
                            <button onclick="downloadFile('{}', '{}')" 
                                    class="text-blue-400 hover:text-blue-300"
                                    title="Download">
                                <i class="fas fa-download"></i>
                            </button>
                        </div>
                    </div>
                </div>
            "#, 
            media.storj_url,
            media.storj_url, 
            media.filename,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.storj_url,
            media.filename)
        } else if is_audio {
            format!(r#"
                <div class="bg-gray-800/50 rounded-lg p-6 mb-4">
                    <div class="flex items-center space-x-6">
                        <div class="w-24 h-24 bg-purple-600/20 rounded-lg flex items-center justify-center flex-shrink-0 border border-purple-500/30">
                            <i class="fas fa-music text-3xl text-purple-400"></i>
                        </div>
                        <div class="flex-1 min-w-0">
                            <h4 class="font-medium text-white/90 mb-2">{}</h4>
                            <div class="text-sm text-gray-400 mb-4">
                                <span class="mr-3">{} • {}</span>
                                <span class="px-2 py-1 bg-gray-700 rounded text-xs">Audio</span>
                            </div>
                            <audio controls class="w-full">
                                <source src="{}" type="{}">
                                Your browser does not support the audio element.
                            </audio>
                        </div>
                    </div>
                </div>
                <div class="flex justify-center">
                    <button onclick="downloadFile('{}', '{}')" 
                            class="inline-flex items-center px-4 py-2 bg-brand-500 hover:bg-brand-600 text-white rounded-lg transition-colors">
                        <i class="fas fa-download mr-2"></i>
                        Download Audio File
                    </button>
                </div>
            "#, 
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.mime_type,
            media.storj_url,
            media.filename)
        } else {
            let file_icon = if media.filename.ends_with(".pdf") {
                "fas fa-file-pdf"
            } else if media.filename.ends_with(".doc") || media.filename.ends_with(".docx") {
                "fas fa-file-word"
            } else if media.filename.ends_with(".xls") || media.filename.ends_with(".xlsx") {
                "fas fa-file-excel"
            } else if media.filename.ends_with(".txt") {
                "fas fa-file-alt"
            } else if media.filename.ends_with(".zip") || media.filename.ends_with(".rar") {
                "fas fa-file-archive"
            } else {
                "fas fa-file"
            };
            
            let file_color = if media.filename.ends_with(".pdf") {
                "bg-red-600/20 border-red-500/30 text-red-400"
            } else if media.filename.ends_with(".doc") || media.filename.ends_with(".docx") {
                "bg-blue-600/20 border-blue-500/30 text-blue-400"
            } else if media.filename.ends_with(".xls") || media.filename.ends_with(".xlsx") {
                "bg-green-600/20 border-green-500/30 text-green-400"
            } else if media.filename.ends_with(".zip") || media.filename.ends_with(".rar") {
                "bg-yellow-600/20 border-yellow-500/30 text-yellow-400"
            } else {
                "bg-gray-600/20 border-gray-500/30 text-gray-400"
            };
            
            format!(r#"
                <div class="bg-gray-800/50 rounded-lg p-8 mb-4">
                    <div class="flex flex-col items-center justify-center text-center">
                        <div class="w-32 h-32 {} rounded-lg flex items-center justify-center flex-shrink-0 border mb-6">
                            <i class="{} text-4xl"></i>
                        </div>
                        <h4 class="font-medium text-white/90 mb-2 text-xl">{}</h4>
                        <div class="text-sm text-gray-400 mb-6">
                            <div class="mb-1">{} • {}</div>
                            <span class="px-2 py-1 bg-gray-700 rounded text-xs">Document</span>
                        </div>
                        <button onclick="downloadFile('{}', '{}')" 
                                class="inline-flex items-center px-6 py-3 bg-brand-500 hover:bg-brand-600 text-white rounded-lg transition-colors text-lg">
                            <i class="fas fa-download mr-2"></i>
                            Download File
                        </button>
                    </div>
                </div>
                <div class="text-center text-sm text-gray-400">
                    This document can be downloaded to your device for offline viewing.
                </div>
            "#, 
            file_color,
            file_icon,
            media.filename,
            format_bytes(media.file_size), 
            media.mime_type,
            media.storj_url,
            media.filename)
        };
        
        format!(r#"
            <div class="rounded-xl border border-gray-200 p-6 dark:border-gray-800">
                <h3 class="mb-4 text-xl font-medium text-gray-800 dark:text-white/90">
                    {}
                </h3>
                {}
            </div>
        "#, media.filename, media_content)
    } else {
        // Multiple media files - show with tabs
        format!(r#"
            <div class="rounded-xl border border-gray-200 p-6 dark:border-gray-800" 
                  x-data="{{ activeTab: 'media_0' }}" 
                  id="{}">
                <div class="border-b border-gray-200 dark:border-gray-800">
                    <nav class="-mb-px flex space-x-2 overflow-x-auto [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-gray-200 dark:[&::-webkit-scrollbar-thumb]:bg-gray-600 dark:[&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar]:h-1.5">
                        {}
                    </nav>
                </div>
                
                <div class="pt-4 dark:border-gray-800">
                    {}
                </div>
            </div>
        "#, 
        tab_component_id,
        tabs_html,
        content_html
        )
    }
};
    
let target_photos_html = if target_photos.is_empty() {
        format!(r#"
            <div class="col-span-full text-center py-12 rounded-xl border border-dashed border-gray-200 dark:border-gray-700">
                <i class="fas fa-bullseye text-4xl text-gray-300 dark:text-gray-600 mb-4"></i>
                <p class="text-sm text-gray-400">No target photos identified yet</p>
                {}
            </div>
        "#,
        if is_owner {
            format!(r#"
                <a href="/evidence/complete/{}"
                   class="inline-flex items-center gap-2 mt-4 bg-brand-500 hover:bg-brand-600 text-white px-5 py-2.5 rounded-full text-sm font-medium transition-colors">
                    <i class="fas fa-bullseye"></i>Identify Targets
                </a>
            "#, evidence_id)
        } else {
            String::new()
        })
    } else {
        let mut html = String::new();

        for (index, target) in target_photos.iter().enumerate() {
            let target_number = index + 1;

            let (category_label, category_bg) = match target.category {
                TargetCategory::Person   => ("Person",   "bg-blue-500"),
                TargetCategory::Vehicle  => ("Vehicle",  "bg-green-500"),
                TargetCategory::Object   => ("Object",   "bg-yellow-500"),
                TargetCategory::Location => ("Location", "bg-purple-500"),
                TargetCategory::Other    => ("Other",    "bg-gray-500"),
            };

            let full_desc = target.description.as_deref().unwrap_or(&target.filename);
            let short_desc = if full_desc.len() > 28 {
                format!("{}...", &full_desc[..28])
            } else {
                full_desc.to_string()
            };
            let full_desc_e  = html_escape(full_desc);
            let short_desc_e = html_escape(&short_desc);
            let storj_url_e  = html_escape(&target.storj_url);

            let target_id_e = html_escape(&target.id);
            html.push_str(&format!(r#"
                <div class="tgt-card rounded-xl border-2 border-gray-200 bg-white p-4 dark:border-gray-800 dark:bg-white/[0.03] transition-all duration-150"
                     data-id="{}" data-category="{}" data-desc="{}">
                    <div class="relative mb-4 overflow-hidden rounded-lg">
                        <img src="{}"
                             alt="{}"
                             class="tgt-img h-44 w-full rounded-lg object-cover cursor-pointer"
                             onclick="TargetSelector.openProfile(\'{}\', event)"
                             loading="lazy">
                        <!-- Selection checkbox -->
                        <button class="tgt-select-btn absolute top-2 left-2 z-10 h-6 w-6 rounded-full border-2 border-white/80 bg-black/40 flex items-center justify-center transition-all duration-150 hover:bg-blue-500 hover:border-blue-300"
                                onclick="TargetSelector.toggle(this, event)"
                                title="Select for bulk action">
                          <svg class="tgt-check hidden" width="11" height="11" viewBox="0 0 12 12" fill="none">
                            <path d="M2 6l3 3 5-5" stroke="white" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                          </svg>
                        </button>
                        <span class="absolute top-2 right-2 rounded-full {} px-2 py-0.5 text-xs font-semibold text-white">{}</span>
                        <span class="absolute bottom-2 right-2 rounded-full bg-black/60 px-2 py-0.5 text-xs font-semibold text-white">#{}</span>
                    </div>
                    <div>
                        <h4 class="mb-1 truncate text-base font-semibold text-gray-800 dark:text-white/90" title="{}">{}</h4>
                        <p class="text-sm text-gray-500 dark:text-gray-400">
                            Confidence: <span class="font-medium text-gray-700 dark:text-gray-300">{}%</span>
                        </p>
                        <button onclick="TargetSelector.openProfile(\'{}\', event)"
                                class="mt-4 inline-flex items-center gap-1 text-sm text-brand-500 hover:text-brand-600 dark:text-brand-400">
                            <svg class="fill-current" width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
                                <path fill-rule="evenodd" clip-rule="evenodd" d="M6.88029 3.10905C8.54002 1.44932 11.231 1.44933 12.8907 3.10906C14.5504 4.76878 14.5504 7.45973 12.8907 9.11946L12.0654 9.94479L11.0047 8.88413L11.83 8.0588C12.904 6.98486 12.904 5.24366 11.83 4.16972C10.7561 3.09577 9.01489 3.09577 7.94095 4.16971L7.11562 4.99504L6.05496 3.93438L6.88029 3.10905ZM8.88339 11.0055L9.94405 12.0661L9.11946 12.8907C7.45973 14.5504 4.76878 14.5504 3.10905 12.8907C1.44933 11.231 1.44933 8.54002 3.10905 6.88029L3.93364 6.0557L4.9943 7.11636L4.16971 7.94095C3.09577 9.01489 3.09577 10.7561 4.16971 11.83C5.24366 12.904 6.98486 12.904 8.0588 11.83L8.88339 11.0055ZM9.94422 7.11599C10.2371 6.8231 10.2371 6.34823 9.94422 6.05533C9.65132 5.76244 9.17645 5.76244 8.88356 6.05533L6.05513 8.88376C5.76224 9.17665 5.76224 9.65153 6.05513 9.94442C6.34802 10.2373 6.8229 10.2373 7.11579 9.94442L9.94422 7.11599Z" fill=""></path>
                            </svg>
                            View Profile
                        </button>
                    </div>
                </div>
            "#,
            target_id_e, category_label.to_lowercase(), full_desc_e,
            storj_url_e, full_desc_e, target_id_e,
            category_bg, category_label, target_number,
            full_desc_e, short_desc_e,
            target.confidence_score,
            target_id_e,
            ));
        }

        html
    };

    let target_photos_data: Vec<serde_json::Value> = target_photos.iter().enumerate().map(|(i, target)| {
        json!({
            "index": i,
            "id": target.id,
            "image_url": target.storj_url,
            "description": target.description.clone().unwrap_or_else(|| "No description".to_string()),
            "category": match target.category {
                TargetCategory::Person   => "person",
                TargetCategory::Vehicle  => "vehicle",
                TargetCategory::Object   => "object",
                TargetCategory::Location => "location",
                TargetCategory::Other    => "other",
            },
            "confidence_score": target.confidence_score as f32,
            "filename": target.filename,
            "file_size": target.file_size,
            "mime_type": target.mime_type,
            "created_at": target.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        })
    }).collect();

    let target_photos_json = serde_json::to_string(&target_photos_data)
        .unwrap_or_else(|_| "[]".to_string());

    let additional_sections_html = format!(r#"
        <div class="col-span-full">
            <div style="width:100%;" >
                <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">
                    Injuries Incurred If Any *
                </label>
                <textarea type="text" rows="2" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 w-full resize-none rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30"> {} </textarea>
            </div>
            <div style="width:100%;">
                <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">
                    Property / Asset Loss *
                </label>
                <textarea type="text" rows="2" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 w-full resize-none rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30"> {} </textarea>
            </div>
        </div>
        {}
        {}
    "#,
    evidence.injuries.as_deref().map(html_escape).as_deref().unwrap_or("None reported"),
    evidence.property_damage.as_deref().map(html_escape).as_deref().unwrap_or("None reported"),
    if let Some(suspect_desc) = &evidence.suspect_description {
        format!(r#"
            <div class="col-span-full">
                <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">
                    Detailed Description *
                </label>
                <textarea name="description" rows="2" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 w-full resize-none rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30"> {} </textarea>
            </div>
        "#, html_escape(suspect_desc))
    } else {
        String::new()
    },
    if let Some(vehicle) = &evidence.vehicle_details {
        format!(r#"
            <div class="border-b border-gray-200 px-6 py-4 dark:border-gray-800">
                <h3 class="text-lg font-medium text-gray-800 dark:text-white">
                    <i class="fas fa-car mr-2 text-blue-400"></i>Vehicle Details
                </h3>
                <div class="grid grid-cols-1 gap-5 sm:grid-cols-3">
                    <div>
                        <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">Registration | Plate Number</label>
                        <div class="font-medium">{}</div>
                    </div>
                    <div>
                        <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">Color Description</label>
                        <input type="text" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 h-11 w-full rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30" value="{}">
                    </div>
                    <div>
                        <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">Vehicle Type</label>
                        <input type="text" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 h-11 w-full rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30" value="{}">
                    </div>
                    <div>
                        <label class="mb-1.5 block text-sm font-medium text-gray-700 dark:text-gray-400">Sacco Name</label>
                        <input type="text" class="dark:bg-dark-900 shadow-theme-xs focus:border-brand-300 focus:ring-brand-500/10 dark:focus:border-brand-800 h-11 w-full rounded-lg border border-gray-300 bg-transparent px-4 py-2.5 text-sm text-gray-800 placeholder:text-gray-400 focus:ring-3 focus:outline-hidden dark:border-gray-700 dark:bg-gray-900 dark:text-white/90 dark:placeholder:text-white/30" value="{}">
                    </div>
                </div>
            </div>
        "#,
        html_escape(vehicle.registration.as_deref().unwrap_or("Unknown")),
        html_escape(vehicle.color.as_deref().unwrap_or("Unknown")),
        html_escape(&vehicle.vehicle_type.clone().map(|vt| format!("{:?}", vt)).unwrap_or_else(|| "Unknown".to_string())),
        html_escape(vehicle.sacco_name.as_deref().unwrap_or("Not specified"))
        )
    } else {
        String::new()
    }
    );

    let blockchain_html = if let (Some(_signature), Some(address)) = (&evidence.wallet_signature, &evidence.wallet_address) {
        format!(r#"
            <div class="bg-green-900/20 border border-green-700 rounded-lg p-6 mb-6">
                <h3 class="text-xl font-bold mb-4 flex items-center">
                    <i class="fas fa-lock text-green-500 mr-2"></i>Blockchain Verified
                </h3>
                <p class="text-sm text-gray-300 mb-4">This evidence has been cryptographically signed.</p>
                <div class="text-sm">
                    <div class="text-gray-400 mb-1">Signed by:</div>
                    <div class="font-mono text-xs bg-gray-900 p-2 rounded break-all">{}</div>
                </div>
            </div>
        "#, html_escape(address))
    } else {
        String::new()
    };

    let police_html = if evidence.reported_to_police {
        format!(r#"
            <div class="bg-green-900/20 border border-green-700 rounded-lg p-6 mb-6">
                <h3 class="text-xl font-bold mb-4 flex items-center">
                    <i class="fas fa-shield-alt text-green-500 mr-2"></i>Reported to Police
                </h3>
                <div class="space-y-3">
                    <div>
                        <div class="text-sm text-gray-400">Case ID:</div>
                        <div class="font-medium">{}</div>
                    </div>
                    <div>
                        <div class="text-sm text-gray-400">Station:</div>
                        <div class="font-medium">{}</div>
                    </div>
                </div>
            </div>
        "#,
        evidence.police_case_id.as_deref().map(html_escape).as_deref().unwrap_or("Not specified"),
        evidence.police_station.as_deref().map(html_escape).as_deref().unwrap_or("Not specified"))
    } else {
        r#"
            <div class="bg-red-900/20 border border-red-700 rounded-lg p-6 mb-6">
                <h3 class="text-xl font-bold mb-4 flex items-center">
                    <i class="fas fa-exclamation-triangle text-red-500 mr-2"></i>Not Reported
                </h3>
                <p class="text-sm text-gray-300 mb-4">This evidence has not been formally reported to police.</p>
            </div>
        "#.to_string()
    };

    let action_buttons = if is_owner {
        format!(r#"
            <div class="bg-gray-800 rounded-lg p-6">
                <h3 class="text-xl font-bold mb-4">Case Actions</h3>
                <div class="space-y-3">
                    {}
                    {}
                </div>
            </div>
        "#,
        if !evidence.reported_to_police {
            r#"<button @click="isProfileAddressModal = true" class="shadow-theme-xs flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 bg-white px-4 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 lg:inline-flex lg:w-auto dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
                <svg class="fill-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg"><path fill-rule="evenodd" clip-rule="evenodd" d="M15.0911 2.78206C14.2125 1.90338 12.7878 1.90338 11.9092 2.78206L4.57524 10.116C4.26682 10.4244 4.0547 10.8158 3.96468 11.2426L3.31231 14.3352C3.25997 14.5833 3.33653 14.841 3.51583 15.0203C3.69512 15.1996 3.95286 15.2761 4.20096 15.2238L7.29355 14.5714C7.72031 14.4814 8.11172 14.2693 8.42013 13.9609L15.7541 6.62695C16.6327 5.74827 16.6327 4.32365 15.7541 3.44497L15.0911 2.78206ZM12.9698 3.84272C13.2627 3.54982 13.7376 3.54982 14.0305 3.84272L14.6934 4.50563C14.9863 4.79852 14.9863 5.2734 14.6934 5.56629L14.044 6.21573L12.3204 4.49215L12.9698 3.84272ZM11.2597 5.55281L5.6359 11.1766C5.53309 11.2794 5.46238 11.4099 5.43238 11.5522L5.01758 13.5185L6.98394 13.1037C7.1262 13.0737 7.25666 13.003 7.35947 12.9002L12.9833 7.27639L11.2597 5.55281Z" fill=""></path></svg>
                Add Police Report
            </button>"#
        } else {
            r#"<button onclick="showPoliceCaseModal()" class="w-full bg-gray-700 hover:bg-gray-600 py-3 rounded-lg font-medium transition-colors">
                <i class="fas fa-edit mr-2"></i>Update Police Report
            </button>"#
        },
        if evidence.wallet_signature.is_none() {
            r#"<button @click="isProfileInfoModal = true" class="shadow-theme-xs flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 bg-white px-4 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 lg:inline-flex lg:w-auto dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
                <svg class="fill-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg"><path fill-rule="evenodd" clip-rule="evenodd" d="M15.0911 2.78206C14.2125 1.90338 12.7878 1.90338 11.9092 2.78206L4.57524 10.116C4.26682 10.4244 4.0547 10.8158 3.96468 11.2426L3.31231 14.3352C3.25997 14.5833 3.33653 14.841 3.51583 15.0203C3.69512 15.1996 3.95286 15.2761 4.20096 15.2238L7.29355 14.5714C7.72031 14.4814 8.11172 14.2693 8.42013 13.9609L15.7541 6.62695C16.6327 5.74827 16.6327 4.32365 15.7541 3.44497L15.0911 2.78206ZM12.9698 3.84272C13.2627 3.54982 13.7376 3.54982 14.0305 3.84272L14.6934 4.50563C14.9863 4.79852 14.9863 5.2734 14.6934 5.56629L14.044 6.21573L12.3204 4.49215L12.9698 3.84272ZM11.2597 5.55281L5.6359 11.1766C5.53309 11.2794 5.46238 11.4099 5.43238 11.5522L5.01758 13.5185L6.98394 13.1037C7.1262 13.0737 7.25666 13.003 7.35947 12.9002L12.9833 7.27639L11.2597 5.55281Z" fill=""></path></svg>
                Sign With Wallet
            </button>"#
        } else {
            r#"<button onclick="showWalletSignModal()" class="w-full bg-green-600 hover:bg-green-700 py-3 rounded-lg font-medium transition-colors">
                <i class="fas fa-check mr-2"></i>View Signature
            </button>"#
        }
        )
    } else {
        String::new()
    };

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let title_short = if evidence.title.len() > 30 {
        format!("{}...", &evidence.title[..30])
    } else {
        evidence.title.clone()
    };

    let view_js = include_str!("../static/js/evidence_view.js");

    let mut context = HashMap::new();
    context.insert("title",              html_escape(&evidence.title));
    context.insert("evidence_number",    html_escape(&evidence.evidence_number));
    context.insert("incident_date",      evidence.incident_time.format("%B %d, %Y").to_string());
    context.insert("incident_time",      evidence.incident_time.format("%H:%M").to_string());
    context.insert("additional_sections_html", additional_sections_html);
    context.insert("uploader_email",     html_escape(&evidence.uploader_email));
    context.insert("is_owner",           is_owner.to_string());
    context.insert("uploader_role",      if is_owner { "You (Owner)" } else { "Witness" }.to_string());
    context.insert("uploader_phone",     html_escape(evidence.uploader_phone.as_deref().unwrap_or("Not provided")));
    context.insert("created_at",         evidence.created_at.format("%Y-%m-%d %H:%M").to_string());
    context.insert("blockchain_html",    blockchain_html);
    context.insert("police_html",        police_html);
    context.insert("action_buttons",     action_buttons);
    context.insert("target_photos_json", target_photos_json);
    context.insert("evidence_id",        html_escape(&evidence_id));
    context.insert("title_short",        html_escape(&title_short));
    context.insert("today",              today);
    context.insert("view_js",            view_js.to_string());
    context.insert("Preview",            media_files_html);
    context.insert("county",             html_escape(&evidence.location.county));
    context.insert("constituency",       html_escape(evidence.location.constituency.as_deref().unwrap_or("Not specified")));
    context.insert("ward",               html_escape(evidence.location.ward.as_deref().unwrap_or("Not specified")));
    context.insert("latitude",           evidence.location.latitude.to_string());
    context.insert("longitude",          evidence.location.longitude.to_string());
    context.insert("location",           html_escape(evidence.location.landmark.as_deref().unwrap_or("Not specified")));
    context.insert("synopsis",           html_escape(&evidence.description));
    context.insert("target_photos",      target_photos_html);

    let html = render_template("evidence_view", &context);
    HttpResponse::Ok().body(html)
}

// Add missing evidence_my_page function
pub async fn evidence_my_page(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
    req: HttpRequest,
) -> HttpResponse {
    println!("📱 EVIDENCE_MY_PAGE: Loading my evidence page");
    
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("📱 EVIDENCE_MY_PAGE: User email: {}", email);
        
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("📱 EVIDENCE_MY_PAGE: Profile not complete, redirecting");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
            }
            _ => {
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        let user_id = session.get::<String>("user_id").unwrap_or(None);
        let query_str = req.query_string();
        println!("📱 EVIDENCE_MY_PAGE: Query string: {}", query_str);
        
        let mut query_params = HashMap::new();
        for param in query_str.split('&') {
            if param.is_empty() { continue; }
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 {
                query_params.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        
        println!("📱 EVIDENCE_MY_PAGE: Query params: {:?}", query_params);
        
        let filters = EvidenceSearchFilters {
            query: query_params.get("q").cloned(),
            incident_type: query_params.get("incident_type").cloned(),
            county: query_params.get("county").cloned(),
            emergency_level: query_params.get("emergency_level").cloned(),
            status: query_params.get("status").cloned(),
            reported_to_police: query_params.get("reported_to_police").map(|v| v == "true"),
            needs_attention: query_params.get("needs_attention").map(|v| v == "true"),
            signed_only: query_params.get("signed_only").map(|v| v == "true"),
            uploader_id: user_id.clone(),
            date_from: query_params.get("date_from").cloned(),
            date_to: query_params.get("date_to").cloned(),
            start_date: query_params.get("start_date").cloned(),
            end_date: query_params.get("end_date").cloned(),
            sort_by: Some(query_params.get("sort_by").cloned().unwrap_or_else(|| "newest".to_string())),
            page: 1,
            limit: 100,
        };
        
        println!("📱 EVIDENCE_MY_PAGE: Search filters: {:?}", filters);
        
        let search_result = match evidence_service.search_evidence_with_filters(&filters, user_id.as_deref().unwrap_or("")).await {
            Ok(result) => {
                println!("📱 EVIDENCE_MY_PAGE: Search successful, found {} items", result.total);
                result
            }
            Err(e) => {
                println!("📱 EVIDENCE_MY_PAGE: Error searching evidence: {}", e);
                EvidenceSearchResponse {
                    evidence: Vec::new(),
                    summaries: Vec::new(),
                    total: 0,
                    page: 1,
                    total_pages: 1,
                }
            }
        };
        
        let table_rows = if search_result.summaries.is_empty() {
    println!("📱 EVIDENCE_MY_PAGE: No evidence found, showing empty state");
    r#"
    <tr>
        <td colspan="7" class="px-6 py-12 text-center">
            <div class="inline-block p-8 bg-gray-900/50 rounded-lg">
                <i class="fas fa-file-alt text-4xl text-gray-700 mb-4"></i>
                <h3 class="text-xl font-bold mb-2">No Evidence Found</h3>
                <p class="text-gray-400 mb-4">You haven't uploaded any evidence yet.</p>
                <a href="/evidence/upload" class="inline-flex items-center bg-red-600 px-6 py-3 rounded-lg hover:bg-red-700">
                    <i class="fas fa-upload mr-2"></i>
                    Upload Your First Evidence
                </a>
            </div>
        </td>
    </tr>
    "#.to_string()
} else {
    println!("📱 EVIDENCE_MY_PAGE: Generating table with {} items", search_result.summaries.len());
    let mut rows_html = String::new();
    
    for evidence in search_result.summaries.iter().take(50) {
        // ── Per-row variables ───────────────────────────────────────────────

        let incident_type_str = format!("{:?}", evidence.incident_type);
        let title_short = if evidence.title.len() > 44 {
            format!("{}…", &evidence.title[..41])
        } else {
            evidence.title.clone()
        };
        let location_details = if !evidence.county.is_empty() {
            evidence.county.clone()
        } else {
            "Unknown".to_string()
        };
        let date_modified = evidence.incident_time.format("%d %b %Y").to_string();
        let police_str    = if evidence.reported_to_police { "true" } else { "false" };
        let media_bool    = if evidence.has_media { "true" } else { "false" };
        let attn_str      = if evidence.needs_attention { "true" } else { "false" };

        let simple_hash  = format!("{:x}", evidence.id.as_bytes().iter().fold(0u64, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u64)));
        let hash_display = &simple_hash[..8.min(simple_hash.len())];

        let (emg_str, emg_bg, emg_icon_color) = match evidence.emergency_level {
            EmergencyLevel::Red    => ("Critical", "bg-red-50 dark:bg-red-500/10",    "text-red-400"),
            EmergencyLevel::Orange => ("High",     "bg-orange-50 dark:bg-orange-500/10", "text-orange-400"),
            EmergencyLevel::Yellow => ("Medium",   "bg-yellow-50 dark:bg-yellow-500/10", "text-yellow-400"),
            EmergencyLevel::Blue   => ("Low",      "bg-blue-50 dark:bg-blue-500/10",   "text-blue-400"),
        };
        let status_str = match evidence.status {
            EvidenceStatus::Submitted    => "Submitted",
            EvidenceStatus::Reported     => "Reported",
            EvidenceStatus::UnderReview  => "Under Review",
            EvidenceStatus::Archived     => "Archived",
            EvidenceStatus::Rejected     => "Rejected",
            EvidenceStatus::Draft        => "Draft",
        };

        // ── Colorful status/emergency inline styles (matching ev-col-* CSS) ──
        let status_bg_style = match evidence.status {
            EvidenceStatus::Submitted   => "background:#eff6ff;color:#1d4ed8",
            EvidenceStatus::Reported    => "background:#f0fdf4;color:#166534",
            EvidenceStatus::UnderReview => "background:#fffbeb;color:#92400e",
            EvidenceStatus::Archived    => "background:#fdf4ff;color:#7e22ce",
            EvidenceStatus::Rejected    => "background:#fef2f2;color:#991b1b",
            EvidenceStatus::Draft       => "background:#f8fafc;color:#475569",
        };
        let emg_bg_style = match evidence.emergency_level {
            EmergencyLevel::Red    => "background:#fef2f2;color:#991b1b",
            EmergencyLevel::Orange => "background:#fff7ed;color:#9a3412",
            EmergencyLevel::Yellow => "background:#fefce8;color:#854d0e",
            EmergencyLevel::Blue   => "background:#eff6ff;color:#1e40af",
        };

        rows_html.push_str(&format!(r#"
        <div class="evidence-item grid grid-cols-12 items-center border-t border-gray-100 px-6 py-4 dark:border-gray-800 hover:bg-gray-50 dark:hover:bg-gray-800/40 transition-colors cursor-pointer"
             data-id="{id}"
             data-title="{title_esc}"
             data-status="{status_str}"
             data-emergency="{emg_str}"
             data-type="{inc_type}"
             data-date="{date}"
             data-location="{loc_esc}"
             data-evnum="{evnum}"
             data-hash="{hash_full}"
             data-police="{police}"
             data-size=""
             data-media="{media_bool}"
             data-attention="{attn}"
             data-policecase=""
             data-search="{search_blob}">

            <!-- Col 1 (3): Evidence ID + title -->
            <div class="col-span-3 flex items-center gap-3 min-w-0">
                <div class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-lg {emg_bg}">
                    <svg class="fill-current {emg_icon_color}" width="15" height="15" viewBox="0 0 20 20">
                        <path fill-rule="evenodd" d="M4 4a2 2 0 012-2h4.586A2 2 0 0112 2.586L15.414 6A2 2 0 0116 7.414V16a2 2 0 01-2 2H6a2 2 0 01-2-2V4zm2 6a1 1 0 011-1h6a1 1 0 110 2H7a1 1 0 01-1-1zm1 3a1 1 0 100 2h6a1 1 0 100-2H7z" clip-rule="evenodd"/>
                    </svg>
                </div>
                <div class="min-w-0">
                    <p class="text-sm font-semibold text-gray-800 dark:text-white/90 truncate">{title_short}</p>
                    <p class="ev-col-id">{evnum}</p>
                </div>
            </div>

            <!-- Col 2 (2): Type -->
            <div class="col-span-2">
                <span class="ev-col-type">{inc_type}</span>
            </div>

            <!-- Col 3 (2): Date -->
            <div class="col-span-2">
                <span class="ev-col-date">
                    <svg class="fill-current opacity-60" width="12" height="12" viewBox="0 0 16 16"><path fill-rule="evenodd" clip-rule="evenodd" d="M5.333 1.083a.75.75 0 01.75.75v.417h3.834v-.417a.75.75 0 011.5 0v.417H12.333c.967 0 1.75.784 1.75 1.75v8.667a1.75 1.75 0 01-1.75 1.75H3.667a1.75 1.75 0 01-1.75-1.75V4.583a1.75 1.75 0 011.75-1.75h.916v-.417a.75.75 0 01.75-.75zm-1.666 3.584H3.417V12.5c0 .138.112.25.25.25h8.666a.25.25 0 00.25-.25V4.667h-9z" fill=""/></svg>
                    {date}
                </span>
            </div>

            <!-- Col 4 (2): Location -->
            <div class="col-span-2 min-w-0">
                <span class="ev-col-loc truncate block">{location}</span>
            </div>

            <!-- Col 5 (1): Status badge -->
            <div class="col-span-1">
                <span class="status-badge" style="{status_bg_style}">{status_str}</span>
            </div>

            <!-- Col 6 (2): Emergency badge + actions -->
            <div class="col-span-2 flex items-center justify-end gap-2">
                <span class="emerg-badge" style="{emg_bg_style}">{emg_str}</span>
                <!-- Eye / View → opens sidebar drawer -->
                <button class="shadow-theme-xs inline-flex h-8 w-8 items-center justify-center rounded-lg border border-gray-300 text-gray-500 hover:bg-gray-50 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200"
                        data-ev-action="open-drawer"
                        data-ev-id="{id}"
                        title="View evidence details">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 21 20" fill="none">
                        <path d="M2.96487 10.7925C2.73306 10.2899 2.73306 9.71023 2.96487 9.20764C4.28084 6.35442 7.15966 4.375 10.4993 4.375C13.8389 4.375 16.7178 6.35442 18.0337 9.20765C18.2655 9.71024 18.2655 10.2899 18.0337 10.7925C16.7178 13.6458 13.8389 15.6252 10.4993 15.6252C7.15966 15.6252 4.28084 13.6458 2.96487 10.7925Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                        <path d="M13.5202 10C13.5202 11.6684 12.1677 13.0208 10.4993 13.0208C8.83099 13.0208 7.47852 11.6684 7.47852 10C7.47852 8.33164 8.83099 6.97917 10.4993 6.97917C12.1677 6.97917 13.5202 8.33164 13.5202 10Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                    </svg>
                </button>
                <!-- Delete → launches confirm-delete modal -->
                <button class="shadow-theme-xs inline-flex h-8 w-8 items-center justify-center rounded-lg border border-gray-300 text-gray-500 hover:bg-red-50 hover:border-red-300 hover:text-red-500 dark:border-gray-700 dark:text-gray-400 dark:hover:bg-red-500/10 dark:hover:border-red-800 dark:hover:text-red-400"
                        data-ev-action="delete"
                        data-ev-id="{id}"
                        title="Delete evidence">
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 21 20" fill="none">
                        <path d="M3.20833 5.41675H17.7917M8.625 2.08325H12.375M9.04167 14.1667V9.16675M11.9583 14.1667V9.16675M4.45833 5.41675L5.29167 15.8334C5.29167 16.7539 6.04119 17.5001 6.95833 17.5001H14.0417C14.9588 17.5001 15.7083 16.7539 15.7083 15.8334L16.5417 5.41675H4.45833Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                    </svg>
                </button>
            </div>

        </div>
        "#,
        id            = html_escape(&evidence.id),
        title_esc     = html_escape(&evidence.title),
        title_short   = html_escape(&title_short),
        status_str    = status_str,
        emg_str       = emg_str,
        inc_type      = html_escape(&incident_type_str),
        date          = html_escape(&date_modified),
        loc_esc       = html_escape(&location_details),
        location      = html_escape(&location_details),
        evnum         = html_escape(&evidence.evidence_number),
        hash_full     = html_escape(&hash_display),
        police        = police_str,
        media_bool    = media_bool,
        attn          = attn_str,
        emg_bg        = emg_bg,
        emg_icon_color= emg_icon_color,
        status_bg_style = status_bg_style,
        emg_bg_style  = emg_bg_style,
        search_blob   = html_escape(&format!("{} {} {} {} {} {}",
            evidence.title, evidence.evidence_number,
            incident_type_str, emg_str, status_str, location_details
        )),
        ));
    }
    
    rows_html
};
        // ── Card view rows (for the grid / card toggle) ──────────────────────
        let card_rows = if search_result.summaries.is_empty() {
            String::new()
        } else {
            search_result.summaries.iter()
                .take(100)
                .map(|e| render_evidence_card_my(e))
                .collect::<Vec<_>>()
                .join("\n")
        };

        // ── Chart breakdown counts ────────────────────────────────────────────
        let red_count    = search_result.summaries.iter().filter(|e| matches!(e.emergency_level, EmergencyLevel::Red)).count();
        let orange_count = search_result.summaries.iter().filter(|e| matches!(e.emergency_level, EmergencyLevel::Orange)).count();
        let yellow_count = search_result.summaries.iter().filter(|e| matches!(e.emergency_level, EmergencyLevel::Yellow)).count();
        let blue_count   = search_result.summaries.iter().filter(|e| matches!(e.emergency_level, EmergencyLevel::Blue)).count();

        let submitted_count  = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::Submitted)).count();
        let reported_st_count = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::Reported)).count();
        let review_count     = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::UnderReview)).count();
        let draft_count      = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::Draft)).count();
        let archived_count   = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::Archived)).count();
        let rejected_count   = search_result.summaries.iter().filter(|e| matches!(e.status, EvidenceStatus::Rejected)).count();

        let total_evidence = search_result.total;
        let urgent_count = search_result.summaries.iter().filter(|e| 
            matches!(e.emergency_level, EmergencyLevel::Red | EmergencyLevel::Orange)
        ).count();
        let reported_count = search_result.summaries.iter().filter(|e| e.reported_to_police).count();
        let needs_attention_count = search_result.summaries.iter().filter(|e| e.needs_attention).count();
        
        let mut context = HashMap::new();
        context.insert("email", email.clone());
        context.insert("total_evidence", total_evidence.to_string());
        context.insert("urgent_count", urgent_count.to_string());
        context.insert("reported_count", reported_count.to_string());
        context.insert("needs_attention_count", needs_attention_count.to_string());
        context.insert("table_rows", table_rows);
        context.insert("card_rows", card_rows);

        // Emergency chart data
        context.insert("chart_red",    red_count.to_string());
        context.insert("chart_orange", orange_count.to_string());
        context.insert("chart_yellow", yellow_count.to_string());
        context.insert("chart_blue",   blue_count.to_string());

        // Status chart data
        context.insert("chart_submitted", submitted_count.to_string());
        context.insert("chart_reported",  reported_st_count.to_string());
        context.insert("chart_review",    review_count.to_string());
        context.insert("chart_draft",     draft_count.to_string());
        context.insert("chart_archived",  archived_count.to_string());
        context.insert("chart_rejected",  rejected_count.to_string());
        
        // ── Trend chart: last 7 months upload counts ─────────────────────────
        let now = chrono::Utc::now();
        let mut trend_labels: Vec<String> = Vec::new();
        let mut trend_values: Vec<usize>  = Vec::new();
        for months_ago in (0i64..7).rev() {
            let target = now - chrono::Duration::days(months_ago * 30);
            let label  = target.format("%b").to_string();
            let count  = search_result.summaries.iter().filter(|e| {
                e.incident_time.format("%Y-%m").to_string() == target.format("%Y-%m").to_string()
            }).count();
            trend_labels.push(label);
            trend_values.push(count);
        }
        let trend_labels_json = serde_json::to_string(&trend_labels)
            .unwrap_or_else(|_| r#"["Jan","Feb","Mar","Apr","May","Jun","Jul"]"#.to_string());
        let trend_values_json = serde_json::to_string(&trend_values)
            .unwrap_or_else(|_| "[0,0,0,0,0,0,0]".to_string());
        context.insert("trend_labels_json", trend_labels_json);
        context.insert("trend_values_json", trend_values_json);

        // ── User initial for header avatar ────────────────────────────────────
        let user_initial = email.chars().next().unwrap_or('?').to_uppercase().to_string();
        context.insert("user_initial", user_initial);

        let html = render_template("evidence_my", &context);
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

// ==================== API ROUTES ====================



pub async fn api_complete_evidence(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
    mut payload: Multipart,
) -> HttpResponse {
    println!("📝 API_COMPLETE_EVIDENCE: Starting evidence completion");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => {
            println!("📝 API_COMPLETE_EVIDENCE: User email: {}", email);
            email
        }
        None => {
            println!("❌ API_COMPLETE_EVIDENCE: Not authenticated");
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    let user = match auth_service.get_session_user(&email).await {
        Ok(Some(user)) => {
            println!("📝 API_COMPLETE_EVIDENCE: User found: {}", user.id);
            user
        }
        Ok(None) => {
            println!("❌ API_COMPLETE_EVIDENCE: User not found");
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("User not found"));
        }
        Err(e) => {
            println!("❌ API_COMPLETE_EVIDENCE: Error getting user: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal error"));
        }
    };
    
    let user_id = user.id.clone();
    
    // Parse form data (IGNORE target photo fields - they're handled separately)
    let mut evidence_id = String::new();
    let mut form = EvidenceForm::default();
    
    println!("📝 API_COMPLETE_EVIDENCE: Parsing multipart data...");
    
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(field) => {
                println!("📝 API_COMPLETE_EVIDENCE: Processing field: {}", field.name());
                field
            }
            Err(e) => {
                println!("❌ API_COMPLETE_EVIDENCE: Error reading field: {}", e);
                continue;
            }
        };
        
        let field_name = field.name().to_string();
        
        // Skip target photo fields - they're handled by separate API
        if field_name.starts_with("target_") {
            println!("📝 API_COMPLETE_EVIDENCE: Skipping target field: {}", field_name);
            continue; // Skip these fields
        }
        
        // Handle text fields
        let mut value = String::new();
        while let Some(chunk) = field.next().await {
            match chunk {
                Ok(data) => {
                    if let Ok(text) = String::from_utf8(data.to_vec()) {
                        value.push_str(&text);
                    }
                }
                Err(e) => {
                    println!("❌ API_COMPLETE_EVIDENCE: Error reading chunk: {}", e);
                    break;
                }
            }
        }
        
        println!("📝 API_COMPLETE_EVIDENCE: Field {} = '{}'", field_name, value);
        
        match field_name.as_str() {
            "evidence_id" => evidence_id = value,
            "title" => form.title = value,
            "description" => form.description = value,
            "emergency_level" => form.emergency_level = value,
            "incident_type" => form.incident_type = value,
            "sub_type" => form.sub_type = Some(value),
            "incident_date" => form.incident_date = value,
            "incident_time" => form.incident_time = value,
            "county" => form.county = value,
            "constituency" => form.constituency = Some(value),
            "ward" => form.ward = Some(value),
            // Parse lat/lon as Option<f64>; empty string or "0" → None
            "latitude" => form.latitude = value.parse::<f64>().ok().filter(|v| *v != 0.0),
            "longitude" => form.longitude = value.parse::<f64>().ok().filter(|v| *v != 0.0),
            "landmark" => form.landmark = Some(value),
            "vehicle_registration" => form.vehicle_registration = Some(value),
            "vehicle_color" => form.vehicle_color = Some(value),
            "vehicle_type" => form.vehicle_type = Some(value),
            "sacco_name" => form.sacco_name = Some(value),
            "injuries" => form.injuries = Some(value),
            "property_damage" => form.property_damage = Some(value),
            "suspect_description" => form.suspect_description = Some(value),
            "reported_to_police" => form.reported_to_police = value == "true",
            "police_case_id" => form.police_case_id = Some(value),
            "police_station" => form.police_station = Some(value),
            "is_anonymous" => form.is_anonymous = value == "true",
            "sign_with_wallet" => form.sign_with_wallet = value == "true",
            // ── New RobustGeolocation fields ─────────────────────────────────
            "city" => form.city = if value.is_empty() { None } else { Some(value) },
            "region" => form.region = if value.is_empty() { None } else { Some(value) },
            "country" => form.country = if value.is_empty() { None } else { Some(value) },
            "location_accuracy" => form.location_accuracy = if value.is_empty() { None } else { Some(value) },
            "location_source" => form.location_source = if value.is_empty() { None } else { Some(value) },
            "proxy_detected" => form.proxy_detected = Some(value == "true"),
            "target_count" => {
                // Just log this but don't store it
                println!("📝 API_COMPLETE_EVIDENCE: Target count: {}", value);
            }
            _ => {
                println!("📝 API_COMPLETE_EVIDENCE: Unknown field (ignored): {}", field_name);
            }
        }
    }
    
    println!("📝 API_COMPLETE_EVIDENCE: Evidence ID: {}", evidence_id);
    
    if evidence_id.is_empty() {
        println!("❌ API_COMPLETE_EVIDENCE: Missing evidence ID");
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Missing evidence ID"));
    }
    
    // Validate required fields
    // lat/lon are now Option<f64>; None means the JS geolocation returned empty/failed
    if form.title.is_empty() || form.description.is_empty() ||
       form.emergency_level.is_empty() || form.incident_type.is_empty() ||
       form.county.is_empty() || form.latitude.is_none() || form.longitude.is_none() {
        println!("❌ API_COMPLETE_EVIDENCE: Missing required fields");
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Missing required fields. Location coordinates are required — please allow location access or enter them manually."));
    }
    
    // Complete the draft evidence
    let sign_with_wallet = form.sign_with_wallet;

    match evidence_service.complete_draft_evidence(
        &evidence_id,
        form,
        &user_id,
        &auth_service,
        sign_with_wallet,
    ).await {
        Ok(evidence) => {
            println!("✅ API_COMPLETE_EVIDENCE: Evidence completed successfully!");
            println!("   Evidence ID: {}", evidence.id);
            println!("   New Status: {:?}", evidence.status);
            
            let response_data = serde_json::json!({
                "id": evidence.id,
                "evidence_number": evidence.evidence_number,
                "title": evidence.title,
                "status": format!("{:?}", evidence.status),
                "incident_type": format!("{:?}", evidence.incident_type),
                "emergency_level": format!("{:?}", evidence.emergency_level),
                "location": {
                    "county": evidence.location.county,
                    "latitude": evidence.location.latitude,
                    "longitude": evidence.location.longitude
                },
                "created_at": evidence.created_at.to_rfc3339()
            });
            
            HttpResponse::Ok().json(ApiResponse::success(response_data))
        }
        Err(e) => {
            println!("❌ API_COMPLETE_EVIDENCE: Failed to complete evidence: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e)))
        }
    }
} 



pub async fn api_evidence_report_to_police(
    session: Session,
    path: web::Path<String>,
    body: web::Json<PoliceReportJson>,   // Accept JSON — frontend sends application/json
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if user_id.is_none() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
    }
    
    println!("🚨 API_EVIDENCE_REPORT_TO_POLICE: Reporting evidence ID: {}", evidence_id);
    
    let payload = body.into_inner();
    println!("🚨 API_EVIDENCE_REPORT_TO_POLICE: Police report number: {}", payload.report_number);
    
    // Map the JSON fields onto the PoliceReportForm the service expects
    let police_report = PoliceReportForm {
        evidence_id:      evidence_id.clone(),
        police_station:   payload.police_station,
        report_number:    payload.report_number,
        officer_name:     Some(payload.officer_name.unwrap_or_default()),
        contact_number:   Some(String::new()),
        additional_notes: Some(payload.additional_notes.unwrap_or_default()),
    };
    
    match evidence_service.report_evidence_to_police(
        &evidence_id,
        &user_id.unwrap(),
        police_report,
    ).await {
        Ok(evidence) => {
            println!("✅ API_EVIDENCE_REPORT_TO_POLICE: Successfully reported evidence: {}", evidence.evidence_number);
            HttpResponse::Ok().json(ApiResponse::success(
                json!({
                    "evidence_id": evidence.id,
                    "evidence_number": evidence.evidence_number,
                    "police_case_id": evidence.police_case_id,
                    "reported": evidence.reported_to_police,
                    "redirect": format!("/evidence/view/{}", evidence.id)
                })
            ))
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_REPORT_TO_POLICE: Error: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to report evidence: {}", e)))
        }
    }
}

pub async fn api_evidence_submit(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if user_id.is_none() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
    }
    
    println!("📤 API_EVIDENCE_SUBMIT: Submitting evidence ID: {}", evidence_id);
    
    match evidence_service.submit_evidence(&evidence_id, &user_id.unwrap()).await {
        Ok(evidence) => {
            println!("✅ API_EVIDENCE_SUBMIT: Successfully submitted evidence: {}", evidence.evidence_number);
            HttpResponse::Ok().json(ApiResponse::success(
                json!({
                    "evidence_id": evidence.id,
                    "evidence_number": evidence.evidence_number,
                    "status": format!("{:?}", evidence.status),
                    "redirect": format!("/evidence/view/{}", evidence.id)
                })
            ))
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_SUBMIT: Error: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to submit evidence: {}", e)))
        }
    }
}



pub async fn api_upload_media(
    session: Session,
    mut payload: Multipart,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    let email = session.get::<String>("user_email").unwrap_or(None);
    
    if user_id.is_none() || email.is_none() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
    }
    
    let user_id_clone = user_id.clone();
    let email_clone = email.clone();
    
    let mut upload_result = None;
    let mut field_name = String::new();
    let mut evidence_id = String::new();
    
    // Use StreamExt trait
    while let Some(item_result) = payload.next().await {
        match item_result {
            Ok(mut field) => {
                let content_disposition = field.content_disposition();
                field_name = content_disposition.get_name().unwrap_or("unknown").to_string();
                
                println!("📤 API_UPLOAD_MEDIA: Processing field: {}", field_name);
                
                if field_name == "evidence_id" {
                    let mut evidence_id_bytes = Vec::new();
                    
                    // Collect chunks properly
                    while let Some(chunk_result) = field.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                evidence_id_bytes.extend_from_slice(&chunk);
                            }
                            Err(e) => {
                                println!("❌ API_UPLOAD_MEDIA: Error reading evidence_id: {}", e);
                                return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Failed to read evidence ID"));
                            }
                        }
                    }
                    
                    evidence_id = String::from_utf8_lossy(&evidence_id_bytes).to_string();
                    println!("📤 API_UPLOAD_MEDIA: Evidence ID: {}", evidence_id);
                    
                    // Get user info
                    if let Some(ref email_val) = email_clone {
                        match auth_service.get_user_by_email(email_val).await {
                            Ok(Some(user)) => {
                                let session_user = SessionUser {
                                    id: user.id.clone(),
                                    email: user.email.clone(),
                                    has_password: user.password_hash.is_some(),
                                    has_wallet: user.wallet_address.is_some(),
                                    wallet_address: user.wallet_address.clone(),
                                    wallet_type: user.wallet_type.clone(),
                                    wallet_chain: user.wallet_chain.clone(),
                                    is_verified: user.is_verified,
                                    wallet_connections: Vec::new(),
                                    account_type: user.account_type.clone(),
                                    business_name: user.business_name.clone(),
                                    geo_latitude: user.geo_latitude,
                                    geo_longitude: user.geo_longitude,
                                    is_profile_complete: user.is_profile_complete,
                                    phone_number: user.phone_number.clone(),
                                    county: user.county.clone(),
                                    id_number: user.id_number.clone(),
                                };
                                
                                // Create evidence form
                                let form = EvidenceForm {
                                    title: "Evidence Upload".to_string(),
                                    description: "Evidence uploaded via media upload".to_string(),
                                    emergency_level: "blue".to_string(),
                                    incident_type: "Other".to_string(),
                                    sub_type: None,
                                    incident_date: chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                    incident_time: chrono::Utc::now().format("%H:%M").to_string(),
                                    county: "Nairobi".to_string(),
                                    constituency: None,
                                    ward: None,
                                    latitude: None,
                                    longitude: None,
                                    landmark: None,
                                    vehicle_registration: None,
                                    vehicle_color: None,
                                    vehicle_type: None,
                                    sacco_name: None,
                                    injuries: None,
                                    property_damage: None,
                                    suspect_description: None,
                                    reported_to_police: false,
                                    police_case_id: None,
                                    police_station: None,
                                    is_anonymous: false,
                                    sign_with_wallet: false,
                                    // New geolocation fields — unknown at this point
                                    city: None,
                                    region: None,
                                    country: None,
                                    location_accuracy: None,
                                    location_source: None,
                                    proxy_detected: None,
                                };
                                
                                upload_result = Some(evidence_service.create_evidence_with_media(
                                    form,
                                    &session_user,
                                    &auth_service
                                ).await);
                            }
                            Ok(None) => {
                                return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("User not found"));
                            }
                            Err(e) => {
                                println!("❌ API_UPLOAD_MEDIA: Error getting user: {}", e);
                                return HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error"));
                            }
                        }
                    }
                } else if field_name.starts_with("media_file_") {
                    let filename = content_disposition.get_filename().unwrap_or("unknown").to_string();
                    println!("📤 API_UPLOAD_MEDIA: Uploading file: {}", filename);
                    
                    let mut file_bytes = Vec::new();
                    
                    // Collect chunks properly
                    while let Some(chunk_result) = field.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                file_bytes.extend_from_slice(&chunk);
                            }
                            Err(e) => {
                                println!("❌ API_UPLOAD_MEDIA: Error reading file chunk: {}", e);
                                return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Failed to read file"));
                            }
                        }
                    }
                    
                    if let Some(Ok(ref evidence)) = upload_result {
                        // Get MIME type from filename
                        let mime_type = if filename.ends_with(".jpg") || filename.ends_with(".jpeg") {
                            "image/jpeg"
                        } else if filename.ends_with(".png") {
                            "image/png"
                        } else if filename.ends_with(".mp4") {
                            "video/mp4"
                        } else if filename.ends_with(".mov") {
                            "video/quicktime"
                        } else if filename.ends_with(".avi") {
                            "video/x-msvideo"
                        } else if filename.ends_with(".webm") {
                            "video/webm"
                        } else if filename.ends_with(".mp3") {
                            "audio/mpeg"
                        } else if filename.ends_with(".wav") {
                            "audio/wav"
                        } else {
                            "application/octet-stream"
                        };
                        
                        if let Some(ref user_id_val) = user_id_clone {
                            match evidence_service.upload_evidence_media(
                                &evidence.id,
                                file_bytes,
                                mime_type,
                                user_id_val
                            ).await {
                                Ok(_) => {
                                    println!("✅ API_UPLOAD_MEDIA: Successfully uploaded media: {}", filename);
                                }
                                Err(e) => {
                                    println!("❌ API_UPLOAD_MEDIA: Error uploading media: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("❌ API_UPLOAD_MEDIA: Multipart error: {}", e);
                return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Failed to parse multipart data"));
            }
        }
    }
    
    match upload_result {
        Some(Ok(evidence)) => {
            println!("✅ API_UPLOAD_MEDIA: Successfully created evidence: {}", evidence.evidence_number);
            HttpResponse::Ok().json(ApiResponse::success(
                json!({
                    "evidence_id": evidence.id,
                    "evidence_number": evidence.evidence_number,
                    "redirect": format!("/evidence/complete/{}", evidence.id)
                })
            ))
        }
        Some(Err(e)) => {
            println!("❌ API_UPLOAD_MEDIA: Error creating evidence: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to create evidence: {}", e)))
        }
        None => {
            HttpResponse::BadRequest().json(ApiResponse::<()>::error("No evidence ID provided"))
        }
    }
}


pub async fn api_evidence_delete(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if user_id.is_none() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
    }
    
    println!("🗑️ API_EVIDENCE_DELETE: Deleting evidence ID: {}", evidence_id);
    
    match evidence_service.delete_evidence(&evidence_id, &user_id.unwrap()).await {
        Ok(_) => {
            println!("✅ API_EVIDENCE_DELETE: Successfully deleted evidence: {}", evidence_id);
            HttpResponse::Ok().json(ApiResponse::success(
                json!({
                    "evidence_id": evidence_id,
                    "redirect": "/evidence/dashboard"
                })
            ))
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_DELETE: Error: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to delete evidence: {}", e)))
        }
    }
}

pub async fn api_get_evidence_targets(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if user_id.is_none() {
        return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
    }
    
    println!("🎯 API_GET_EVIDENCE_TARGETS: Getting targets for evidence ID: {}", evidence_id);
    
    match evidence_service.get_evidence_targets(&evidence_id).await {
        Ok(targets) => {
            println!("✅ API_GET_EVIDENCE_TARGETS: Found {} targets", targets.len());
            let target_data: Vec<serde_json::Value> = targets.iter().enumerate().map(|(i, target)| {
                json!({
                    "index": i,
                    "image_url": target.storj_url,
                    "description": target.description.clone().unwrap_or_else(|| "No description".to_string()),
                    "category": match target.category {
                        TargetCategory::Person => "person",
                        TargetCategory::Vehicle => "vehicle",
                        TargetCategory::Object => "object",
                        TargetCategory::Location => "location",
                        TargetCategory::Other => "other",
                    },
                    "confidence_score": target.confidence_score as f32,
                    "filename": target.filename,
                    "file_size": target.file_size,
                    "mime_type": target.mime_type,
                    "created_at": target.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                })
            }).collect();
            
            HttpResponse::Ok().json(ApiResponse::success(target_data))
        }
        Err(e) => {
            println!("❌ API_GET_EVIDENCE_TARGETS: Error: {}", e);
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(&format!("Failed to get targets: {}", e)))
        }
    }
}

pub async fn api_evidence_sign(
    session: Session,
    path: web::Path<String>,
    // Note: no form body — frontend sends a bare POST with no body.
    // The wallet address is read from the session server-side.
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    let wallet_address = session.get::<String>("wallet_address").unwrap_or(None);
    let _wallet_chain = session.get::<String>("wallet_chain").unwrap_or(None);
    
    // Check authentication
    if user_id.is_none() {
        return HttpResponse::Unauthorized().json(SimpleApiResponse {
            success: false,
            message: "Not authenticated".to_string(),
        });
    }
    
    if wallet_address.is_none() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("No wallet connected"));
    }
    
    println!("🔏 API_EVIDENCE_SIGN: Signing evidence ID: {}", evidence_id);
    
    let user_id = user_id.unwrap();
    
    // 1. Get the evidence from database
    let evidence = match evidence_service.get_evidence(&evidence_id, false).await {
        Ok(Some(evidence)) => evidence,
        Ok(None) => {
            println!("❌ Evidence not found: {}", evidence_id);
            return HttpResponse::NotFound().json(ApiResponse::<()>::error("Evidence not found"));
        }
        Err(e) => {
            println!("❌ Error getting evidence: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Failed to load evidence"));
        }
    };
    
    // 2. Verify ownership
    if evidence.uploader_id != user_id {
        println!("❌ User {} doesn't own evidence {}", user_id, evidence_id);
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("You don't own this evidence"));
    }
    
    // 3. Check if already signed
    if evidence.wallet_signature.is_some() {
        println!("⚠️ Evidence already signed: {}", evidence_id);
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Evidence already signed"));
    }
    
    // 4. Get user's wallet info
    let user_email = session.get::<String>("user_email").unwrap_or(None);
    let wallet_info = if let Some(email) = user_email {
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if user.wallet_address.is_none() || user.wallet_chain.is_none() {
                    println!("⚠️ User has no wallet info");
                    None
                } else {
                    Some((user.wallet_address.unwrap(), user.wallet_chain.unwrap()))
                }
            }
            _ => None,
        }
    } else {
        None
    };
    
    // 5. Try to sign the evidence
    match evidence_service.sign_evidence(
        &evidence_id,
        &user_id,
        &auth_service,
    ).await {
        Ok(signature) => {
            println!("✅ API_EVIDENCE_SIGN: Successfully signed evidence: {}", evidence.evidence_number);
            
            // 6. Get updated evidence to verify signature was saved
            let updated_evidence = match evidence_service.get_evidence(&evidence_id, false).await {
                Ok(Some(evidence)) => evidence,
                Ok(None) => {
                    println!("⚠️ Could not retrieve updated evidence");
                    evidence // Use original as fallback
                }
                Err(e) => {
                    println!("⚠️ Error retrieving updated evidence: {}", e);
                    evidence // Use original as fallback
                }
            };
            
            // 7. Get signatures for this evidence
            let signatures = evidence_service.get_evidence_signatures(&evidence_id).await
                .unwrap_or_else(|e| {
                    println!("⚠️ Error getting signatures: {}", e);
                    Vec::new()
                });
            
            // 8. Build success response
            let response_data = json!({
                "success": true,
                "message": format!("Evidence signed successfully: {}", updated_evidence.evidence_number),
                "evidence": {
                    "id": updated_evidence.id,
                    "evidence_number": updated_evidence.evidence_number,
                    "title": updated_evidence.title,
                    "emergency_level": match updated_evidence.emergency_level {
                        EmergencyLevel::Red => "red",
                        EmergencyLevel::Orange => "orange",
                        EmergencyLevel::Yellow => "yellow",
                        EmergencyLevel::Blue => "blue",
                    },
                    "status": match updated_evidence.status {
                        EvidenceStatus::Draft => "draft",
                        EvidenceStatus::Submitted => "submitted",
                        EvidenceStatus::Reported => "reported",
                        EvidenceStatus::UnderReview => "under_review",
                        EvidenceStatus::Archived => "archived",
                        EvidenceStatus::Rejected => "rejected",
                    },
                    "reported_to_police": updated_evidence.reported_to_police,
                    "wallet_signed": updated_evidence.wallet_signature.is_some(),
                    "wallet_address": updated_evidence.wallet_address,
                    "signature_count": signatures.len(),
                },
                "signature": {
                    "evidence_id": signature.evidence_id,
                    "wallet_address": signature.wallet_address,
                    "signature_short": if signature.signature.len() > 16 {
                        format!("{}...", &signature.signature[0..16])
                    } else {
                        signature.signature.clone()
                    },
                    "signed_hash_short": if signature.signed_hash.len() > 16 {
                        format!("{}...", &signature.signed_hash[0..16])
                    } else {
                        signature.signed_hash.clone()
                    },
                    "timestamp": signature.timestamp.timestamp(),
                    "chain": signature.chain,
                    "transaction_id": signature.transaction_id,
                    "formatted_time": signature.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                },
                "redirect": format!("/evidence/view/{}", evidence_id),
                "timestamp": chrono::Utc::now().timestamp(),
            });
            
            HttpResponse::Ok()
                .content_type("application/json")
                .json(ApiResponse::success(response_data))
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_SIGN: Error signing evidence: {}", e);
            
            // Provide more specific error messages
            let error_message = if e.to_string().contains("already signed") {
                "Evidence is already signed".to_string()
            } else if e.to_string().contains("No wallet connected") {
                "No wallet connected. Please connect a wallet first".to_string()
            } else if e.to_string().contains("not found") {
                "Evidence not found".to_string()
            } else if e.to_string().contains("don't own") {
                "You don't own this evidence".to_string()
            } else {
                format!("Failed to sign evidence: {}", e)
            };
            
            HttpResponse::BadRequest()
                .content_type("application/json")
                .json(ApiResponse::<()>::error(&error_message))
        }
    }
}

/// Upload evidence API - Simplified version matching old flow
pub async fn api_evidence_upload(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
    mut payload: Multipart,
) -> HttpResponse {
    println!("📤 API_EVIDENCE_UPLOAD: Starting evidence upload");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => {
            println!("📤 API_EVIDENCE_UPLOAD: User email: {}", email);
            email
        }
        None => {
            println!("❌ API_EVIDENCE_UPLOAD: Not authenticated");
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    let user = match auth_service.get_session_user(&email).await {
        Ok(Some(user)) => {
            println!("📤 API_EVIDENCE_UPLOAD: User found: {}", user.id);
            user
        }
        Ok(None) => {
            println!("❌ API_EVIDENCE_UPLOAD: User not found");
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("User not found"));
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_UPLOAD: Error getting user: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal error"));
        }
    };
    
    // Parse form data - for DRAFT uploads, we expect minimal data
    let mut form = EvidenceForm::default();
    let mut files = Vec::new();
    
    println!("📤 API_EVIDENCE_UPLOAD: Parsing multipart data...");
    
    // Track if we have basic form data
    let mut has_basic_data = false;
    
    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(field) => {
                println!("📤 API_EVIDENCE_UPLOAD: Processing field: {}", field.name());
                field
            }
            Err(e) => {
                println!("❌ API_EVIDENCE_UPLOAD: Error reading field: {}", e);
                continue;
            }
        };
        
        let field_name = field.name().to_string();
        
        // Handle text fields
        if field_name != "files" {
            let mut value = String::new();
            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(data) => {
                        if let Ok(text) = String::from_utf8(data.to_vec()) {
                            value.push_str(&text);
                        }
                    }
                    Err(e) => {
                        println!("❌ API_EVIDENCE_UPLOAD: Error reading chunk: {}", e);
                        break;
                    }
                }
            }
            
            println!("📤 API_EVIDENCE_UPLOAD: Field {} = '{}'", field_name, value);
            
            match field_name.as_str() {
                "title" => {
                    form.title = value;
                    has_basic_data = true;
                }
                "description" => {
                    form.description = value;
                    has_basic_data = true;
                }
                "emergency_level" => {
                    form.emergency_level = value;
                    has_basic_data = true;
                }
                "incident_type" => {
                    form.incident_type = value;
                    has_basic_data = true;
                }
                "incident_date" => form.incident_date = value,
                "incident_time" => form.incident_time = value,
                "county" => form.county = value,
                // Parse lat/lon as Option<f64>; empty string or "0" → None
                "latitude" => form.latitude = value.parse::<f64>().ok().filter(|v| *v != 0.0),
                "longitude" => form.longitude = value.parse::<f64>().ok().filter(|v| *v != 0.0),
                // ── New RobustGeolocation fields ─────────────────────────────
                "city" => form.city = if value.is_empty() { None } else { Some(value) },
                "region" => form.region = if value.is_empty() { None } else { Some(value) },
                "country" => form.country = if value.is_empty() { None } else { Some(value) },
                "location_accuracy" => form.location_accuracy = if value.is_empty() { None } else { Some(value) },
                "location_source" => form.location_source = if value.is_empty() { None } else { Some(value) },
                "proxy_detected" => form.proxy_detected = Some(value == "true"),
                // Other fields can be ignored for draft
                _ => {
                    // Ignore other fields for draft
                }
            }
        } else {
            // Handle file fields
            let content_disposition = field.content_disposition();
            let filename = content_disposition.get_filename().map(|s| s.to_string()).unwrap_or_default();
            let mime_type = field.content_type().map(|ct| ct.to_string()).unwrap_or_else(|| "application/octet-stream".to_string());
            
            println!("📤 API_EVIDENCE_UPLOAD: Receiving file: {} ({})", filename, mime_type);
            
            let mut file_bytes = Vec::new();
            let mut chunk_count = 0;
            
            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(data) => {
                        chunk_count += 1;
                        file_bytes.extend_from_slice(&data);
                    }
                    Err(e) => {
                        println!("❌ API_EVIDENCE_UPLOAD: Error reading file chunk: {}", e);
                        break;
                    }
                }
            }
            
            if !file_bytes.is_empty() && !filename.is_empty() {
                println!("📤 API_EVIDENCE_UPLOAD: File received: {} ({} bytes, {} chunks)", 
                    filename, file_bytes.len(), chunk_count);
                files.push((file_bytes, filename, mime_type));
            } else {
                println!("⚠️ API_EVIDENCE_UPLOAD: Empty file or filename: {}", filename);
            }
        }
    }
    
    println!("📤 API_EVIDENCE_UPLOAD: Parsing complete:");
    println!("   Has basic data: {}", has_basic_data);
    println!("   Files received: {}", files.len());
    
    if files.is_empty() {
        println!("❌ API_EVIDENCE_UPLOAD: No evidence files provided");
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("At least one evidence file is required"));
    }
    
    // Always create as DRAFT - this matches your old flow
    println!("📤 API_EVIDENCE_UPLOAD: Creating DRAFT evidence");
    
    // Fill in any missing required fields with defaults
    if form.title.is_empty() {
        form.title = "DRAFT - Captured Evidence".to_string();
    }
    if form.description.is_empty() {
        form.description = "Evidence captured via live recording. Details pending completion.".to_string();
    }
    if form.emergency_level.is_empty() {
        form.emergency_level = "blue".to_string();
    }
    if form.incident_type.is_empty() {
        form.incident_type = "Other".to_string();
    }
    if form.county.is_empty() {
        form.county = "Nairobi".to_string();
    }
    if form.incident_date.is_empty() {
        form.incident_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    }
    if form.incident_time.is_empty() {
        form.incident_time = chrono::Utc::now().format("%H:%M").to_string();
    }
    
    // For drafts, we don't require location coordinates != 0.0
    // The create_draft_evidence function should handle drafts differently
    
    // ============ FIXED LINE: Use create_draft_evidence instead of create_evidence ============
    println!("📤 API_EVIDENCE_UPLOAD: Calling create_draft_evidence method");
    match evidence_service.create_draft_evidence(
        form,
        &user,
        files,
    ).await {
        Ok(evidence) => {
            println!("✅ API_EVIDENCE_UPLOAD: Draft evidence created successfully!");
            println!("   Evidence ID: {}", evidence.id);
            println!("   Evidence Number: {}", evidence.evidence_number);
            println!("   Media files: {}", evidence.media_files.len());
            println!("   Status: {:?}", evidence.status);
            
            // Verify it's actually a draft
            if evidence.status != EvidenceStatus::Draft {
                println!("⚠️ WARNING: Evidence created but status is {:?}, not Draft!", evidence.status);
            }
            
            let response_data = serde_json::json!({
                "success": true,
                "id": evidence.id,
                "evidence_number": evidence.evidence_number,
                "title": evidence.title,
                "status": format!("{:?}", evidence.status),
                "incident_type": format!("{:?}", evidence.incident_type),
                "emergency_level": format!("{:?}", evidence.emergency_level),
                "media_files": evidence.media_files.len(),
                "created_at": evidence.created_at.to_rfc3339(),
                "is_draft": matches!(evidence.status, EvidenceStatus::Draft),
                "redirect": format!("/evidence/complete/{}", evidence.id)
            });
            
            HttpResponse::Ok().json(ApiResponse::success(response_data))
        }
        Err(e) => {
            println!("❌ API_EVIDENCE_UPLOAD: Failed to create draft evidence: {}", e);
            
            // Check if it's a validation error
            let error_msg = if e.to_string().contains("Missing required fields") {
                "Failed to create evidence: Please ensure all required fields are filled"
            } else if e.to_string().contains("latitude") || e.to_string().contains("longitude") {
                "Failed to create evidence: Location coordinates are required. Please enable location services or enter coordinates manually."
            } else {
                &format!("Failed to create draft evidence: {}", e)
            };
            
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(error_msg))
        }
    }
}


/// Upload target photos API
pub async fn api_upload_targets(
    session: Session,
    data: web::Json<TargetUploadRequest>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("🎯 API_UPLOAD_TARGETS: Starting target photos upload");
    
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if let Some(user_id) = user_id {
        println!("🎯 API_UPLOAD_TARGETS: User ID: {}", user_id);
        println!("🎯 API_UPLOAD_TARGETS: Evidence ID: {}", data.evidence_id);
        println!("🎯 API_UPLOAD_TARGETS: Number of photos: {}", data.photos.len());
        
        for (i, photo) in data.photos.iter().enumerate() {
            println!("🎯 API_UPLOAD_TARGETS: Photo {}: {} ({} bytes)", 
                i, photo.filename, photo.data_base64.len());
        }
        
        match evidence_service.upload_targets(data.into_inner(), &user_id).await {
            Ok(targets) => {
                println!("✅ API_UPLOAD_TARGETS: {} target photos uploaded", targets.len());
                HttpResponse::Ok().json(ApiResponse::success(targets))
            }
            Err(e) => {
                println!("❌ API_UPLOAD_TARGETS: Failed: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e)))
            }
        }
    } else {
        HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"))
    }
}



/// Sign evidence API
pub async fn api_sign_evidence(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if let Some(user_id) = user_id {
        match evidence_service.sign_evidence(&evidence_id, &user_id, &auth_service).await {
            Ok(signature) => HttpResponse::Ok().json(ApiResponse::success(signature)),
            Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
        }
    } else {
        HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"))
    }
}

/// Search evidence API
pub async fn api_search_evidence(
    session: Session,
    data: web::Json<EvidenceSearchFilters>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    match evidence_service.search_evidence_with_filters(&data.into_inner(), user_id.as_deref().unwrap_or("")).await {
        Ok(result) => HttpResponse::Ok().json(ApiResponse::success(result)),
        Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
    }
}


/// Update evidence API
pub async fn api_update_evidence(
    session: Session,
    path: web::Path<String>,
    data: web::Json<EvidenceUpdate>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    let user_id = session.get::<String>("user_id").unwrap_or(None);
    
    if let Some(user_id) = user_id {
        match evidence_service.update_evidence(&evidence_id, &user_id, data.into_inner()).await {
            Ok(evidence) => HttpResponse::Ok().json(ApiResponse::success(evidence)),
            Err(e) => HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e))),
        }
    } else {
        HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"))
    }
}


// Add to media_routes.rs, in the API routes section
pub async fn api_get_evidence_locations(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("🗺️ API_GET_EVIDENCE_LOCATIONS: Fetching all evidence locations");
    
    // Check authentication (optional - can be public or require auth)
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🗺️ User authenticated: {}", email);
    } else {
        // You can make this endpoint public by removing this check
        println!("⚠️ User not authenticated, but proceeding with locations");
    }
    
    match evidence_service.get_all_evidence_locations().await {
        Ok(locations) => {
            println!("🗺️ Found {} evidence locations", locations.len());
            
            // Group by county for the stats section
            let mut county_stats: HashMap<String, (usize, usize)> = HashMap::new();
            for location in &locations {
                let entry = county_stats.entry(location.county.clone()).or_insert((0, 0));
                entry.0 += 1;
                if matches!(location.emergency_level, EmergencyLevel::Red | EmergencyLevel::Orange) {
                    entry.1 += 1;
                }
            }
            
            // Convert to vector and sort by count
            let mut county_stats_vec: Vec<(String, usize, usize)> = county_stats
                .into_iter()
                .map(|(county, (total, urgent))| (county, total, urgent))
                .collect();
            county_stats_vec.sort_by(|a, b| b.1.cmp(&a.1));
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "locations": locations,
                "stats": {
                    "total": locations.len(),
                    "counties": county_stats_vec.iter().take(5).map(|(county, total, urgent)| {
                        json!({
                            "county": county,
                            "total": total,
                            "urgent": urgent,
                            "percentage": if locations.len() > 0 {
                                (*total as f64 / locations.len() as f64 * 100.0).round() as i32
                            } else { 0 }
                        })
                    }).collect::<Vec<_>>(),
                    "urgent_count": locations.iter().filter(|l| 
                        matches!(l.emergency_level, EmergencyLevel::Red | EmergencyLevel::Orange)
                    ).count(),
                    "reported_count": locations.iter().filter(|l| 
                        matches!(l.status, EvidenceStatus::Reported)
                    ).count(),
                }
            })))
        }
        Err(e) => {
            println!("❌ API_GET_EVIDENCE_LOCATIONS: Error: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get locations: {}", e)))
        }
    }
}



// Add this function to media_routes.rs, after the existing routes

pub async fn targets_page(
    session: Session,
    auth_service: web::Data<AuthService>,
    evidence_service: web::Data<EvidenceService>,
    req: HttpRequest,
) -> HttpResponse {
    println!("🎯 TARGETS_PAGE: Loading targets page");
    
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🎯 TARGETS_PAGE: User email: {}", email);
        
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("🎯 TARGETS_PAGE: Profile not complete, redirecting");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
            }
            _ => {
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        let user_id = session.get::<String>("user_id").unwrap_or(None);
        let query_str = req.query_string();
        println!("🎯 TARGETS_PAGE: Query string: {}", query_str);
        
        let mut query_params = HashMap::new();
        for param in query_str.split('&') {
            if param.is_empty() { continue; }
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 {
                query_params.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        
        println!("🎯 TARGETS_PAGE: Query params: {:?}", query_params);
        
        let show_all = query_params.get("show").map(|v| v == "all").unwrap_or(true);
        let category_filter = query_params.get("category").cloned();
        let evidence_filter = query_params.get("evidence_id").cloned();
        let search_query = query_params.get("q").cloned();
        let view_mode = query_params.get("view").cloned().unwrap_or_else(|| "cards".to_string());
        
        // Load all evidence to get their targets
        let filters = EvidenceSearchFilters {
            query: search_query.clone(),
            incident_type: None,
            county: None,
            emergency_level: None,
            status: None,
            reported_to_police: None,
            needs_attention: None,
            signed_only: None,
            uploader_id: if show_all { None } else { user_id.clone() },
            date_from: None,
            date_to: None,
            start_date: None,
            end_date: None,
            sort_by: Some("newest".to_string()),
            page: 1,
            limit: 1000,
        };
        
        let search_result = match evidence_service.search_evidence_with_filters(&filters, user_id.as_deref().unwrap_or("")).await {
            Ok(result) => {
                println!("🎯 TARGETS_PAGE: Found {} evidence records", result.evidence.len());
                result
            }
            Err(e) => {
                println!("❌ TARGETS_PAGE: Error searching evidence: {}", e);
                EvidenceSearchResponse {
                    evidence: Vec::new(),
                    summaries: Vec::new(),
                    total: 0,
                    page: 1,
                    total_pages: 1,
                }
            }
        };
        
        // Build targets data structure
        let mut all_targets: Vec<(TargetPhoto, EvidenceSummary)> = Vec::new();
        let mut user_targets: Vec<(TargetPhoto, EvidenceSummary)> = Vec::new();
        let mut evidence_targets_map: HashMap<String, Vec<TargetPhoto>> = HashMap::new();
        
        for evidence in &search_result.evidence {
            let targets_result = evidence_service.get_evidence_targets(&evidence.id).await;
            if let Ok(targets) = targets_result {
                let evidence_summary = EvidenceSummary {
                    id: evidence.id.clone(),
                    evidence_number: evidence.evidence_number.clone(),
                    emergency_level: evidence.emergency_level.clone(),
                    incident_type: evidence.incident_type.clone(),
                    title: evidence.title.clone(),
                    county: evidence.location.county.clone(),
                    incident_time: evidence.incident_time,
                    status: evidence.status.clone(),
                    reported_to_police: evidence.reported_to_police,
                    police_case_id: evidence.police_case_id.clone(),
                    has_media: !evidence.media_files.is_empty(),
                    needs_attention: evidence.needs_attention,
                };
                
                evidence_targets_map.insert(evidence.id.clone(), targets.clone());
                
                for target in targets {
                    // Apply filters
                    let matches_category = category_filter.as_ref()
                        .map_or(true, |cat| {
                            match cat.as_str() {
                                "person" => matches!(target.category, TargetCategory::Person),
                                "vehicle" => matches!(target.category, TargetCategory::Vehicle),
                                "object" => matches!(target.category, TargetCategory::Object),
                                "location" => matches!(target.category, TargetCategory::Location),
                                "other" => matches!(target.category, TargetCategory::Other),
                                _ => true,
                            }
                        });
                    
                    let matches_evidence = evidence_filter.as_ref()
                        .map_or(true, |ev_id| &evidence.id == ev_id);
                    
                    let matches_search = search_query.as_ref()
                        .map_or(true, |q| {
                            target.description.as_ref().map_or(false, |desc| 
                                desc.to_lowercase().contains(&q.to_lowercase())
                            ) ||
                            target.filename.to_lowercase().contains(&q.to_lowercase()) ||
                            evidence.evidence_number.to_lowercase().contains(&q.to_lowercase()) ||
                            evidence.title.to_lowercase().contains(&q.to_lowercase())
                        });
                    
                    if matches_category && matches_evidence && matches_search {
                        all_targets.push((target.clone(), evidence_summary.clone()));
                        
                        // Check if this is user's target
                        if let Some(ref uid) = user_id {
                            if evidence.uploader_id == *uid {
                                user_targets.push((target, evidence_summary.clone()));
                            }
                        }
                    }
                }
            }
        }
        
        println!("🎯 TARGETS_PAGE: Found {} total targets, {} user targets", 
                all_targets.len(), user_targets.len());
        
        let targets_to_show = if show_all { &all_targets } else { &user_targets };
        let is_user_view = !show_all;
        
        // Build targets HTML based on view mode
        let targets_html = if targets_to_show.is_empty() {
            format!(r#"
                <div class="col-span-full px-6 py-12 text-center">
                    <div class="inline-block p-8 bg-gray-900/50 rounded-lg">
                        <i class="fas fa-bullseye text-4xl text-gray-700 mb-4"></i>
                        <h3 class="text-xl font-bold mb-2">No Targets Found</h3>
                        <p class="text-gray-400 mb-4">
                            {}
                        </p>
                        <a href="/evidence/browse" class="inline-flex items-center bg-blue-600 px-6 py-3 rounded-lg hover:bg-blue-700">
                            <i class="fas fa-search mr-2"></i>
                            Browse Evidence
                        </a>
                    </div>
                </div>
            "#, if is_user_view {
                "You haven't identified any targets yet."
            } else {
                "No targets have been identified in the system."
            })
        } else {
            if view_mode == "table" {
                build_targets_table(targets_to_show, &evidence_targets_map)
            } else {
                build_targets_cards(targets_to_show)
            }
        };
        
        // Statistics for the sidebar
        let total_targets = all_targets.len();
        let user_targets_count = user_targets.len();
        
        let category_stats = {
            let mut stats = HashMap::new();
            for (target, _) in &all_targets {
                *stats.entry(format!("{:?}", target.category)).or_insert(0) += 1;
            }
            stats
        };
        
        let mut context = HashMap::new();
        context.insert("email", email);
        context.insert("show_all", show_all.to_string());
        context.insert("total_targets", total_targets.to_string());
        context.insert("user_targets_count", user_targets_count.to_string());
        context.insert("targets_html", targets_html);
        context.insert("view_mode", view_mode.clone());
        context.insert("search_query", search_query.unwrap_or_default());
        context.insert("category_filter", category_filter.unwrap_or_default());
        context.insert("evidence_filter", evidence_filter.unwrap_or_default());
        
        // Add category stats to context
        context.insert("person_count", category_stats.get("Person").unwrap_or(&0).to_string());
        context.insert("vehicle_count", category_stats.get("Vehicle").unwrap_or(&0).to_string());
        context.insert("object_count", category_stats.get("Object").unwrap_or(&0).to_string());
        context.insert("location_count", category_stats.get("Location").unwrap_or(&0).to_string());
        context.insert("other_count", category_stats.get("Other").unwrap_or(&0).to_string());
        
        // Add Swiper JS and CSS to context
        let swiper_css = r#"
        <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/swiper@11/swiper-bundle.min.css">
        <style>
            .swiper {
                width: 100%;
                height: 100%;
            }
            
            .swiper-slide {
                text-align: center;
                font-size: 18px;
                background: #000;
                display: flex;
                justify-content: center;
                align-items: center;
            }
            
            .swiper-slide img {
                display: block;
                width: 100%;
                height: 100%;
                object-fit: cover;
            }
            
            .target-swiper {
                height: 300px;
                border-radius: 0.5rem;
                overflow: hidden;
            }
            
            .swiper-pagination-bullet {
                background: white;
                opacity: 0.5;
            }
            
            .swiper-pagination-bullet-active {
                background: #ef4444;
                opacity: 1;
            }
            
            .swiper-button-next, .swiper-button-prev {
                color: white;
                background: rgba(0,0,0,0.5);
                width: 40px;
                height: 40px;
                border-radius: 50%;
            }
            
            .swiper-button-next:after, .swiper-button-prev:after {
                font-size: 20px;
            }
            
            .table-target-thumbnail {
                width: 40px;
                height: 40px;
                border-radius: 50%;
                object-fit: cover;
                border: 2px solid #e5e7eb;
                transition: all 0.3s;
            }
            
            .table-target-thumbnail:hover {
                transform: scale(2);
                border-color: #ef4444;
                z-index: 10;
            }
        </style>
        "#;
        
        let swiper_js = r#"
        <script src="https://cdn.jsdelivr.net/npm/swiper@11/swiper-bundle.min.js"></script>
       
        "#;
        
        context.insert("swiper_css", swiper_css.to_string());
        context.insert("swiper_js", swiper_js.to_string());
        
        // Render the template
        let mut html = render_template("targets", &context);
        html = html.replace("{{#if show_all}}", if show_all { "" } else { "" });
        html = html.replace("{{/if}}", "");
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

// Helper function to build targets cards view
fn build_targets_cards(targets: &[(TargetPhoto, EvidenceSummary)]) -> String {
    let mut html = String::new();
    
    for (index, (target, evidence)) in targets.iter().enumerate() {
        let _target_number = index + 1;
        let _evidence_number_short = if evidence.evidence_number.len() > 10 {
            format!("...{}", &evidence.evidence_number[evidence.evidence_number.len()-10..])
        } else {
            evidence.evidence_number.clone()
        };
        
        let category_color = match target.category {
            TargetCategory::Person => "border-blue-500 bg-blue-50 dark:bg-blue-900/20",
            TargetCategory::Vehicle => "border-green-500 bg-green-50 dark:bg-green-900/20", 
            TargetCategory::Object => "border-yellow-500 bg-yellow-50 dark:bg-yellow-900/20",
            TargetCategory::Location => "border-purple-500 bg-purple-50 dark:bg-purple-900/20",
            TargetCategory::Other => "border-gray-500 bg-gray-50 dark:bg-gray-900/20",
        };
        
        let category_text = match target.category {
            TargetCategory::Person => "Person",
            TargetCategory::Vehicle => "Vehicle",
            TargetCategory::Object => "Object",
            TargetCategory::Location => "Location",
            TargetCategory::Other => "Other",
        };
        
        let category_icon = match target.category {
            TargetCategory::Person => "fas fa-user",
            TargetCategory::Vehicle => "fas fa-car",
            TargetCategory::Object => "fas fa-cube",
            TargetCategory::Location => "fas fa-map-marker-alt",
            TargetCategory::Other => "fas fa-question-circle",
        };
        
        let confidence_color = if target.confidence_score >= 80 {
            "text-green-600 bg-green-100 dark:text-green-400 dark:bg-green-900/30"
        } else if target.confidence_score >= 60 {
            "text-yellow-600 bg-yellow-100 dark:text-yellow-400 dark:bg-yellow-900/30"
        } else {
            "text-red-600 bg-red-100 dark:text-red-400 dark:bg-red-900/30"
        };
        
        let evidence_status_badge = match evidence.status {
            EvidenceStatus::Draft => r#"<span class="px-2 py-0.5 text-xs font-medium bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300 rounded">Draft</span>"#,
            EvidenceStatus::Submitted => r#"<span class="px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 rounded">Submitted</span>"#,
            EvidenceStatus::Reported => r#"<span class="px-2 py-0.5 text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400 rounded">Reported</span>"#,
            EvidenceStatus::UnderReview => r#"<span class="px-2 py-0.5 text-xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400 rounded">Review</span>"#,
            EvidenceStatus::Archived => r#"<span class="px-2 py-0.5 text-xs font-medium bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400 rounded">Archived</span>"#,
            EvidenceStatus::Rejected => r#"<span class="px-2 py-0.5 text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400 rounded">Rejected</span>"#,
        };
        
        let emergency_badge = match evidence.emergency_level {
            EmergencyLevel::Red => r#"<span class="px-2 py-0.5 text-xs font-medium bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-400 rounded">Red</span>"#,
            EmergencyLevel::Orange => r#"<span class="px-2 py-0.5 text-xs font-medium bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-400 rounded">Orange</span>"#,
            EmergencyLevel::Yellow => r#"<span class="px-2 py-0.5 text-xs font-medium bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400 rounded">Yellow</span>"#,
            EmergencyLevel::Blue => r#"<span class="px-2 py-0.5 text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400 rounded">Blue</span>"#,
        };
        
        let short_description = if let Some(desc) = &target.description {
            if desc.len() > 80 { format!("{}...", &desc[..77]) } else { desc.clone() }
        } else {
            "No description provided".to_string()
        };

        // ── Phase 5: Auto-generated indicator ────────────────────────────────
        let auto_badge = if target.auto_generated {
            r#"<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400 rounded-full border border-amber-200 dark:border-amber-700"
                    title="Automatically detected face — please review and confirm">
                <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                    <path d="M12 2a5 5 0 1 0 0 10A5 5 0 0 0 12 2z"/><path d="M12 14c-5.33 0-8 2.67-8 4v2h16v-2c0-1.33-2.67-4-8-4z"/>
                </svg>
                Auto-detected
            </span>"#
        } else {
            ""
        };

        // ── Phase 5: pHash stored indicator (non-face targets) ────────────────
        let phash_badge = if target.phash.is_some() && !matches!(target.category, TargetCategory::Person) {
            r#"<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-indigo-100 text-indigo-700 dark:bg-indigo-900/30 dark:text-indigo-400 rounded-full border border-indigo-200 dark:border-indigo-700"
                    title="Visual fingerprint stored — will match similar images in future uploads">
                <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M2 12C2 6.48 6.48 2 12 2s10 4.48 10 10-4.48 10-10 10S2 17.52 2 12z"/>
                    <path d="M8 12c0-2.21 1.79-4 4-4s4 1.79 4 4-1.79 4-4 4-4-1.79-4-4z"/>
                    <path d="M12 12h.01"/>
                </svg>
                pHash
            </span>"#
        } else {
            ""
        };

        // ── Phase 5: Face encoding indicator (person targets) ─────────────────
        let face_badge = if matches!(target.category, TargetCategory::Person) && !target.auto_generated {
            r#"<span class="inline-flex items-center gap-1 px-2 py-0.5 text-xs font-medium bg-cyan-100 text-cyan-700 dark:bg-cyan-900/30 dark:text-cyan-400 rounded-full border border-cyan-200 dark:border-cyan-700"
                    title="Face descriptor stored — will match this person in future uploads">
                <svg class="w-2.5 h-2.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <circle cx="12" cy="8" r="4"/><path d="M4 20c0-4 3.58-7 8-7s8 3 8 7"/>
                </svg>
                Face ID
            </span>"#
        } else {
            ""
        };

        // ── Phase 5: Dismiss button for auto-generated targets ────────────────
        let dismiss_button = if target.auto_generated {
            format!(r#"<button onclick="dismissAutoTarget('{}', this)"
                        class="text-xs text-amber-600 dark:text-amber-400 hover:text-red-600 dark:hover:text-red-400 flex items-center gap-1 transition-colors"
                        title="Remove this auto-detected target">
                <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M18 6L6 18M6 6l12 12"/>
                </svg>
                Dismiss
            </button>"#, target.id)
        } else {
            String::new()
        };

        // Create Swiper slides for multiple images
        let swiper_slides = format!(r#"
            <div class="swiper-wrapper">
                <div class="swiper-slide">
                    <div class="overflow-hidden">
                        <img src="{}" alt="{}" class="w-full h-64 object-cover">
                    </div>
                </div>
                <!-- Additional slides would be added here from evidence media -->
                <div class="swiper-slide">
                    <div class="overflow-hidden">
                        <img src="{}" alt="{} - alternative view" class="w-full h-64 object-cover">
                    </div>
                </div>
            </div>
            <div class="swiper-pagination"></div>
            <div class="swiper-button-prev">
                <svg class="h-auto w-auto stroke-current" width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M15.25 6L9 12.25L15.25 18.5" stroke="" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                </svg>
            </div>
            <div class="swiper-button-next">
                <svg class="stroke-current" width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                    <path d="M8.75 19L15 12.75L8.75 6.5" stroke="" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
                </svg>
            </div>
        "#, target.storj_url, target.filename, target.storj_url, target.filename);

        // ── Phase 5: auto-generated gets amber border treatment ───────────────
        let card_border = if target.auto_generated {
            "border-amber-300 dark:border-amber-700 bg-amber-50/30 dark:bg-amber-900/10"
        } else {
            "border-gray-200 dark:border-gray-700"
        };
        
        html.push_str(&format!(r#"
        <div class="group relative bg-white dark:bg-gray-800 rounded-xl border {} overflow-hidden hover:shadow-lg transition-all duration-300 hover:-translate-y-1 grid-item" 
             data-target-id="{}" data-category="{}" data-evidence-id="{}"
             data-filename="{}" data-description="{}" data-confidence="{}"
             data-evidence-number="{}" data-created-at="{}" data-file-size="{}"
             data-storj-url="{}" data-evidence-title="{}" data-mime-type="{}"
             data-target-number="{}" data-hash="{}"
             data-auto-generated="{}" data-phash="{}">
            <input type="checkbox" class="target-checkbox absolute top-3 left-3 z-10" value="{}" onchange="updateBulkActions()">
            
            <!-- Target image carousel -->
            <div class="swiper target-swiper">
                {}
            </div>
            
            <!-- Target details -->
            <div class="p-5">
                <!-- Category and evidence row -->
                <div class="flex items-center justify-between mb-3">
                    <div class="flex items-center space-x-2">
                        <span class="px-3 py-1 text-xs font-medium rounded-full {}">
                            <i class="{} mr-1"></i>
                            {}
                        </span>
                        {}
                    </div>
                    {}
                </div>

                <!-- Phase 5: Match type badges row -->
                <div class="flex flex-wrap gap-1.5 mb-3">
                    {}{}{}
                </div>

                <!-- Phase 5: Dismiss button for auto-detected targets -->
                {}
                
                <!-- Target description -->
                <h3 class="text-lg font-semibold text-gray-800 dark:text-white mb-2 line-clamp-1">
                    {}
                </h3>
                
                <p class="text-sm text-gray-600 dark:text-gray-300 mb-4 line-clamp-2">
                    {}
                </p>
                
                <!-- Evidence info -->
                <div class="space-y-2">
                    <div class="flex items-center justify-between text-sm">
                        <div class="flex items-center space-x-3">
                            <span class="text-gray-500 dark:text-gray-400">
                                <i class="fas fa-file-alt mr-1"></i>
                                {}
                            </span>
                            <span class="text-gray-500 dark:text-gray-400">
                                <i class="fas fa-calendar mr-1"></i>
                                {}
                            </span>
                        </div>
                        <a href="/evidence/view/{}" 
                           class="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 font-medium text-sm flex items-center">
                            View Evidence
                            <i class="fas fa-arrow-right ml-1"></i>
                        </a>
                    </div>
                    
                    <div class="flex items-center justify-between text-xs">
                        <div class="flex items-center space-x-2">
                            <span class="px-2 py-1 text-xs font-medium rounded-full {}">
                                {}% confidence
                            </span>
                            <span class="text-gray-500 dark:text-gray-400">
                                <i class="fas fa-database mr-1"></i>
                                {}
                            </span>
                        </div>
                        <div class="flex items-center space-x-1">
                            <button onclick="downloadTargetImage('{}', '{}')" 
                                    class="text-gray-500 hover:text-blue-600 dark:text-gray-400 dark:hover:text-blue-400 p-1"
                                    title="Download">
                                <i class="fas fa-download text-sm"></i>
                            </button>
                            <button onclick="zoomTargetImage('{}')" 
                                    class="text-gray-500 hover:text-green-600 dark:text-gray-400 dark:hover:text-green-400 p-1"
                                    title="Zoom">
                                <i class="fas fa-search-plus text-sm"></i>
                            </button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        "#,
        card_border,
        target.id,
        format!("{:?}", target.category).to_lowercase(),
        html_escape(&evidence.id),
        html_escape(&target.filename),
        html_escape(target.description.as_deref().unwrap_or("")),
        target.confidence_score,
        html_escape(&evidence.evidence_number),
        target.created_at.format("%Y-%m-%d %H:%M").to_string(),
        format_bytes(target.file_size),
        html_escape(&target.storj_url),
        html_escape(&if evidence.title.len() > 60 { format!("{}...", &evidence.title[..57]) } else { evidence.title.clone() }),
        html_escape(&target.mime_type),
        target.target_number,
        html_escape(&target.hash),
        target.auto_generated,
        html_escape(target.phash.as_deref().unwrap_or("")),
        target.id,
        swiper_slides,
        category_color,
        category_icon,
        category_text,
        evidence_status_badge,
        emergency_badge,
        // Phase 5 badges
        auto_badge,
        face_badge,
        phash_badge,
        dismiss_button,
        html_escape(target.description.as_deref().unwrap_or("Target")),
        html_escape(&short_description),
        html_escape(&if evidence.title.len() > 40 { format!("{}...", &evidence.title[..37]) } else { evidence.title.clone() }),
        evidence.incident_time.format("%d %b %Y").to_string(),
        html_escape(&evidence.id),
        confidence_color,
        target.confidence_score,
        format_bytes(target.file_size),
        html_escape(&target.storj_url),
        html_escape(&target.filename),
        html_escape(&target.storj_url)
        ));
    }
    
    html
}


// Helper function to build targets table view
fn build_targets_table(targets: &[(TargetPhoto, EvidenceSummary)], evidence_targets_map: &HashMap<String, Vec<TargetPhoto>>) -> String {
    let mut html = String::new();
    
    // Create an empty vector to use as default
    let empty_vec = Vec::new();
    
    html.push_str(r#"
    <div class="col-span-full bg-white dark:bg-gray-800 rounded-xl border border-gray-200 dark:border-gray-700 overflow-hidden">
        <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
                <thead>
                    <tr class="bg-gray-50 dark:bg-gray-900">
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            <input type="checkbox" id="selectAll" onchange="selectAllTargets()">
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Target
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Category
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Description
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Evidence
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Confidence
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Size
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Created
                        </th>
                        <th scope="col" class="px-6 py-3 text-left text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider">
                            Actions
                        </th>
                    </tr>
                </thead>
                <tbody class="bg-white dark:bg-gray-800 divide-y divide-gray-200 dark:divide-gray-700">
    "#);
    
    for (index, (target, evidence)) in targets.iter().enumerate() {
        let category_color = match target.category {
            TargetCategory::Person => "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
            TargetCategory::Vehicle => "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200", 
            TargetCategory::Object => "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200",
            TargetCategory::Location => "bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200",
            TargetCategory::Other => "bg-gray-100 text-gray-800 dark:bg-gray-900 dark:text-gray-200",
        };
        
        let category_text = match target.category {
            TargetCategory::Person => "Person",
            TargetCategory::Vehicle => "Vehicle",
            TargetCategory::Object => "Object",
            TargetCategory::Location => "Location",
            TargetCategory::Other => "Other",
        };
        
        let category_icon = match target.category {
            TargetCategory::Person => "fas fa-user",
            TargetCategory::Vehicle => "fas fa-car",
            TargetCategory::Object => "fas fa-cube",
            TargetCategory::Location => "fas fa-map-marker-alt",
            TargetCategory::Other => "fas fa-question-circle",
        };
        
        let confidence_color = if target.confidence_score >= 80 {
            "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200"
        } else if target.confidence_score >= 60 {
            "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200"
        } else {
            "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200"
        };
        
        // Get all targets for this evidence to show as thumbnail group
        let evidence_targets = evidence_targets_map.get(&evidence.id).unwrap_or(&empty_vec);
        let thumbnail_group = if evidence_targets.len() > 1 {
            let mut thumbnails = String::new();
            for (i, t) in evidence_targets.iter().enumerate().take(3) {
                thumbnails.push_str(&format!(r#"
                    <img src="{}" 
                         alt="Target {}" 
                         class="table-target-thumbnail {}"
                         style="margin-left: {}px;"
                         title="{}"
                         onclick="zoomTargetImage('{}')">
                "#, 
                t.storj_url,
                i + 1,
                if i == 0 { "" } else { "absolute" },
                (i as i32) * -10,
                t.filename,
                t.storj_url
                ));
            }
            if evidence_targets.len() > 3 {
                thumbnails.push_str(&format!(r#"
                    <div class="absolute table-target-thumbnail bg-gray-800 text-white text-xs flex items-center justify-center border-2 border-white dark:border-gray-700"
                         style="margin-left: 30px;">
                        +{}
                    </div>
                "#, evidence_targets.len() - 3));
            }
            format!(r#"<div class="flex relative h-10">{}</div>"#, thumbnails)
        } else if !evidence_targets.is_empty() {
            // Handle case with exactly one target
            format!(r#"
                <img src="{}" 
                     alt="{}" 
                     class="table-target-thumbnail cursor-pointer"
                     onclick="zoomTargetImage('{}')"
                     title="{}">
            "#, target.storj_url, target.filename, target.storj_url, target.filename)
        } else {
            // Handle case with no targets (shouldn't happen, but just in case)
            format!(r#"
                <div class="table-target-thumbnail bg-gray-200 dark:bg-gray-700 flex items-center justify-center">
                    <i class="fas fa-image text-gray-400"></i>
                </div>
            "#)
        };
        
        html.push_str(&format!(r#"
        <tr class="target-row hover:bg-gray-50 dark:hover:bg-gray-900 transition-colors"
            data-target-id="{}"
            data-filename="{}"
            data-category="{}"
            data-confidence="{}"
            data-evidence-id="{}"
            data-evidence-number="{}"
            data-description="{}"
            data-created-at="{}"
            data-file-size="{}">
            <td class="px-6 py-4 whitespace-nowrap">
                <input type="checkbox" class="target-checkbox" value="{}" onchange="updateBulkActions()">
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="flex items-center">
                    <div class="flex-shrink-0">
                        {}
                    </div>
                    <div class="ml-4">
                        <div class="text-sm font-medium text-gray-900 dark:text-white">
                            {}
                        </div>
                        <div class="text-sm text-gray-500 dark:text-gray-400">
                            ID: {}
                        </div>
                    </div>
                </div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class="px-3 py-1 inline-flex text-xs leading-5 font-semibold rounded-full {}">
                    <i class="{} mr-1"></i>
                    {}
                </span>
            </td>
            <td class="px-6 py-4">
                <div class="text-sm text-gray-900 dark:text-white max-w-xs truncate" title="{}">
                    {}
                </div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <div class="text-sm">
                    <a href="/evidence/view/{}" class="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 font-medium">
                        #{}
                    </a>
                    <div class="text-xs text-gray-500 dark:text-gray-400 truncate max-w-xs" title="{}">
                        {}
                    </div>
                </div>
            </td>
            <td class="px-6 py-4 whitespace-nowrap">
                <span class="px-3 py-1 inline-flex text-xs leading-5 font-semibold rounded-full {}">
                    {}%
                </span>
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                {}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500 dark:text-gray-400">
                {}
            </td>
            <td class="px-6 py-4 whitespace-nowrap text-sm font-medium">
                <div class="flex space-x-2">
                    <button onclick="zoomTargetImage('{}')" 
                            class="text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300"
                            title="View">
                        <i class="fas fa-eye"></i>
                    </button>
                    <button onclick="downloadTargetImage('{}', '{}')" 
                            class="text-green-600 dark:text-green-400 hover:text-green-800 dark:hover:text-green-300"
                            title="Download">
                        <i class="fas fa-download"></i>
                    </button>
                    <a href="/evidence/view/{}" 
                       class="text-purple-600 dark:text-purple-400 hover:text-purple-800 dark:hover:text-purple-300"
                       title="Go to Evidence">
                        <i class="fas fa-external-link-alt"></i>
                    </a>
                </div>
            </td>
        </tr>
        "#,
        target.id,
        target.filename,
        category_text,
        target.confidence_score,
        html_escape(&evidence.id),
        html_escape(&evidence.evidence_number),
        html_escape(target.description.as_deref().unwrap_or("")),
        target.created_at.format("%Y-%m-%d %H:%M").to_string(),
        target.file_size,
        target.id,
        thumbnail_group,
        html_escape(&target.filename),
        &target.id[..8],
        category_color,
        category_icon,
        category_text,
        html_escape(target.description.as_deref().unwrap_or("No description")),
        html_escape(target.description.as_deref().unwrap_or("No description")),
        html_escape(&evidence.id),
        html_escape(&evidence.evidence_number),
        html_escape(&evidence.title),
        html_escape(&if evidence.title.len() > 50 { format!("{}...", &evidence.title[..47]) } else { evidence.title.clone() }),
        confidence_color,
        target.confidence_score,
        format_bytes(target.file_size),
        target.created_at.format("%Y-%m-%d").to_string(),
        html_escape(&target.storj_url),
        html_escape(&target.storj_url),
        html_escape(&target.filename),
        html_escape(&evidence.id)
        ));
    }
    
    html.push_str(r#"
                </tbody>
            </table>
        </div>
    </div>
    "#);
    
    html
}

// Add to media_routes.rs, near the other page routes
pub async fn maps_dashboard_page(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
    req: HttpRequest,
) -> HttpResponse {
    println!("🗺️ MAPS_DASHBOARD_PAGE: Loading maps dashboard");
    
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🗺️ MAPS_DASHBOARD_PAGE: User email: {}", email);
        
        match auth_service.get_user_by_email(&email).await {
            Ok(Some(user)) => {
                if !user.is_profile_complete {
                    println!("🗺️ MAPS_DASHBOARD_PAGE: Profile not complete, redirecting");
                    return HttpResponse::SeeOther()
                        .append_header(("Location", "/profile/complete"))
                        .finish();
                }
            }
            _ => {
                let _ = session.clear();
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/login"))
                    .finish();
            }
        }
        
        // Parse query parameters for filters
        let query_str = req.query_string();
        let mut query_params = HashMap::new();
        for param in query_str.split('&') {
            if param.is_empty() { continue; }
            let parts: Vec<&str> = param.split('=').collect();
            if parts.len() == 2 {
                query_params.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
        
        println!("🗺️ MAPS_DASHBOARD_PAGE: Query params: {:?}", query_params);
        
        // Create filters from query parameters
        let filters = EvidenceLocationFilters {
            county: query_params.get("county").cloned(),
            emergency_level: query_params.get("emergency_level").cloned(),
            incident_type: query_params.get("incident_type").cloned(),
            status: query_params.get("status").cloned(),
            reported_to_police: query_params.get("reported").map(|v| v == "true"),
            date_from: query_params.get("date_from").cloned(),
            date_to: query_params.get("date_to").cloned(),
            search_query: query_params.get("q").cloned(),
        };
        
        // Get locations and statistics with filters
        let locations_result = evidence_service.get_evidence_locations_with_filters(&filters).await;
        let stats_result = evidence_service.get_evidence_map_statistics(&filters).await;
        
        let locations = match locations_result {
            Ok(locs) => {
                println!("🗺️ MAPS_DASHBOARD_PAGE: Found {} locations", locs.len());
                locs
            }
            Err(e) => {
                println!("❌ MAPS_DASHBOARD_PAGE: Error getting locations: {}", e);
                Vec::new()
            }
        };
        
        let stats = match stats_result {
            Ok(stats) => {
                println!("🗺️ MAPS_DASHBOARD_PAGE: Got statistics");
                stats
            }
            Err(e) => {
                println!("❌ MAPS_DASHBOARD_PAGE: Error getting statistics: {}", e);
                EvidenceMapStatistics {
                    total_evidence: 0,
                    urgent_count: 0,
                    reported_count: 0,
                    county_stats: HashMap::new(),
                    incident_stats: HashMap::new(),
                }
            }
        };
        
        // Prepare locations data for JavaScript
        let locations_json: Vec<serde_json::Value> = locations.iter().map(|loc| {
            let marker_color = match loc.emergency_level {
                EmergencyLevel::Red => "#ef4444",     // red
                EmergencyLevel::Orange => "#f97316", // orange
                EmergencyLevel::Yellow => "#eab308", // yellow
                EmergencyLevel::Blue => "#3b82f6",   // blue
            };
            
            let marker_icon = match loc.emergency_level {
                EmergencyLevel::Red => "🔴",
                EmergencyLevel::Orange => "🟠",
                EmergencyLevel::Yellow => "🟡",
                EmergencyLevel::Blue => "🔵",
            };
            
            let status_text = match loc.status {
                EvidenceStatus::Draft => "Draft",
                EvidenceStatus::Submitted => "Submitted",
                EvidenceStatus::Reported => "Reported to Police",
                EvidenceStatus::UnderReview => "Under Review",
                EvidenceStatus::Archived => "Archived",
                EvidenceStatus::Rejected => "Rejected",
            };
            
            json!({
                "id": loc.id,
                "evidence_number": loc.evidence_number,
                "title": loc.title,
                "emergency_level": format!("{:?}", loc.emergency_level),
                "incident_type": format!("{:?}", loc.incident_type),
                "county": loc.county,
                "latitude": loc.latitude,
                "longitude": loc.longitude,
                "incident_time": loc.incident_time.format("%Y-%m-%d %H:%M").to_string(),
                "created_time": loc.created_at.format("%Y-%m-%d %H:%M").to_string(),
                "status": status_text,
                "reported_to_police": loc.reported_to_police,
                "police_case_id": loc.police_case_id,
                "uploader_email": loc.uploader_email,
                "media_count": loc.media_count,
                "needs_attention": loc.needs_attention,
                "marker_color": marker_color,
                "marker_icon": marker_icon,
                "view_url": format!("/evidence/view/{}", loc.id),
            })
        }).collect();
        
        let locations_json_str = serde_json::to_string(&locations_json).unwrap_or_else(|_| "[]".to_string());
        
        // Build county stats HTML
        let county_stats_html = if stats.county_stats.is_empty() {
            r#"<div class="text-center py-8 text-gray-500">No county data available</div>"#.to_string()
        } else {
            let mut html = String::new();
            let mut index = 0;
            for (county, (total, urgent)) in stats.county_stats.iter().take(10) {
                index += 1;
                let percentage = if stats.total_evidence > 0 {
                    (*total as f64 / stats.total_evidence as f64 * 100.0) as i32
                } else {
                    0
                };
                
                let urgent_percentage = if *total > 0 {
                    (*urgent as f64 / *total as f64 * 100.0) as i32
                } else {
                    0
                };
                
                html.push_str(&format!(r#"
                    <div class="flex items-center justify-between p-3 hover:bg-gray-50 dark:hover:bg-gray-800/50 rounded-lg transition-colors">
                        <div class="flex items-center space-x-3">
                            <div class="w-10 h-10 rounded-full bg-blue-100 dark:bg-blue-900/30 flex items-center justify-center">
                                <span class="font-semibold text-blue-600 dark:text-blue-400">
                                    #{}
                                </span>
                            </div>
                            <div>
                                <div class="font-medium text-gray-800 dark:text-white/90">{}</div>
                                <div class="text-sm text-gray-500 dark:text-gray-400">
                                    {} cases • {}% urgent
                                </div>
                            </div>
                        </div>
                        <div class="flex flex-col items-end">
                            <div class="text-lg font-bold text-gray-800 dark:text-white">{}</div>
                            <div class="w-24 bg-gray-200 dark:bg-gray-700 rounded-full h-2 mt-1">
                                <div class="bg-blue-500 h-2 rounded-full" style="width: {}%"></div>
                            </div>
                        </div>
                    </div>
                "#, 
                index,
                county, 
                total, 
                urgent_percentage,
                total,
                percentage
                ));
            }
            html
        };
        
        // Build incident type stats
        let incident_stats_html = if stats.incident_stats.is_empty() {
            String::new()
        } else {
            let mut html = String::new();
            html.push_str(r#"<div class="space-y-3">"#);
            
            for (incident_type, count) in stats.incident_stats.iter() {
                let percentage = if stats.total_evidence > 0 {
                    (*count as f64 / stats.total_evidence as f64 * 100.0) as i32
                } else {
                    0
                };
                
                let type_color = match incident_type.as_str() {
                    "HitAndRun" => "bg-red-500",
                    "Assault" => "bg-orange-500",
                    "ThreatToLife" => "bg-red-600",
                    "PropertyDamage" => "bg-yellow-500",
                    "Theft" => "bg-purple-500",
                    _ => "bg-blue-500",
                };
                
                let type_text = match incident_type.as_str() {
                    "HitAndRun" => "Hit & Run",
                    "Assault" => "Assault",
                    "ThreatToLife" => "Threat to Life",
                    "PropertyDamage" => "Property Damage",
                    "Theft" => "Theft",
                    _ => incident_type,
                };
                
                html.push_str(&format!(r#"
                    <div class="flex items-center justify-between">
                        <div class="flex items-center space-x-2">
                            <div class="w-2 h-2 rounded-full {}"></div>
                            <span class="text-sm font-medium text-gray-700 dark:text-gray-300">{}</span>
                        </div>
                        <div class="flex items-center space-x-3">
                            <span class="text-sm text-gray-500 dark:text-gray-400">{}</span>
                            <div class="w-20 bg-gray-200 dark:bg-gray-700 rounded-full h-2">
                                <div class="{} h-2 rounded-full" style="width: {}%"></div>
                            </div>
                        </div>
                    </div>
                "#, 
                type_color,
                type_text,
                count,
                type_color,
                percentage
                ));
            }
            
            html.push_str("</div>");
            html
        };
        
        // Kenya counties for filter dropdown
        let kenya_counties = vec![
            "Nairobi", "Mombasa", "Kisumu", "Nakuru", "Eldoret", "Thika", "Malindi", "Kitale",
            "Garissa", "Kakamega", "Kisii", "Meru", "Nyeri", "Machakos", "Kiambu", "Kilifi",
            "Bungoma", "Busia", "Embu", "Homa Bay", "Isiolo", "Kajiado", "Kericho", "Kirinyaga",
            "Kitui", "Kwale", "Laikipia", "Lamu", "Mandera", "Marsabit", "Migori", "Murang'a",
            "Nyamira", "Nyandarua", "Narok", "Samburu", "Siaya", "Taita Taveta", "Tana River",
            "Trans Nzoia", "Turkana", "Uasin Gishu", "Vihiga", "Wajir", "West Pokot", "all"
        ];
        
        let county_options: String = kenya_counties.iter().map(|county| {
            if query_params.get("county").unwrap_or(&"all".to_string()) == *county {
                format!("<option value=\"{}\" selected>{}</option>", county, county)
            } else {
                format!("<option value=\"{}\">{}</option>", county, county)
            }
        }).collect();
        
        // Emergency levels for filter
        let emergency_levels = vec![
            ("all", "All Levels"),
            ("red", "Red - Emergency"),
            ("orange", "Orange - High"),
            ("yellow", "Yellow - Medium"),
            ("blue", "Blue - Low")
        ];
        
        let emergency_options: String = emergency_levels.iter().map(|(value, label)| {
            if query_params.get("emergency_level").unwrap_or(&"all".to_string()) == *value {
                format!("<option value=\"{}\" selected>{}</option>", value, label)
            } else {
                format!("<option value=\"{}\">{}</option>", value, label)
            }
        }).collect();
        
        // Incident types for filter
        let incident_types = vec![
            ("all", "All Types"),
            ("HitAndRun", "Hit & Run"),
            ("Assault", "Assault"),
            ("ThreatToLife", "Threat to Life"),
            ("PropertyDamage", "Property Damage"),
            ("Theft", "Theft"),
            ("Other", "Other")
        ];
        
        let incident_options: String = incident_types.iter().map(|(value, label)| {
            if query_params.get("incident_type").unwrap_or(&"all".to_string()) == *value {
                format!("<option value=\"{}\" selected>{}</option>", value, label)
            } else {
                format!("<option value=\"{}\">{}</option>", value, label)
            }
        }).collect();
        
        // Status options for filter
        let statuses = vec![
            ("all", "All Status"),
            ("draft", "Draft"),
            ("submitted", "Submitted"),
            ("reported", "Reported to Police"),
            ("under_review", "Under Review"),
            ("archived", "Archived"),
        ];
        
        let status_options: String = statuses.iter().map(|(value, label)| {
            if query_params.get("status").unwrap_or(&"all".to_string()) == *value {
                format!("<option value=\"{}\" selected>{}</option>", value, label)
            } else {
                format!("<option value=\"{}\">{}</option>", value, label)
            }
        }).collect();
        
        // Get today's date for date filters
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let search_query = query_params.get("q").cloned().unwrap_or_default();
        let date_from = query_params.get("date_from").cloned().unwrap_or_default();
        let date_to = query_params.get("date_to").cloned().unwrap_or_default();
        
        // Build context
        let mut context = HashMap::new();
        context.insert("email", email);
        context.insert("total_evidence", stats.total_evidence.to_string());
        context.insert("urgent_count", stats.urgent_count.to_string());
        context.insert("reported_count", stats.reported_count.to_string());
        context.insert("locations_json", locations_json_str);
        context.insert("locations_count", locations.len().to_string());
        context.insert("county_stats_html", county_stats_html);
        context.insert("incident_stats_html", incident_stats_html);
        context.insert("county_options", county_options);
        context.insert("emergency_options", emergency_options);
        context.insert("incident_options", incident_options);
        context.insert("status_options", status_options);
        context.insert("search_query", search_query);
        context.insert("today", today);
        context.insert("date_from", date_from);
        context.insert("date_to", date_to);
        
        let html = render_template("maps_dashboard", &context);
        HttpResponse::Ok().body(html)
    } else {
        HttpResponse::SeeOther()
            .append_header(("Location", "/login"))
            .finish()
    }
}

// Add API route for getting map data
pub async fn api_get_map_data(
    session: Session,
    web::Query(filters): web::Query<EvidenceLocationFilters>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("🗺️ API_GET_MAP_DATA: Getting map data with filters: {:?}", filters);
    
    // Check authentication (optional - can be public or require auth)
    if let Some(email) = session.get::<String>("user_email").unwrap_or(None) {
        println!("🗺️ User authenticated: {}", email);
    } else {
        println!("⚠️ User not authenticated, but proceeding with map data");
    }
    
    match evidence_service.get_evidence_locations_with_filters(&filters).await {
        Ok(locations) => {
            println!("🗺️ Found {} evidence locations", locations.len());
            
            let locations_data: Vec<serde_json::Value> = locations.iter().map(|loc| {
                let marker_color = match loc.emergency_level {
                    EmergencyLevel::Red => "#ef4444",
                    EmergencyLevel::Orange => "#f97316",
                    EmergencyLevel::Yellow => "#eab308",
                    EmergencyLevel::Blue => "#3b82f6",
                };
                
                let marker_icon = match loc.emergency_level {
                    EmergencyLevel::Red => "🔴",
                    EmergencyLevel::Orange => "🟠",
                    EmergencyLevel::Yellow => "🟡",
                    EmergencyLevel::Blue => "🔵",
                };
                
                json!({
                    "id": loc.id,
                    "evidence_number": loc.evidence_number,
                    "title": loc.title,
                    "emergency_level": format!("{:?}", loc.emergency_level),
                    "incident_type": format!("{:?}", loc.incident_type),
                    "county": loc.county,
                    "latitude": loc.latitude,
                    "longitude": loc.longitude,
                    "incident_time": loc.incident_time.format("%Y-%m-%d %H:%M").to_string(),
                    "status": format!("{:?}", loc.status),
                    "reported_to_police": loc.reported_to_police,
                    "police_case_id": loc.police_case_id,
                    "uploader_email": loc.uploader_email,
                    "marker_color": marker_color,
                    "marker_icon": marker_icon,
                    "view_url": format!("/evidence/view/{}", loc.id),
                })
            }).collect();
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "locations": locations_data,
                "total": locations.len(),
            })))
        }
        Err(e) => {
            println!("❌ API_GET_MAP_DATA: Error: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get map data: {}", e)))
        }
    }
}

// Add this function to your media_routes.rs file
pub async fn user_profile_page(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    auth_service: web::Data<AuthService>,
    req: HttpRequest,
) -> HttpResponse {
    println!("👤 USER_PROFILE_PAGE: Loading user profile");
    
    // Get email from session
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    };
    
    // Get user from database
    let user = match auth_service.get_user_by_email(&email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let _ = session.clear();
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
        Err(e) => {
            println!("❌ Error getting user: {}", e);
            let _ = session.clear();
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    };
    
    // Get user statistics
    let stats = match evidence_service.get_dashboard_stats(&user.id).await {
        Ok(stats) => stats,
        Err(e) => {
            println!("⚠️ Error getting stats: {}", e);
            DashboardStats {
                total_evidence: 0,
                urgent_count: 0,
                reported_count: 0,
                needs_attention_count: 0,
                today_count: 0,
                by_county: Vec::new(),
                by_type: Vec::new(),
            }
        }
    };
    
    // Get wallet connections
    let wallet_connections = match auth_service.get_wallet_connections(&email).await {
        Ok(connections) => connections,
        Err(e) => {
            println!("⚠️ Error getting wallet connections: {}", e);
            Vec::new()
        }
    };
    
    // Get activity logs
    let activity_logs = match auth_service.get_user_activity_logs(&user.id, 10).await {
        Ok(logs) => logs,
        Err(e) => {
            println!("⚠️ Error getting activity logs: {}", e);
            Vec::new()
        }
    };
    
    // Helper function for time formatting
    fn format_time_ago(dt: chrono::DateTime<chrono::Utc>) -> String {
        let now = chrono::Utc::now();
        let duration = now - dt;
        
        if duration.num_seconds() < 60 {
            format!("{}s ago", duration.num_seconds())
        } else if duration.num_minutes() < 60 {
            format!("{}m ago", duration.num_minutes())
        } else if duration.num_hours() < 24 {
            format!("{}h ago", duration.num_hours())
        } else if duration.num_days() < 30 {
            format!("{}d ago", duration.num_days())
        } else if duration.num_days() < 365 {
            format!("{}mo ago", duration.num_days() / 30)
        } else {
            format!("{}y ago", duration.num_days() / 365)
        }
    }
    
    // Build activity timeline HTML
    // In the activity_timeline building section, replace this block:


    // Build activity timeline HTML
let activity_timeline = if activity_logs.is_empty() {
    r#"<div class="text-center py-8 text-gray-500"><i class="fas fa-history text-3xl mb-3"></i><p>No activity recorded yet</p></div>"#.to_string()
} else {
    let mut html = String::new();
    
    // Main container
    html.push_str(r#"<div class="rounded-2xl border border-gray-200 bg-white p-6 dark:border-gray-800 dark:bg-white/[0.03]">
  <div class="mb-6 flex justify-between">
    <div>
      <h3 class="text-lg font-semibold text-gray-800 dark:text-white/90">
        Activities
      </h3>
    </div>
    <div x-data="{openDropDown: false}" class="relative h-fit">
      <button @click="openDropDown = !openDropDown" :class="openDropDown ? 'text-gray-700 dark:text-white' : 'text-gray-400 hover:text-gray-700 dark:hover:text-white'" class="text-gray-400 hover:text-gray-700 dark:hover:text-white">
        <svg class="fill-current" width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path fill-rule="evenodd" clip-rule="evenodd" d="M10.2441 6C10.2441 5.0335 11.0276 4.25 11.9941 4.25H12.0041C12.9706 4.25 13.7541 5.0335 13.7541 6C13.7541 6.9665 12.9706 7.75 12.0041 7.75H11.9941C11.0276 7.75 10.2441 6.9665 10.2441 6ZM10.2441 18C10.2441 17.0335 11.0276 16.25 11.9941 16.25H12.0041C12.9706 16.25 13.7541 17.0335 13.7541 18C13.7541 18.9665 12.9706 19.75 12.0041 19.75H11.9941C11.0276 19.75 10.2441 18.9665 10.2441 18ZM11.9941 10.25C11.0276 10.25 10.2441 11.0335 10.2441 12C10.2441 12.9665 11.0276 13.75 11.9941 13.75H12.0041C12.9706 13.75 13.7541 12.9665 13.7541 12C13.7541 11.0335 12.9706 10.25 12.0041 10.25H11.9941Z" fill=""></path>
        </svg>
      </button>
      <div x-show="openDropDown" @click.outside="openDropDown = false" class="shadow-theme-lg dark:bg-gray-dark absolute top-full right-0 z-40 w-40 space-y-1 rounded-2xl border border-gray-200 bg-white p-2 dark:border-gray-800" style="display: none;">
        <button class="text-theme-xs flex w-full rounded-lg px-3 py-2 text-left font-medium text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-white/5 dark:hover:text-gray-300">
          View More
        </button>
        <button class="text-theme-xs flex w-full rounded-lg px-3 py-2 text-left font-medium text-gray-500 hover:bg-gray-100 hover:text-gray-700 dark:text-gray-400 dark:hover:bg-white/5 dark:hover:text-gray-300">
          Delete
        </button>
      </div>
    </div>
  </div>
  <div class="relative">
    <!-- Timeline line -->"#);
    
    // Add timeline line only if we have activities
    if !activity_logs.is_empty() {
        html.push_str(r#"<div class="absolute top-6 bottom-10 left-5 w-px bg-gray-200 dark:bg-gray-800"></div>"#);
    }
    
    // Activity items
    for log in activity_logs.iter().take(10) {
        let action_icon = match log.action_type.as_str() {
            "user_login" => "fas fa-sign-in-alt text-blue-500",
            "evidence_created" => "fas fa-file-upload text-green-500",
            "evidence_updated" => "fas fa-edit text-yellow-500",
            "evidence_signed" => "fas fa-signature text-purple-500",
            "evidence_reported" => "fas fa-shield-alt text-red-500",
            "wallet_connected" => "fas fa-wallet text-indigo-500",
            "password_changed" => "fas fa-key text-gray-500",
            "profile_updated" => "fas fa-user-edit text-teal-500",
            _ => "fas fa-history text-gray-400",
        };
        
        let time_ago = format_time_ago(log.created_at);
        
        // Get user initial from user_id or use default - handle Option<String>
        let user_initial = log.user_id.as_ref()
            .and_then(|id| id.chars().next())
            .map(|c| c.to_string())
            .unwrap_or_else(|| "U".to_string());
        
        // SVG icon based on action type (use a simpler SVG without # character issues)
        let svg_icon = match log.action_type.as_str() {
            "evidence_created" => r#"<svg width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
            <path d="M9 5.0625H14.0625L12.5827 8.35084C12.4506 8.64443 12.4506 8.98057 12.5827 9.27416L14.0625 12.5625H10.125C9.50368 12.5625 9 12.0588 9 11.4375V10.875M3.9375 10.875H9M3.9375 3.375H7.875C8.49632 3.375 9 3.87868 9 4.5V10.875M3.9375 15.9375V2.0625" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"></path>
          </svg>"#,
            _ => "", // Default empty for other actions
        };
        
        // Action label color and text
        let (action_color, action_label) = match log.action_type.as_str() {
            "user_login" => ("text-blue-500", "User login"),
            "evidence_created" => ("text-green-500", "New evidence"),
            "evidence_updated" => ("text-yellow-500", "Evidence updated"),
            "evidence_signed" => ("text-purple-500", "Evidence signed"),
            "evidence_reported" => ("text-red-500", "Evidence reported"),
            "wallet_connected" => ("text-indigo-500", "Wallet connected"),
            "password_changed" => ("text-gray-500", "Password changed"),
            "profile_updated" => ("text-teal-500", "Profile updated"),
            _ => ("text-gray-400", "Activity"),
        };
        
    for (index, log) in activity_logs.iter().take(50).enumerate() {
        let time_ago = format_time_ago(log.created_at);
        let formatted_date = format!("{}", log.created_at.format("%d %b, %Y"));
        
        // Get user display name
        let user_display_name = log.user_id.as_ref()
            .map(|id| id.clone())
            .unwrap_or_else(|| "System".to_string());
        
        let action_type_display = format_action_type(&log.action_type);
        
        // Determine status and color based on action type
        let (status_text, status_class) = match log.action_type.as_str() {
            "evidence_created" | "evidence_signed" | "user_login" | "wallet_connected" | "profile_updated" => 
                ("Completed", "bg-success-50 dark:bg-success-500/15 text-success-700 dark:text-success-500"),
            "evidence_updated" | "password_changed" => 
                ("Updated", "bg-warning-50 dark:bg-warning-500/15 text-warning-700 dark:text-warning-500"),
            "evidence_reported" => 
                ("Reported", "bg-red-50 dark:bg-red-500/15 text-red-700 dark:text-red-500"),
            _ => 
                ("Processed", "bg-blue-50 dark:bg-blue-500/15 text-blue-700 dark:text-blue-500"),
        };
        
        // Get target info (evidence ID, etc.)
        let target_info = if let Some(target_id) = &log.target_id {
            if !target_id.is_empty() {
                format!("ID: {}", target_id)
            } else {
                "N/A".to_string()
            }
        } else {
            "N/A".to_string()
        };
        
        html.push_str(&format!(r#"
        <tr class="transition hover:bg-gray-50 dark:hover:bg-gray-900">
          <td class="p-4 whitespace-nowrap">
            <div class="group flex items-center gap-3">
              <label class="flex cursor-pointer items-center text-sm font-medium text-gray-700 select-none dark:text-gray-400">
                <span class="relative">
                  <input type="checkbox" class="sr-only" value="{}" data-id="{}">
                  <span class="activity-checkbox-display flex h-4 w-4 items-center justify-center rounded-sm border-[1.25px] bg-transparent border-gray-300 dark:border-gray-700">
                    <span class="opacity-0">
                      <svg width="12" height="12" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M10 3L4.5 8.5L2 6" stroke="white" stroke-width="1.6666" stroke-linecap="round" stroke-linejoin="round"></path>
                      </svg>
                    </span>
                  </span>
                </span>
              </label>
              <span class="text-theme-xs font-medium text-gray-700 dark:text-gray-400">#{}</span>
            </div>
          </td>
          <td class="p-4 whitespace-nowrap">
            <span class="text-sm font-medium text-gray-700 dark:text-gray-400">{}</span>
          </td>
          <td class="p-4 whitespace-nowrap">
            <span class="text-sm text-gray-700 dark:text-gray-400">{}</span>
          </td>
          <td class="p-4">
            <p class="text-sm text-gray-500 dark:text-gray-400 max-w-xs truncate">{}</p>
          </td>
          <td class="p-4 whitespace-nowrap">
            <p class="text-sm text-gray-500 dark:text-gray-400">{}</p>
          </td>
          <td class="p-4 whitespace-nowrap">
            <span class="text-theme-xs rounded-full px-2 py-0.5 font-medium {}">{}</span>
          </td>
          <td class="p-4 whitespace-nowrap">
            <div>
              <p class="text-sm text-gray-700 dark:text-gray-400">{}</p>
              <p class="text-xs text-gray-500 dark:text-gray-400">{}</p>
            </div>
          </td>
          <td class="p-4 whitespace-nowrap">
            <div class="relative flex justify-center">
              <button class="text-gray-500 dark:text-gray-400 activity-action-btn" data-id="{}">
                <svg class="fill-current" width="24" height="24" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                  <path fill-rule="evenodd" clip-rule="evenodd" d="M5.99902 10.245C6.96552 10.245 7.74902 11.0285 7.74902 11.995V12.005C7.74902 12.9715 6.96552 13.755 5.99902 13.755C5.03253 13.755 4.24902 12.9715 4.24902 12.005V11.995C4.24902 11.0285 5.03253 10.245 5.99902 10.245ZM17.999 10.245C18.9655 10.245 19.749 11.0285 19.749 11.995V12.005C19.749 12.9715 18.9655 13.755 17.999 13.755C17.0325 13.755 16.249 12.9715 16.249 12.005V11.995C16.249 11.0285 17.0325 10.245 17.999 10.245ZM13.749 11.995C13.749 11.0285 12.9655 10.245 11.999 10.245C11.0325 10.245 10.249 11.0285 10.249 11.995V12.005C10.249 12.9715 11.0325 13.755 11.999 13.755C12.9655 13.755 13.749 12.9715 13.749 12.005V11.995Z" fill=""></path>
                </svg>
              </button>
            </div>
          </td>
        </tr>"#,
        log.id, log.id, log.id,
        user_display_name,
        action_type_display,
        log.details,
        target_info,
        status_class,
        status_text,
        formatted_date,
        time_ago,
        log.id
    ));
    }
}
    
    // Close containers
    html.push_str(r#"
  </div>
</div>"#);
    
    html
};

// Calculate activity items with interactive HTML
let mut activity_items_html = String::new();
activity_items_html.push_str(r#"<div class="flex flex-col gap-2">"#);

// Helper function to generate activity item
fn generate_activity_item(date: &str, time: &str, title: &str, description: &str) -> String {
    format!(r#"
        <div x-data="{{checked: false}}" @click="checked = !checked" class="flex cursor-pointer items-center gap-9 rounded-lg p-3 hover:bg-gray-50 dark:hover:bg-white/[0.03]">
          <div class="flex items-start gap-3">
            <div class="flex h-5 w-5 items-center justify-center rounded-md border-[1.25px] bg-white dark:bg-white/0 border-gray-300 dark:border-gray-700" :class="checked ? 'border-brand-500 dark:border-brand-500 bg-brand-500' : 'bg-white dark:bg-white/0 border-gray-300 dark:border-gray-700' ">
              <svg :class="checked ? 'block' : 'hidden'" width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg" class="hidden">
                <path d="M11.6668 3.5L5.25016 9.91667L2.3335 7" stroke="white" stroke-width="1.94437" stroke-linecap="round" stroke-linejoin="round"></path>
              </svg>
            </div>
            <div>
              <span class="mb-0.5 block text-theme-xs text-gray-500 dark:text-gray-400">
                {}
              </span>
              <span class="text-theme-sm font-medium text-gray-700 dark:text-gray-400">
                {}
              </span>
            </div>
          </div>
          <div>
            <span class="mb-1 block text-theme-sm font-medium text-gray-700 dark:text-gray-400">
              {}
            </span>
            <span class="text-theme-xs text-gray-500 dark:text-gray-400">
              {}
            </span>
          </div>
        </div>"#,
        date, time, title, description
    )
}

// Get current date and time for activity items
let now = Local::now();
let mut activity_count = 0;

// Predefined dates for activity items (similar to your example)
let dates = vec!["Wed, 11 jan", "Fri, 15 feb", "Thu, 18 mar", "Mon, 22 apr", "Tue, 30 may"];
let times = vec!["09:20 AM", "10:35 AM", "1:15 AM", "3:45 PM", "11:10 AM"];
let mut date_index = 0;

// Add activity items based on user status
if user.is_verified {
    let date = dates.get(date_index).unwrap_or(&"Today");
    let time = times.get(date_index).unwrap_or(&"Now");
    let title = "Account Verified";
    let description = "Your account has been successfully verified";
    activity_items_html.push_str(&generate_activity_item(date, time, title, description));
    activity_count += 1;
    date_index += 1;
}

if user.is_profile_complete {
    let date = dates.get(date_index).unwrap_or(&"Yesterday");
    let time = times.get(date_index).unwrap_or(&"Morning");
    let title = "Profile Complete";
    let description = "All profile information has been completed";
    activity_items_html.push_str(&generate_activity_item(date, time, title, description));
    activity_count += 1;
    date_index += 1;
}

if !wallet_connections.is_empty() {
    let date = dates.get(date_index).unwrap_or(&"3 days ago");
    let time = times.get(date_index).unwrap_or(&"Afternoon");
    let title = "Wallet Connected";
    let description = "Cryptocurrency wallet successfully connected";
    activity_items_html.push_str(&generate_activity_item(date, time, title, description));
    activity_count += 1;
    date_index += 1;
}

if stats.total_evidence > 0 {
    let date = dates.get(date_index).unwrap_or(&"Last week");
    let time = times.get(date_index).unwrap_or(&"Evening");
    let title = format!("{} Evidence Submitted", stats.total_evidence);
    let description = format!("Submitted {} pieces of evidence", stats.total_evidence);
    activity_items_html.push_str(&generate_activity_item(date, time, &title, &description));
    activity_count += 1;
    date_index += 1;
}

if stats.reported_count > 0 {
    let date = dates.get(date_index).unwrap_or(&"10 days ago");
    let time = times.get(date_index).unwrap_or(&"Night");
    let title = format!("{} Police Reports", stats.reported_count);
    let description = format!("Filed {} police reports", stats.reported_count);
    activity_items_html.push_str(&generate_activity_item(date, time, &title, &description));
    activity_count += 1;
}

// If no activities, show a placeholder
if activity_count == 0 {
    activity_items_html.push_str(r#"
        <div class="flex items-center gap-9 rounded-lg p-3">
          <div class="flex items-start gap-3">
            <div class="flex h-5 w-5 items-center justify-center rounded-md border-[1.25px] border-gray-300 dark:border-gray-700"></div>
            <div>
              <span class="mb-0.5 block text-theme-xs text-gray-500 dark:text-gray-400">
                No activities yet
              </span>
              <span class="text-theme-sm font-medium text-gray-700 dark:text-gray-400">
                --
              </span>
            </div>
          </div>
          <div>
            <span class="mb-1 block text-theme-sm font-medium text-gray-700 dark:text-gray-400">
              Get Started
            </span>
            <span class="text-theme-xs text-gray-500 dark:text-gray-400">
              Complete your profile and submit evidence
            </span>
          </div>
        </div>"#);
}

activity_items_html.push_str(r#"</div>"#);


    
    // Create wallet HTML
    let wallets_html = if wallet_connections.is_empty() {
        r#"<div class="bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800 rounded-lg p-6">
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <div class="w-12 h-12 rounded-full bg-yellow-100 dark:bg-yellow-900/30 flex items-center justify-center">
                        <i class="fas fa-wallet text-yellow-600 dark:text-yellow-400 text-xl"></i>
                    </div>
                    <div>
                        <h3 class="text-lg font-semibold text-gray-900 dark:text-white">No Wallet Connected</h3>
                        <p class="text-sm text-gray-600 dark:text-gray-300">Connect a wallet to sign and monetize evidence</p>
                    </div>
                </div>
                <a href="/connect-wallet" class="bg-gradient-to-r from-yellow-500 to-orange-500 hover:from-yellow-600 hover:to-orange-600 text-white px-6 py-3 rounded-lg font-medium transition-all hover:scale-105">
                    <i class="fas fa-plus mr-2"></i>Connect Wallet
                </a>
            </div>
        </div>"#.to_string()
    } else {
        let mut html = String::new();
        for (index, wallet) in wallet_connections.iter().enumerate() {
            let addr = &wallet.wallet_address;
            let addr_short = if addr.len() > 12 {
                format!("{}...{}", &addr[0..6], &addr[addr.len()-4..])
            } else {
                addr.clone()
            };
            
            let chain_class = match wallet.chain.as_str() {
                "ethereum" => "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400",
                "base" => "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
                "avalanche" => "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
                _ => "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400",
            };
            
            html.push_str(&format!(r#"
                <div class="my-6">
                    <div class="flex items-center justify-between border-b border-gray-100 pb-4 dark:border-gray-800">
                        <span class="text-theme-xs text-gray-400">Element</span>
                        <span class="text-right text-theme-xs text-gray-400">Value</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Chain Class</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400 px-2 py-1 text-xs font-medium rounded {}">{}</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Chain ID</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Wallet Address</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Short Address</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Connected On</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
                    </div>
                    <div class="flex items-center justify-between border-b border-gray-100 py-3 dark:border-gray-800">
                        <span class="text-theme-sm text-gray-500 dark:text-gray-400">Last Used</span>
                        <span class="text-right text-theme-sm text-gray-500 dark:text-gray-400">{}</span>
                    </div>
                    <div class="mt-4 flex space-x-2">
                        <button onclick="changeWallet('{}')" class="px-3 py-1 text-sm bg-blue-50 dark:bg-blue-900/30 text-blue-600 dark:text-blue-400 hover:bg-blue-100 dark:hover:bg-blue-900/50 rounded">
                            <i class="fas fa-exchange-alt mr-1"></i>Change Wallet
                        </button>
                        <form method="POST" action="/api/wallet/disconnect" class="inline">
                            <input type="hidden" name="wallet_address" value="{}">
                            <button type="submit" onclick="return confirm('Disconnect this wallet?')" class="px-3 py-1 text-sm bg-red-50 dark:bg-red-900/30 text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/50 rounded">
                                <i class="fas fa-unlink mr-1"></i>Disconnect
                            </button>
                        </form>
                    </div>
                </div>
            "#,
            chain_class,
            wallet.chain,
            wallet.chain,
            wallet.wallet_address,
            addr_short,
            wallet.connected_at.format("%b %d, %Y %H:%M"),
            wallet.last_used.format("%b %d, %Y %H:%M"),
            wallet.wallet_address,
            wallet.wallet_address
            ));
        }
        html
    };
    
    // Create statistics HTML
    let stats_html = format!(r#"
        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:gap-6 xl:grid-cols-4">
            <div class="rounded-2xl border border-gray-200 bg-white p-5 dark:border-gray-800 dark:bg-white/[0.03]">
                <p class="text-theme-sm text-gray-500 dark:text-gray-400">Total Evidence</p>
                <div class="mt-3 flex items-end justify-between">
                    <div><h4 class="text-2xl font-bold text-gray-800 dark:text-white/90">{}</h4></div>
                    <div class="flex items-center gap-1">
                        <span class="flex items-center gap-1 rounded-full bg-success-50 px-2 py-0.5 text-theme-xs font-medium text-success-600 dark:bg-success-500/15 dark:text-success-500">
                            <i class="fas fa-file-alt text-2xl"></i>
                        </span>
                        <span class="text-theme-xs text-gray-500 dark:text-gray-400">{} urgent cases</span>
                    </div>
                </div>
            </div>
            <div class="rounded-2xl border border-gray-200 bg-white p-5 dark:border-gray-800 dark:bg-white/[0.03]">
                <p class="text-theme-sm text-gray-500 dark:text-gray-400">Reported Cases</p>
                <div class="mt-3 flex items-end justify-between">
                    <div><h4 class="text-2xl font-bold text-gray-800 dark:text-white/90">{}</h4></div>
                    <div class="flex items-center gap-1">
                        <span class="flex items-center gap-1 rounded-full bg-success-50 px-2 py-0.5 text-theme-xs font-medium text-success-600 dark:bg-success-500/15 dark:text-success-500">
                            <i class="fas fa-shield-alt text-2xl"></i>
                        </span>
                        <span class="text-theme-xs text-gray-500 dark:text-gray-400">{} needs attention</span>
                    </div>
                </div>
            </div>
            <div class="rounded-2xl border border-gray-200 bg-white p-5 dark:border-gray-800 dark:bg-white/[0.03]">
                <p class="text-theme-sm text-gray-500 dark:text-gray-400">Wallet Status</p>
                <div class="mt-3 flex items-end justify-between">
                    <div><h4 class="text-2xl font-bold text-gray-800 dark:text-white/90">{}</h4></div>
                    <div class="flex items-center gap-1">
                        <span class="flex items-center gap-1 rounded-full bg-error-50 px-2 py-0.5 text-theme-xs font-medium text-error-600 dark:bg-error-500/15 dark:text-error-500">
                            <i class="fas fa-wallet text-2xl"></i>
                        </span>
                        <span class="text-theme-xs text-gray-500 dark:text-gray-400">{} Connected</span>
                    </div>
                </div>
            </div>
            <div class="rounded-2xl border border-gray-200 bg-white p-5 dark:border-gray-800 dark:bg-white/[0.03]">
                <p class="text-theme-sm text-gray-500 dark:text-gray-400">Account Age</p>
                <div class="mt-3 flex items-end justify-between">
                    <div><h4 class="text-2xl font-bold text-gray-800 dark:text-white/90">{}d</h4></div>
                    <div class="flex items-center gap-1">
                        <span class="flex items-center gap-1 rounded-full bg-success-50 px-2 py-0.5 text-theme-xs font-medium text-success-600 dark:bg-success-500/15 dark:text-success-500">
                            <i class="fas fa-calendar-alt text-2xl"></i>
                        </span>
                        <span class="text-theme-xs text-gray-500 dark:text-gray-400">Active member</span>
                    </div>
                </div>
            </div>
        </div>
    "#,
    stats.total_evidence,
    stats.urgent_count,
    stats.reported_count,
    stats.needs_attention_count,
    if wallet_connections.is_empty() { "0" } else { "✓" },
    wallet_connections.len(),
    chrono::Utc::now().signed_duration_since(
        chrono::DateTime::<chrono::Utc>::from_timestamp(user.created_at as i64, 0)
            .unwrap_or_else(chrono::Utc::now)
    ).num_days()
    );


    // In your user_profile_page function, add this after getting user data:

// Generate the profile HTML directly in Rust
let profile_html = format!(r#"
    <div class="mt-4 mb-2 rounded-2xl border border-gray-200 p-5 lg:p-6 dark:border-gray-800">
        <div class="flex flex-col gap-6 lg:flex-row lg:items-start lg:justify-between">
            <div>
                <h4 class="text-lg font-semibold text-gray-800 lg:mb-6 dark:text-white/90">
                    Personal Information
                </h4>

                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2 lg:gap-7 2xl:gap-x-32">
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Email Address
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                        <p class="mt-1 text-xs text-gray-400 dark:text-gray-500">Email cannot be changed</p>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Phone Number
                        </p>
                        <div class="relative">
                            <input type="tel" name="phone_number" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="+254 7XX XXX XXX">
                        </div>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            County
                        </p>
                        <div class="relative">
                            <select name="county"
                                    class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 appearance-none dark:text-white/90">
                                <option value="">Select County</option>
                                <option value="Nairobi" {}>Nairobi</option>
                                <option value="Mombasa" {}>Mombasa</option>
                                <option value="Kisumu" {}>Kisumu</option>
                                <option value="Nakuru" {}>Nakuru</option>
                            </select>
                            <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                                <svg class="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                                    <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z"/>
                                </svg>
                            </div>
                        </div>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            ID Number
                        </p>
                        <div class="relative">
                            <input type="text" name="id_number" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="National ID or Passport">
                        </div>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Business/Organization
                        </p>
                        <div class="relative">
                            <input type="text" name="business_name" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="Optional">
                        </div>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Account Type
                        </p>
                        <div class="relative">
                            <select name="account_type"
                                    class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 appearance-none dark:text-white/90">
                                <option value="individual" {}>Individual</option>
                                <option value="business" {}>Business</option>
                                <option value="organization" {}>Organization</option>
                                <option value="government" {}>Government Agency</option>
                            </select>
                            <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                                <svg class="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                                    <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z"/>
                                </svg>
                            </div>
                        </div>
                    </div>
                </div>
            </div>

            <div class="flex flex-col gap-3 lg:flex-col lg:items-end">
                <button onclick="saveProfile()"
                        class="shadow-theme-xs flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 bg-white px-4 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 lg:inline-flex lg:w-auto dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
                    <svg class="fill-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M15.0911 2.78206C14.2125 1.90338 12.7878 1.90338 11.9092 2.78206L4.57524 10.116C4.26682 10.4244 4.0547 10.8158 3.96468 11.2426L3.31231 14.3352C3.25997 14.5833 3.33653 14.841 3.51583 15.0203C3.69512 15.1996 3.95286 15.2761 4.20096 15.2238L7.29355 14.5714C7.72031 14.4814 8.11172 14.2693 8.42013 13.9609L15.7541 6.62695C16.6327 5.74827 16.6327 4.32365 15.7541 3.44497L15.0911 2.78206ZM12.9698 3.84272C13.2627 3.54982 13.7376 3.54982 14.0305 3.84272L14.6934 4.50563C14.9863 4.79852 14.9863 5.2734 14.6934 5.56629L14.044 6.21573L12.3204 4.49215L12.9698 3.84272ZM11.2597 5.55281L5.6359 11.1766C5.53309 11.2794 5.46238 11.4099 5.43238 11.5522L5.01758 13.5185L6.98394 13.1037C7.1262 13.0737 7.25666 13.003 7.35947 12.9002L12.9833 7.27639L11.2597 5.55281Z" fill=""></path>
                    </svg>
                    Save Changes
                </button>
                
                <button type="button" onclick="resetForm()"
                        class="shadow-theme-xs flex w-full items-center justify-center gap-2 rounded-full border border-gray-300 bg-white px-4 py-3 text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-800 lg:inline-flex lg:w-auto dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] dark:hover:text-gray-200">
                    <svg class="fill-current" width="18" height="18" viewBox="0 0 18 18" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path fill-rule="evenodd" clip-rule="evenodd" d="M9 2.25C5.27208 2.25 2.25 5.27208 2.25 9C2.25 12.7279 5.27208 15.75 9 15.75C12.7279 15.75 15.75 12.7279 15.75 9C15.75 5.27208 12.7279 2.25 9 2.25ZM4.5 9C4.5 6.51472 6.51472 4.5 9 4.5C10.2597 4.5 11.393 5.03233 12.1893 5.875L10.5 7.5H13.5V4.5L12.0429 5.95711C11.1202 5.01088 9.87022 4.5 8.5 4.5C6.01472 4.5 4 6.51472 4 9C4 11.4853 6.01472 13.5 8.5 13.5C10.9853 13.5 13 11.4853 13 9H15C15 12.3137 12.3137 15 9 15C5.68629 15 3 12.3137 3 9C3 5.68629 5.68629 3 9 3C12.3137 3 15 5.68629 15 9V9.75H13.5V9C13.5 6.51472 11.4853 4.5 9 4.5Z" fill=""></path>
                    </svg>
                    Reset
                </button>
            </div>
        </div>
    </div>

    <form id="profileForm" method="POST" action="/api/user/profile/update" class="hidden">
        <input type="hidden" name="phone_number" id="hiddenPhone" value="{}">
        <input type="hidden" name="county" id="hiddenCounty" value="{}">
        <input type="hidden" name="id_number" id="hiddenId" value="{}">
        <input type="hidden" name="business_name" id="hiddenBusiness" value="{}">
        <input type="hidden" name="account_type" id="hiddenAccountType" value="{}">
    </form>
"#,
    // Email
    html_escape(&email),
    // Phone number
    html_escape(user.phone_number.as_deref().unwrap_or("")),
    // County options
    if user.county.as_deref() == Some("Nairobi") { "selected" } else { "" },
    if user.county.as_deref() == Some("Mombasa") { "selected" } else { "" },
    if user.county.as_deref() == Some("Kisumu") { "selected" } else { "" },
    if user.county.as_deref() == Some("Nakuru") { "selected" } else { "" },
    // ID number
    html_escape(user.id_number.as_deref().unwrap_or("")),
    // Business name
    html_escape(user.business_name.as_deref().unwrap_or("")),
    // Account type options
    if user.account_type.as_deref() == Some("individual") || user.account_type.as_deref() == Some("Individual") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("business") || user.account_type.as_deref() == Some("Business") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("organization") || user.account_type.as_deref() == Some("Organization") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("government") || user.account_type.as_deref() == Some("Government") || user.account_type.as_deref() == Some("Government Agency") { "selected" } else { "" },
    // Hidden form values
    html_escape(user.phone_number.as_deref().unwrap_or("")),
    html_escape(user.county.as_deref().unwrap_or("")),
    html_escape(user.id_number.as_deref().unwrap_or("")),
    html_escape(user.business_name.as_deref().unwrap_or("")),
    html_escape(user.account_type.as_deref().unwrap_or("individual"))
);



// Create a comprehensive profile HTML with all user data
let full_profile_html = format!(r#"
    <div class="mt-4 mb-2 rounded-2xl border border-gray-200 p-5 lg:p-6 dark:border-gray-800">
        <div class="flex flex-col gap-6 lg:flex-row lg:items-start lg:justify-between">
            <div>
                <h4 class="text-lg font-semibold text-gray-800 lg:mb-6 dark:text-white/90">
                    Complete User Profile
                </h4>

                <div class="grid grid-cols-1 gap-4 lg:grid-cols-2 lg:gap-7 2xl:gap-x-32">
                    <!-- User ID -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            User ID
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                    </div>

                    <!-- Email -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Email Address
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                        <p class="mt-1 text-xs text-gray-400 dark:text-gray-500">Email cannot be changed</p>
                    </div>

                    <!-- Phone Number -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Phone Number
                        </p>
                        <div class="relative">
                            <input type="tel" name="phone_number" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="+254 7XX XXX XXX">
                        </div>
                    </div>

                    <!-- ID Number -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            ID Number / Passport
                        </p>
                        <div class="relative">
                            <input type="text" name="id_number" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="National ID or Passport">
                        </div>
                    </div>

                    <!-- County -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            County
                        </p>
                        <div class="relative">
                            <select name="county"
                                    class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 appearance-none dark:text-white/90">
                                <option value="">Select County</option>
                                <option value="Nairobi" {}>Nairobi</option>
                                <option value="Mombasa" {}>Mombasa</option>
                                <option value="Kisumu" {}>Kisumu</option>
                                <option value="Nakuru" {}>Nakuru</option>
                            </select>
                            <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                                <svg class="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                                    <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z"/>
                                </svg>
                            </div>
                        </div>
                    </div>

                    <!-- Business/Organization -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Business/Organization
                        </p>
                        <div class="relative">
                            <input type="text" name="business_name" value="{}"
                                   class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 dark:text-white/90"
                                   placeholder="Optional">
                        </div>
                    </div>

                    <!-- Account Type -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Account Type
                        </p>
                        <div class="relative">
                            <select name="account_type"
                                    class="w-full px-4 py-2 text-sm font-medium text-gray-800 bg-transparent border-0 focus:outline-none focus:ring-0 appearance-none dark:text-white/90">
                                <option value="individual" {}>Individual</option>
                                <option value="business" {}>Business</option>
                                <option value="organization" {}>Organization</option>
                                <option value="government" {}>Government Agency</option>
                            </select>
                            <div class="pointer-events-none absolute inset-y-0 right-0 flex items-center px-2 text-gray-700 dark:text-gray-300">
                                <svg class="fill-current h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
                                    <path d="M9.293 12.95l.707.707L15.657 8l-1.414-1.414L10 10.828 5.757 6.586 4.343 8z"/>
                                </svg>
                            </div>
                        </div>
                    </div>

                    <!-- Wallet Address -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Wallet Address
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90 font-mono">
                            {}
                        </p>
                    </div>

                    <!-- Wallet Chain -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Wallet Chain
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                    </div>

                    <!-- Location Coordinates -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Location (Lat, Long)
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}, {}
                        </p>
                    </div>

                    <!-- Account Status -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Account Status
                        </p>
                        <div class="flex items-center space-x-2">
                            <span class="px-2 py-1 text-xs rounded-full {}">
                                {}
                            </span>
                            <span class="px-2 py-1 text-xs rounded-full {}">
                                Profile {}
                            </span>
                        </div>
                    </div>

                    <!-- Account Dates -->
                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Account Created
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                    </div>

                    <div>
                        <p class="mb-2 text-xs leading-normal text-gray-500 dark:text-gray-400">
                            Last Login
                        </p>
                        <p class="text-sm font-medium text-gray-800 dark:text-white/90">
                            {}
                        </p>
                    </div>
                </div>
            </div>

           <div class="flex items-center gap-3 px-2 mt-6 lg:justify-end">
        <button onclick="resetForm()" type="button" class="flex w-full justify-center rounded-lg border border-gray-300 bg-white px-4 py-2.5 text-sm font-medium text-gray-700 hover:bg-gray-50 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-white/[0.03] sm:w-auto">
          Discard
        </button>
        <button type="button"  onclick="saveProfile()" class="flex w-full justify-center rounded-lg bg-brand-500 px-4 py-2.5 text-sm font-medium text-white hover:bg-brand-600 sm:w-auto">
          Update
        </button>
      </div>
            </div>
        </div>
        
    </div>

    <form id="profileForm" method="POST" action="/api/user/profile/update" class="hidden">
        <input type="hidden" name="phone_number" id="hiddenPhone" value="{}">
        <input type="hidden" name="county" id="hiddenCounty" value="{}">
        <input type="hidden" name="id_number" id="hiddenId" value="{}">
        <input type="hidden" name="business_name" id="hiddenBusiness" value="{}">
        <input type="hidden" name="account_type" id="hiddenAccountType" value="{}">
    </form>
"#,
    // User ID
    html_escape(&user.id),
    // Email
    html_escape(&email),
    // Phone number
    html_escape(user.phone_number.as_deref().unwrap_or("")),
    // ID number
    html_escape(user.id_number.as_deref().unwrap_or("")),
    // County options
    if user.county.as_deref() == Some("Nairobi") { "selected" } else { "" },
    if user.county.as_deref() == Some("Mombasa") { "selected" } else { "" },
    if user.county.as_deref() == Some("Kisumu") { "selected" } else { "" },
    if user.county.as_deref() == Some("Nakuru") { "selected" } else { "" },
    // Business name
    html_escape(user.business_name.as_deref().unwrap_or("")),
    // Account type options
    if user.account_type.as_deref() == Some("individual") || user.account_type.as_deref() == Some("Individual") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("business") || user.account_type.as_deref() == Some("Business") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("organization") || user.account_type.as_deref() == Some("Organization") { "selected" } else { "" },
    if user.account_type.as_deref() == Some("government") || user.account_type.as_deref() == Some("Government") || user.account_type.as_deref() == Some("Government Agency") { "selected" } else { "" },
    // Wallet info
    html_escape(&wallet_connections.first().map(|w| w.wallet_address.clone()).unwrap_or_else(|| "Not connected".to_string())),
    html_escape(&wallet_connections.first().map(|w| w.chain.clone()).unwrap_or_else(|| "N/A".to_string())),
    // Location coordinates (you'll need to get these from user data)
    -1.2920659, // latitude
    36.8219462, // longitude
    // Account status badges
    if user.is_verified { "bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400" } else { "bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400" },
    if user.is_verified { "Verified" } else { "Not Verified" },
    if user.is_profile_complete { "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400" } else { "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400" },
    if user.is_profile_complete { "Complete" } else { "Incomplete" },
    // Dates
    DateTime::<Utc>::from_timestamp(user.created_at as i64, 0)
        .unwrap_or_else(Utc::now)
        .format("%B %d, %Y %H:%M").to_string(),
    DateTime::<Utc>::from_timestamp(user.created_at as i64, 0)
        .unwrap_or_else(Utc::now)
        .format("%B %d, %Y %H:%M").to_string(),

    // Hidden form values
    html_escape(user.phone_number.as_deref().unwrap_or("")),
    html_escape(user.county.as_deref().unwrap_or("")),
    html_escape(user.id_number.as_deref().unwrap_or("")),
    html_escape(user.business_name.as_deref().unwrap_or("")),
    html_escape(user.account_type.as_deref().unwrap_or("individual"))
);


    // Create context for template
    let mut context = std::collections::HashMap::new();
    
    // User Profile Data - CORRECTED to match template expectations
    context.insert("email", email.clone());
    context.insert("phone_number", user.phone_number.unwrap_or_else(|| "".to_string()));
    context.insert("county", user.county.unwrap_or_else(|| "".to_string()));
    context.insert("business_name", user.business_name.unwrap_or_else(|| "".to_string()));
    context.insert("id_number", user.id_number.unwrap_or_else(|| "".to_string()));

    // ── FIX: user_id was missing — template {{ user_id }} rendered as U+FFFD
    //         causing index.js to navigate to /user/%EF%BF%BD (404)
    context.insert("user_id", user.id.clone());

    // ── FIX: days_member / evidence_count / signed_count missing from context
    let days_member = {
        let created = chrono::DateTime::<chrono::Utc>::from_timestamp(user.created_at as i64, 0)
            .unwrap_or_else(chrono::Utc::now);
        chrono::Utc::now().signed_duration_since(created).num_days()
    };
    context.insert("days_member",    days_member.to_string());
    context.insert("evidence_count", stats.total_evidence.to_string());
    context.insert("signed_count",   stats.reported_count.to_string());

    // ── FIX: wallet fields missing — use first active connection if present
    let (wallet_address, wallet_chain, wallet_type) = wallet_connections
        .iter()
        .find(|w| w.is_active)
        .or_else(|| wallet_connections.first())
        .map(|w| (
            w.wallet_address.clone(),
            w.chain.clone(),
            w.wallet_type.clone(),
        ))
        .unwrap_or_else(|| ("".to_string(), "".to_string(), "".to_string()));

    context.insert("wallet_address", wallet_address);
    context.insert("wallet_chain",   wallet_chain);
    context.insert("wallet_type",    wallet_type);
    
    // Critical Fix: Handle account_type for template comparisons
    let account_type_val = match user.account_type.as_deref() {
        Some("individual") | Some("Individual") => "individual",
        Some("business") | Some("Business") => "business",
        Some("organization") | Some("Organization") => "organization",
        Some("government") | Some("Government") | Some("Government Agency") => "government",
        _ => "individual",
    };
    context.insert("account_type", account_type_val.to_string());
    
    // Status flags
    context.insert("is_verified", user.is_verified.to_string());
    context.insert("is_profile_complete", user.is_profile_complete.to_string());
    context.insert("wallet_connections", (!wallet_connections.is_empty()).to_string());
    context.insert("wallet_verified", (!wallet_connections.is_empty()).to_string());
    
    // Dates
    let created_at_dt = chrono::DateTime::<chrono::Utc>::from_timestamp(user.created_at as i64, 0)
        .unwrap_or_else(chrono::Utc::now);
    context.insert("created_at", created_at_dt.format("%B %d, %Y").to_string());
    context.insert("last_login", "Just now".to_string());
    
    // Statistics
    context.insert("total_evidence", stats.total_evidence.to_string());
    context.insert("reported_count", stats.reported_count.to_string());
    context.insert("urgent_count", stats.urgent_count.to_string());
    context.insert("needs_attention_count", stats.needs_attention_count.to_string());
    
    // HTML content blocks
    context.insert("stats_cards", stats_html);
    context.insert("wallets_html", wallets_html);
    context.insert("activity_items", activity_items_html);
    context.insert("activity_timeline", activity_timeline);
    // Now add this to your context
    context.insert("profile_html", profile_html);
    // Add to context
    context.insert("full_profile_html", full_profile_html);

    
    
    // Active tab
    let query_str = req.query_string();
    let active_tab = if query_str.contains("tab=wallets") {
        "wallets"
    } else if query_str.contains("tab=activity") {
        "activity"
    } else if query_str.contains("tab=security") {
        "security"
    } else {
        "profile"
    };
    context.insert("active_tab", active_tab.to_string());
    
    // Debug output
    println!("📋 Template context populated for user: {}", email);
    println!("  phone: {:?}", context.get("phone_number"));
    println!("  county: {:?}", context.get("county"));
    println!("  account_type: {:?}", context.get("account_type"));
    
    // Render template
    let html = render_template("user_profile", &context);
    HttpResponse::Ok().body(html)
}

// Helper function to format action types
fn format_action_type(action_type: &str) -> String {
    match action_type {
        "user_login" => "User Login".to_string(),
        "evidence_created" => "Evidence Created".to_string(),
        "evidence_updated" => "Evidence Updated".to_string(),
        "evidence_signed" => "Evidence Signed".to_string(),
        "evidence_reported" => "Evidence Reported".to_string(),
        "wallet_connected" => "Wallet Connected".to_string(),
        "password_changed" => "Password Changed".to_string(),
        "profile_updated" => "Profile Updated".to_string(),
        _ => {
            action_type.replace('_', " ")
                .split_whitespace()
                .map(|s| {
                    let mut chars = s.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    }
}

// Add these API endpoints to media_routes.rs
pub async fn api_update_profile(
    session: Session,
    form: web::Form<ProfileUpdateForm>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("📝 API_UPDATE_PROFILE: Updating user profile");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.get_user_by_email(&email).await {
        Ok(Some(mut user)) => {
            let form_data = form.into_inner();
            
            // Update all fields that are provided
            if let Some(phone) = form_data.phone_number {
                if !phone.is_empty() {
                    user.phone_number = Some(phone);
                }
            }
            
            if let Some(county) = form_data.county {
                if !county.is_empty() {
                    user.county = Some(county);
                }
            }
            
            if let Some(business_name) = form_data.business_name {
                if !business_name.is_empty() {
                    user.business_name = Some(business_name);
                }
            }
            
            if let Some(account_type) = form_data.account_type {
                if !account_type.is_empty() {
                    user.account_type = Some(account_type);
                }
            }
            
            if let Some(id_number) = form_data.id_number {
                if !id_number.is_empty() {
                    user.id_number = Some(id_number);
                }
            }
            
            // Validate required fields
            if user.phone_number.is_none() || user.phone_number.as_ref().unwrap().is_empty() {
                return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Phone number is required"));
            }
            
            if user.county.is_none() || user.county.as_ref().unwrap().is_empty() {
                return HttpResponse::BadRequest().json(ApiResponse::<()>::error("County is required"));
            }
            
            user.is_profile_complete = true;
            user.updated_at = Utc::now().timestamp() as u64;
            
            match auth_service.update_user(&user).await {
                Ok(_) => {
                    // Log the profile update
                    // To this:
                    auth_service.log_audit(
                        Some(&user.id),
                        "profile_updated",
                        "user",
                        Some(&user.id),
                        "User profile updated",
                        None,
                    ).await.ok();
                    println!("✅ Profile updated successfully for user: {}", email);
                    
                    HttpResponse::Ok().json(ApiResponse::success(json!({
                        "success": true,
                        "message": "Profile updated successfully",
                        "user": {
                            "email": user.email,
                            "phone_number": user.phone_number.unwrap_or_default(),
                            "county": user.county.unwrap_or_default(),
                            "business_name": user.business_name.unwrap_or_default(),
                            "account_type": user.account_type.unwrap_or_default(),
                            "id_number": user.id_number.unwrap_or_default(),
                            "is_profile_complete": user.is_profile_complete,
                        }
                    })))
                }
                Err(e) => {
                    println!("❌ Error updating profile: {}", e);
                    HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to update profile: {}", e)))
                }
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(ApiResponse::<()>::error("User not found")),
        Err(e) => {
            println!("❌ Error getting user: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("Internal server error"))
        }
    }
}

pub async fn api_wallet_disconnect(
    session: Session,
    form: web::Form<WalletDisconnectForm>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🔗 API_WALLET_DISCONNECT: Disconnecting wallet");
    
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    let _form_data = form.into_inner();
    
    match auth_service.disconnect_wallet(&user_id).await {
        Ok(_) => {
            // Clear wallet session data
            session.remove("wallet_address");
            session.remove("wallet_chain");
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "success": true,
                "message": "Wallet disconnected successfully",
                "redirect": "/user/profile?tab=wallets"
            })))
        }
        Err(e) => {
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed: {}", e)))
        }
    }
}


// Add to media_routes.rs, after the existing API functions

pub async fn api_change_password(
    session: Session,
    form: web::Form<PasswordChangeForm>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🔐 API_CHANGE_PASSWORD: Changing password");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    let form_data = form.into_inner();
    
    // Validate passwords match
    if form_data.new_password != form_data.confirm_password {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Passwords do not match"));
    }
    
    // Validate password strength
    if form_data.new_password.len() < 8 {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("Password must be at least 8 characters"));
    }
    
    match auth_service.change_password(&email, &form_data.current_password, &form_data.new_password).await {
        Ok(_) => {
            println!("✅ Password changed successfully for user: {}", email);
            
            // Log the user out of all sessions
            let _ = session.clear();
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "success": true,
                "message": "Password changed successfully. Please log in again.",
                "redirect": "/login"
            })))
        }
        Err(e) => {
            println!("❌ Error changing password: {}", e);
            let error_msg = if e.to_string().contains("Current password is incorrect") {
                "Current password is incorrect"
            } else {
                &format!("Failed to change password: {}", e)
            };
            HttpResponse::BadRequest().json(ApiResponse::<()>::error(error_msg))
        }
    }
}

pub async fn api_get_user_activity(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("📊 API_GET_USER_ACTIVITY: Getting user activity logs");
    
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(user_id) => user_id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.get_user_activity_logs(&user_id, 50).await {
        Ok(logs) => {
            println!("✅ Found {} activity logs", logs.len());
            
            let activity_data: Vec<serde_json::Value> = logs.iter().map(|log| {
                let action_icon = match log.action_type.as_str() {
                    "user_login" => "fas fa-sign-in-alt",
                    "evidence_created" => "fas fa-file-upload",
                    "evidence_updated" => "fas fa-edit",
                    "evidence_signed" => "fas fa-signature",
                    "evidence_reported" => "fas fa-shield-alt",
                    "wallet_connected" => "fas fa-wallet",
                    "password_changed" => "fas fa-key",
                    "profile_updated" => "fas fa-user-edit",
                    _ => "fas fa-history",
                };
                
                json!({
                    "id": log.id,
                    "action_type": log.action_type,
                    "action_icon": action_icon,
                    "details": log.details,
                    "created_at": log.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "time_ago": format_time_ago(log.created_at),
                    "ip_address": log.ip_address,
                    "user_agent": log.user_agent,
                })
            }).collect();
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "activity": activity_data,
                "total": logs.len(),
            })))
        }
        Err(e) => {
            println!("❌ Error getting activity logs: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get activity logs: {}", e)))
        }
    }
}

/// GET /api/user/audit-logs
///
/// Returns the current user's audit-log / activity history as a flat JSON array.
/// Response shape is intentionally kept unwrapped so the Alpine frontend can
/// iterate it directly:  { "logs": [...], "total": N }
/// Each entry includes both "action" (normalised) and "action_type" (raw) so
/// that older JS code referencing either field continues to work.
pub async fn api_get_user_audit_logs(
    session: Session,
    auth_service: web::Data<AuthService>,
    query: web::Query<HashMap<String, String>>,
) -> HttpResponse {
    println!("📋 API_GET_USER_AUDIT_LOGS: Getting audit logs");

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(json!({ "success": false, "message": "Not authenticated" }));
        }
    };

    let limit: i64 = query.get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    match auth_service.get_user_activity_logs(&user_id, limit as u32).await {
        Ok(logs) => {
            println!("✅ Found {} audit log entries", logs.len());

            let entries: Vec<serde_json::Value> = logs.iter().map(|log| {
                // Normalise action_type to a human-readable label for the JS
                // _normaliseAuditLog() uses "action" key – expose both.
                let action = log.action_type.as_str();
                let label = match action {
                    "evidence_upload" | "evidence_created" => "Evidence Uploaded",
                    "evidence_submitted"                   => "Evidence Submitted",
                    "evidence_updated"                     => "Evidence Updated",
                    "evidence_deleted"                     => "Evidence Deleted",
                    "evidence_signed"                      => "Evidence Signed On-chain",
                    "evidence_reported"                    => "Reported to Police",
                    "evidence_exported"                    => "Evidence Exported",
                    "evidence_viewed"                      => "Evidence Viewed",
                    "profile_updated"                      => "Profile Updated",
                    "password_changed"                     => "Password Changed",
                    "wallet_connected"                     => "Wallet Connected",
                    "wallet_disconnected"                  => "Wallet Disconnected",
                    "user_login"  | "login"                => "Signed In",
                    "user_logout" | "logout"               => "Signed Out",
                    "poi_pinned"                           => "POI Pinned",
                    "target_created"                       => "Target Profile Created",
                    "report_generated"                     => "Police Report Generated",
                    _                                      => action,
                };

                json!({
                    // Both keys so JS normalizer always finds one
                    "action":      action,
                    "action_type": action,
                    // Human-readable title (JS uses this as .title fallback)
                    "title":       label,
                    "description": label,
                    "id":          log.id,
                    "entity_id":   log.id,
                    "details":     log.details,
                    "created_at":  log.created_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                    "time_ago":    format_time_ago(log.created_at),
                    "ip_address":  log.ip_address,
                })
            }).collect();

            // Return as a plain object — the JS checks: Array.isArray(data) ? data
            //   : (data.logs || data.activity || data.data || [])
            HttpResponse::Ok().json(json!({
                "success": true,
                "logs":    entries,
                "total":   logs.len(),
            }))
        }
        Err(e) => {
            println!("❌ Error getting audit logs: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": format!("Failed to get audit logs: {}", e),
            }))
        }
    }
}

pub async fn api_get_wallet_connections(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("💰 API_GET_WALLET_CONNECTIONS: Getting wallet connections");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.get_wallet_connections(&email).await {
        Ok(wallets) => {
            println!("✅ Found {} wallet connections", wallets.len());
            
            let wallet_data: Vec<serde_json::Value> = wallets.iter().map(|wallet| {
                let chain_badge = match wallet.chain.as_str() {
                    "ethereum" => "bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-400",
                    "base" => "bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-400",
                    "avalanche" => "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
                    "stellar" => "bg-black text-white dark:bg-gray-800 dark:text-gray-100",
                    _ => "bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400",
                };
                
                json!({
                    "wallet_address": wallet.wallet_address,
                    "address_short": format!("{}...{}", 
                        &wallet.wallet_address[0..6], 
                        &wallet.wallet_address[wallet.wallet_address.len()-4..]),
                    "chain": wallet.chain,
                    "chain_badge": chain_badge,
                    "wallet_type": wallet.wallet_type,
                    "connected_at": wallet.connected_at.format("%Y-%m-%d %H:%M").to_string(),
                    "last_used": wallet.last_used.format("%Y-%m-%d %H:%M").to_string(),
                    "is_active": wallet.is_active,
                    "days_connected": Utc::now().signed_duration_since(wallet.connected_at).num_days(),
                })
            }).collect();
            
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "wallets": wallet_data,
                "total": wallets.len(),
                "has_wallet": !wallets.is_empty(),
            })))
        }
        Err(e) => {
            println!("❌ Error getting wallet connections: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get wallet connections: {}", e)))
        }
    }
}

// Helper function to format time ago
fn format_time_ago(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);
    
    if duration.num_seconds() < 60 {
        format!("{} seconds ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{} minutes ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_weeks() < 4 {
        format!("{} weeks ago", duration.num_weeks())
    } else {
        format!("{} months ago", duration.num_days() / 30)
    }
}


// ==================== NOTIFICATIONS & LINKED EVIDENCE API ====================

/// Get user notifications
pub async fn api_get_notifications(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🔔 API_GET_NOTIFICATIONS: Getting user notifications");
    
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.get_user_notifications(&user_id, false).await {
        Ok(notifications) => {
            println!("✅ Found {} unread notifications", notifications.len());
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "notifications": notifications,
                "count": notifications.len(),
            })))
        }
        Err(e) => {
            println!("❌ Error getting notifications: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get notifications: {}", e)))
        }
    }
}

/// Get linked evidence for a specific evidence ID
pub async fn api_get_linked_evidence(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let evidence_id = path.into_inner();
    println!("🔗 API_GET_LINKED_EVIDENCE: Getting linked cases for evidence: {}", evidence_id);
    
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    // Verify user owns this evidence or is admin
    match evidence_service.database.get_evidence(&evidence_id, false).await {
        Ok(Some(evidence)) => {
            if evidence.uploader_id != user_id {
                return HttpResponse::Forbidden().json(ApiResponse::<()>::error("You don't have permission to view linked evidence for this case"));
            }
        }
        Ok(None) => {
            return HttpResponse::NotFound().json(ApiResponse::<()>::error("Evidence not found"));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Error: {}", e)));
        }
    }
    
    match evidence_service.database.get_linked_evidence(&evidence_id).await {
        Ok(linked_cases) => {
            println!("✅ Found {} linked cases", linked_cases.len());
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "linked_cases": linked_cases,
                "count": linked_cases.len(),
            })))
        }
        Err(e) => {
            println!("❌ Error getting linked evidence: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get linked evidence: {}", e)))
        }
    }
}

/// Mark notification as read
pub async fn api_mark_notification_read(
    session: Session,
    path: web::Path<String>,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let notification_id = path.into_inner();
    println!("✅ API_MARK_NOTIFICATION_READ: Marking notification as read: {}", notification_id);
    
    let _user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.mark_notification_read(&notification_id).await {
        Ok(_) => {
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "message": "Notification marked as read",
            })))
        }
        Err(e) => {
            println!("❌ Error marking notification as read: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to mark notification as read: {}", e)))
        }
    }
}

/// POST /api/user/notifications/read-all
///
/// Marks every unread notification for the current user as read.
/// Fetches all unread notifications then marks each one individually
/// (reuses the existing mark_notification_read service call).
pub async fn api_mark_all_notifications_read(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("✅ API_MARK_ALL_NOTIFICATIONS_READ: Marking all notifications as read");

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    // Fetch unread notifications for this user, then mark each read
    match auth_service.get_user_notifications(&user_id, false).await {
        Ok(notifications) => {
            let mut marked = 0usize;
            for n in &notifications {
                // Best-effort: ignore individual failures
                let _ = auth_service.mark_notification_read(&n.id).await;
                marked += 1;
            }
            println!("✅ Marked {} notifications as read", marked);
            HttpResponse::Ok().json(json!({
                "success": true,
                "marked":  marked,
                "message": format!("{} notification(s) marked as read", marked),
            }))
        }
        Err(e) => {
            println!("❌ Error fetching notifications for mark-all: {}", e);
            HttpResponse::InternalServerError().json(json!({
                "success": false,
                "message": format!("Failed to mark notifications as read: {}", e),
            }))
        }
    }
}

/// Get unread notification count
pub async fn api_get_notification_count(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    match auth_service.get_unread_notification_count(&user_id).await {
        Ok(count) => {
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "count": count,
            })))
        }
        Err(e) => {
            println!("❌ Error getting notification count: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("Failed to get notification count: {}", e)))
        }
    }
}


/// Linked cases page - shows all evidence linked by target matching
pub async fn linked_cases_page(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🔗 LINKED_CASES_PAGE: Loading linked cases page");
    
    // Check authentication
    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    };
    
    // Check if profile is complete
    match auth_service.get_user_by_email(&email).await {
        Ok(Some(user)) => {
            if !user.is_profile_complete {
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/profile/complete"))
                    .finish();
            }
        }
        _ => {
            let _ = session.clear();
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    }
    
    // Load the HTML template
    match std::fs::read_to_string("static/templates/linked_cases.html") {
        Ok(content) => {
            HttpResponse::Ok()
                .content_type("text/html; charset=utf-8")
                .body(content)
        }
        Err(e) => {
            println!("❌ Error loading template: {}", e);
            HttpResponse::InternalServerError().body("Template not found")
        }
    }
}


// ==================== API HANDLER ====================

/// GET /api/evidence/stats — dashboard statistics widget
pub async fn api_get_evidence_stats(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("📊 API_GET_EVIDENCE_STATS: Fetching dashboard stats");

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    let stats = evidence_service
        .get_dashboard_stats(&user_id)
        .await
        .unwrap_or(DashboardStats {
            total_evidence: 0,
            urgent_count: 0,
            reported_count: 0,
            needs_attention_count: 0,
            today_count: 0,
            by_county: Vec::new(),
            by_type: Vec::new(),
        });

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "total_evidence":        stats.total_evidence,
        "urgent_count":          stats.urgent_count,
        "reported_count":        stats.reported_count,
        "needs_attention_count": stats.needs_attention_count,
        "today_count":           stats.today_count,
    })))
}

/// API endpoint to get all linked cases for the current user
pub async fn api_get_linked_cases(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("🔗 API_GET_LINKED_CASES: Fetching linked cases");
    
    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Not authenticated"));
        }
    };
    
    println!("🔗 API_GET_LINKED_CASES: User ID: {}", user_id);
    
    // Get all evidence for this user
    let user_evidence = match evidence_service.get_user_evidence(&user_id).await {
        Ok(evidence) => evidence,
        Err(e) => {
            println!("❌ Error getting user evidence: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to get evidence: {}", e)));
        }
    };
    
    println!("🔗 Found {} evidence records for user", user_evidence.len());
    
    // For each evidence, get linked cases
    let mut all_linked_cases = Vec::new();
    let mut processed_pairs = std::collections::HashSet::new();
    
    for evidence in user_evidence {
        // Get linked evidence records from database
        match evidence_service.database.get_linked_evidence(&evidence.id).await {
            Ok(linked_records) => {
                println!("🔗 Evidence {} has {} links", evidence.evidence_number, linked_records.len());
                
                for link in linked_records {
                    // Create a canonical pair ID to avoid duplicates
                    let pair_id = if evidence.id < link.evidence_id_2 {
                        format!("{}_{}", evidence.id, link.evidence_id_2)
                    } else {
                        format!("{}_{}", link.evidence_id_2, evidence.id)
                    };
                    
                    // Skip if we've already processed this pair
                    if processed_pairs.contains(&pair_id) {
                        continue;
                    }
                    processed_pairs.insert(pair_id);
                    
                    // Get full details of the other evidence
                    let other_evidence_id = if link.evidence_id_1 == evidence.id {
                        &link.evidence_id_2
                    } else {
                        &link.evidence_id_1
                    };
                    
                    match evidence_service.get_evidence(other_evidence_id, false).await {
                        Ok(Some(other_evidence)) => {
                            // Get current evidence details
                            match evidence_service.get_evidence(&evidence.id, false).await {
                                Ok(Some(current_evidence)) => {
                                    // Format the linked case data
                                    all_linked_cases.push(json!({
                                        "link_id": link.link_id,
                                        "link_type": link.link_type,
                                        "link_reason": link.link_reason,
                                        "matched_target_hash": link.matched_target_hash,
                                        "confidence_score": link.confidence_score,
                                        "created_at": link.created_at.to_rfc3339(),
                                        "my_case": {
                                            "id": current_evidence.id,
                                            "evidence_number": current_evidence.evidence_number,
                                            "title": current_evidence.title,
                                            "description": current_evidence.description,
                                            "emergency_level": format!("{:?}", current_evidence.emergency_level),
                                            "incident_type": format!("{:?}", current_evidence.incident_type),
                                            "incident_time": current_evidence.incident_time.to_rfc3339(),
                                            "location": {
                                                "county": current_evidence.location.county,
                                                "latitude": current_evidence.location.latitude,
                                                "longitude": current_evidence.location.longitude,
                                            },
                                            "uploader_email": current_evidence.uploader_email,
                                            "status": format!("{:?}", current_evidence.status),
                                            "media_files": current_evidence.media_files.iter().map(|m| json!({
                                                "filename": m.filename,
                                                "mime_type": m.mime_type,
                                                "storj_url": m.storj_url,
                                                "file_size": m.file_size,
                                            })).collect::<Vec<_>>(),
                                        },
                                        "linked_case": {
                                            "id": other_evidence.id,
                                            "evidence_number": other_evidence.evidence_number,
                                            "title": other_evidence.title,
                                            "description": other_evidence.description,
                                            "emergency_level": format!("{:?}", other_evidence.emergency_level),
                                            "incident_type": format!("{:?}", other_evidence.incident_type),
                                            "incident_time": other_evidence.incident_time.to_rfc3339(),
                                            "location": {
                                                "county": other_evidence.location.county,
                                                "latitude": other_evidence.location.latitude,
                                                "longitude": other_evidence.location.longitude,
                                            },
                                            "uploader_email": other_evidence.uploader_email,
                                            "status": format!("{:?}", other_evidence.status),
                                            "media_files": other_evidence.media_files.iter().map(|m| json!({
                                                "filename": m.filename,
                                                "mime_type": m.mime_type,
                                                "storj_url": m.storj_url,
                                                "file_size": m.file_size,
                                            })).collect::<Vec<_>>(),
                                        },
                                    }));
                                }
                                Err(e) => {
                                    println!("❌ Error getting current evidence {}: {}", evidence.id, e);
                                }
                                Ok(None) => {
                                    println!("⚠️ Current evidence not found: {}", evidence.id);
                                }
                            }
                        }
                        Err(e) => {
                            println!("❌ Error getting other evidence {}: {}", other_evidence_id, e);
                        }
                        Ok(None) => {
                            println!("⚠️ Other evidence not found: {}", other_evidence_id);
                        }
                    }
                }
            }
            Err(e) => {
                println!("⚠️ Error getting linked evidence for {}: {}", evidence.id, e);
                continue;
            }
        }
    }
    
    println!("✅ API_GET_LINKED_CASES: Returning {} linked cases", all_linked_cases.len());
    
    // If no linked cases found, return empty array
    if all_linked_cases.is_empty() {
        return HttpResponse::Ok().json(ApiResponse::success(json!({
            "linked_cases": [],
            "total": 0,
            "message": "No linked cases found"
        })));
    }
    
    HttpResponse::Ok().json(ApiResponse::success(json!({
        "linked_cases": all_linked_cases,
        "total": all_linked_cases.len(),
    })))
}

// ==================== CREATE LINKED CASE ====================

#[derive(serde::Deserialize)]
pub struct CreateLinkPayload {
    pub evidence_id_1: String,
    pub evidence_id_2: String,
    pub link_type: Option<String>,
    pub link_reason: Option<String>,
    pub confidence_score: Option<i32>,
}

/// POST /api/linked-cases/create
/// Manually link two crime scenes together
pub async fn api_create_linked_case(
    session: Session,
    evidence_service: web::Data<EvidenceService>,
    body: web::Json<CreateLinkPayload>,
) -> HttpResponse {
    println!("🔗 API_CREATE_LINKED_CASE: Creating manual link");

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    let id1 = body.evidence_id_1.trim().to_string();
    let id2 = body.evidence_id_2.trim().to_string();

    // Prevent self-linking
    if id1 == id2 {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("Cannot link an evidence record to itself"));
    }

    // Fetch both evidence records
    let ev1 = match evidence_service.get_evidence(&id1, false).await {
        Ok(Some(e)) => e,
        Ok(None) => return HttpResponse::NotFound().json(ApiResponse::<()>::error("Evidence 1 not found")),
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };
    let ev2 = match evidence_service.get_evidence(&id2, false).await {
        Ok(Some(e)) => e,
        Ok(None) => return HttpResponse::NotFound().json(ApiResponse::<()>::error("Evidence 2 not found")),
        Err(e) => return HttpResponse::InternalServerError().json(ApiResponse::<()>::error(&format!("DB error: {}", e))),
    };

    // Auth check: user must own at least one side
    if ev1.uploader_id != user_id && ev2.uploader_id != user_id {
        return HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("You must own at least one of the evidence records to create a link"));
    }

    // Check for duplicate link
    let existing = evidence_service.database.get_linked_evidence(&id1).await.unwrap_or_default();
    let already_linked = existing.iter().any(|lnk| {
        (lnk.evidence_id_1 == id1 && lnk.evidence_id_2 == id2)
            || (lnk.evidence_id_1 == id2 && lnk.evidence_id_2 == id1)
    });
    if already_linked {
        return HttpResponse::Conflict()
            .json(ApiResponse::<()>::error("These two cases are already linked"));
    }

    // Insert using the rusqlite pool
    let link_type   = body.link_type.clone().unwrap_or_else(|| "manual".to_string());
    let link_reason = body.link_reason.clone().unwrap_or_default();
    let confidence  = body.confidence_score.unwrap_or(50).max(0).min(100);
    let link_id     = uuid::Uuid::new_v4().to_string();
    let now_str     = chrono::Utc::now().to_rfc3339();

    let conn = match evidence_service.database.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("DB pool error: {}", e)));
        }
    };

    let insert_result = conn.execute(
        "INSERT INTO linked_cases (id, evidence_id_1, evidence_id_2, link_type, link_reason, confidence_score, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![link_id, id1, id2, link_type, link_reason, confidence, now_str],
    );

    match insert_result {
        Ok(_) => {
            println!("✅ Created link {} between {} and {}", link_id, id1, id2);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "link_id": link_id,
                "message": "Cases linked successfully",
                "my_case": {
                    "id": ev1.id,
                    "evidence_number": ev1.evidence_number,
                    "title": ev1.title,
                },
                "linked_case": {
                    "id": ev2.id,
                    "evidence_number": ev2.evidence_number,
                    "title": ev2.title,
                },
                "link_type": link_type,
                "link_reason": link_reason,
                "confidence_score": confidence,
                "created_at": now_str,
            })))
        }
        Err(e) => {
            println!("❌ Error creating link: {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to create link: {}", e)))
        }
    }
}

// ==================== DELETE LINKED CASE ====================

/// DELETE /api/linked-cases/{id}
/// Remove a link between two crime scenes (must own at least one side)
pub async fn api_delete_linked_case(
    session: Session,
    path: web::Path<String>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let link_id = path.into_inner();
    println!("🗑️ API_DELETE_LINKED_CASE: Deleting link {}", link_id);

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    // Fetch the link record to verify ownership
    let conn = match evidence_service.database.pool.get() {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("DB pool error: {}", e)));
        }
    };

    let row: rusqlite::Result<(String, String)> = conn.query_row(
        "SELECT evidence_id_1, evidence_id_2 FROM linked_cases WHERE id = ?1",
        rusqlite::params![link_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    );

    let (ev_id1, ev_id2) = match row {
        Ok(r) => r,
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            return HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("Link not found"));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("DB error: {}", e)));
        }
    };

    // Check ownership: user must own at least one side of the link
    let ev1_opt = evidence_service.database.get_evidence(&ev_id1, false).await.unwrap_or(None);
    let ev2_opt = evidence_service.database.get_evidence(&ev_id2, false).await.unwrap_or(None);

    let owns_ev1 = ev1_opt.map(|e| e.uploader_id == user_id).unwrap_or(false);
    let owns_ev2 = ev2_opt.map(|e| e.uploader_id == user_id).unwrap_or(false);

    if !owns_ev1 && !owns_ev2 {
        return HttpResponse::Forbidden()
            .json(ApiResponse::<()>::error("You don't have permission to delete this link"));
    }

    // Delete the link
    let rows_affected = conn.execute(
        "DELETE FROM linked_cases WHERE id = ?1",
        rusqlite::params![link_id],
    );

    match rows_affected {
        Ok(n) if n > 0 => {
            println!("✅ Deleted link {}", link_id);
            HttpResponse::Ok().json(ApiResponse::success(json!({
                "message": "Link deleted successfully",
                "link_id": link_id,
            })))
        }
        Ok(_) => {
            HttpResponse::NotFound()
                .json(ApiResponse::<()>::error("Link not found or already deleted"))
        }
        Err(e) => {
            println!("❌ Error deleting link: {}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to delete link: {}", e)))
        }
    }
}

pub async fn notifications_page(
    session: Session,
    auth_service: web::Data<AuthService>,
) -> HttpResponse {
    println!("🔔 NOTIFICATIONS_PAGE: Loading notifications page");

    let email = match session.get::<String>("user_email").unwrap_or(None) {
        Some(email) => email,
        None => {
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    };

    match auth_service.get_user_by_email(&email).await {
        Ok(Some(user)) => {
            if !user.is_profile_complete {
                return HttpResponse::SeeOther()
                    .append_header(("Location", "/profile/complete"))
                    .finish();
            }
        }
        _ => {
            let _ = session.clear();
            return HttpResponse::SeeOther()
                .append_header(("Location", "/login"))
                .finish();
        }
    }

    match std::fs::read_to_string("static/templates/notifications.html") {
        Ok(content) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(content),
        Err(e) => {
            println!("❌ Error loading template: {}", e);
            HttpResponse::InternalServerError().body("Template not found")
        }
    }
}


/// Get ALL notifications for the notifications page (read + unread),
/// enriched with related evidence data where available.
pub async fn api_get_all_notifications(
    session: Session,
    auth_service: web::Data<AuthService>,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    println!("🔔 API_GET_ALL_NOTIFICATIONS: Getting all notifications (enriched)");

    let user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    let notifications = match auth_service.get_user_notifications(&user_id, true).await {
        Ok(n) => n,
        Err(e) => {
            println!("❌ Error getting notifications: {}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error(&format!("Failed to get notifications: {}", e)));
        }
    };

    println!("✅ Found {} total notifications – enriching…", notifications.len());

    // ── Infer notification type from title/message keywords ──────────────────
    fn infer_type(title: &str, message: &str) -> &'static str {
        let s = format!("{} {}", title, message).to_lowercase();
        if s.contains("upload") || s.contains("media") || s.contains("file") {
            "media_upload"
        } else if s.contains("police") {
            "police_report"
        } else if s.contains("sign") {
            "evidence_signed"
        } else if s.contains("link") {
            "evidence_linked"
        } else if s.contains("under review") || s.contains("review") {
            "under_review"
        } else if s.contains("submitted") || s.contains("submit") {
            "evidence_submitted"
        } else if s.contains("reported") || s.contains("complete") {
            "evidence_reported"
        } else if s.contains("rejected") {
            "evidence_rejected"
        } else if s.contains("archived") {
            "evidence_archived"
        } else if s.contains("draft") {
            "evidence_draft"
        } else {
            "system"
        }
    }

    // ── Extract a UUID-shaped string from free text ──────────────────────────
    fn extract_uuid(text: &str) -> Option<String> {
        let chars: Vec<char> = text.chars().collect();
        for i in 0..chars.len() {
            if i + 36 > chars.len() { break; }
            let candidate: String = chars[i..i + 36].iter().collect();
            let parts: Vec<&str> = candidate.split('-').collect();
            if parts.len() == 5
                && parts[0].len() == 8
                && parts[1].len() == 4
                && parts[2].len() == 4
                && parts[3].len() == 4
                && parts[4].len() == 12
                && parts.iter().all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
            {
                return Some(candidate);
            }
        }
        None
    }

    // ── Enrich each notification ─────────────────────────────────────────────
    let mut enriched: Vec<serde_json::Value> = Vec::with_capacity(notifications.len());

    for n in &notifications {
        let mut entry = serde_json::to_value(n).unwrap_or(json!({}));

        let title   = entry["title"].as_str().unwrap_or("").to_string();
        let message = entry["message"].as_str()
            .or_else(|| entry["body"].as_str())
            .unwrap_or("")
            .to_string();

        entry["notification_type"] = json!(infer_type(&title, &message));

        // Find an evidence ID from known fields or embedded text
        let combined = format!("{} {} {} {}",
            title, message,
            entry["evidence_id"].as_str().unwrap_or(""),
            entry["related_id"].as_str().unwrap_or(""),
        );

        let maybe_ev_id = entry["evidence_id"]
            .as_str().map(|s| s.to_string())
            .or_else(|| entry["related_id"].as_str().map(|s| s.to_string()))
            .or_else(|| extract_uuid(&combined));

        if let Some(ev_id) = maybe_ev_id {
            if let Ok(Some(ev)) = evidence_service.get_evidence(&ev_id, false).await {
                entry["related_evidence"] = json!({
                    "id":               ev.id,
                    "evidence_number":  ev.evidence_number,
                    "title":            ev.title,
                    "status":           format!("{:?}", ev.status),
                    "emergency_level":  format!("{:?}", ev.emergency_level),
                    "incident_type":    format!("{:?}", ev.incident_type),
                    "county":           ev.location.county,     // ✅ correct path
                    "incident_time":    ev.incident_time.format("%d %b %Y").to_string(),
                    "reported_to_police": ev.reported_to_police,
                    "view_url":         format!("/evidence/view/{}", ev.id),
                });
            }
        }

        enriched.push(entry);
    }

    let unread_count = enriched.iter()
        .filter(|n| !n["read"].as_bool().unwrap_or(false))
        .count();

    HttpResponse::Ok().json(ApiResponse::success(json!({
        "notifications":  enriched,
        "count":          enriched.len(),
        "unread_count":   unread_count,
    })))
}


/// GET /api/watchlist/targets
///
/// Returns all records from the billboard that are flagged as POI or watchlist.
/// Derived from the existing billboard data — no separate DB table needed.
pub async fn api_watchlist_targets(
    session:  Session,
    database: web::Data<crate::database::Database>,
) -> HttpResponse {
    let _user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"success":false,"error":"Not authenticated"})),
    };
    match database.get_billboard_data().await {
        Ok(data) => {
            // get_billboard_data returns a Value; extract the records array and
            // keep only those with poi / watchlist flags.
            let empty = vec![];
            let records = data.get("records")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty);
            let targets: Vec<&serde_json::Value> = records.iter().filter(|r| {
                r.get("is_poi").and_then(|v| v.as_bool()).unwrap_or(false)
                || r.get("poi").and_then(|v| v.as_bool()).unwrap_or(false)
                || r.get("is_watchlist").and_then(|v| v.as_bool()).unwrap_or(false)
                || r.get("watchlist").and_then(|v| v.as_bool()).unwrap_or(false)
                || r.get("flag_type").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false)
            }).collect();
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "data": { "targets": targets, "total": targets.len() },
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("{}", e),
        })),
    }
}

/// GET /api/bounties
///
/// Returns evidence records with Reported status, which are treated as
/// active bounty cases on the frontend billboard.
pub async fn api_bounties(
    session:  Session,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let _user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"success":false,"error":"Not authenticated"})),
    };
    let filters = EvidenceSearchFilters {
        status: Some("Reported".to_string()),
        query: None,
        incident_type: None,
        county: None,
        emergency_level: None,
        reported_to_police: None,
        needs_attention: None,
        signed_only: None,
        uploader_id: None,
        date_from: None,
        date_to: None,
        start_date: None,
        end_date: None,
        sort_by: Some("newest".to_string()),
        page: 1,
        limit: 100,
    };
    match evidence_service.search_evidence_with_filters(&filters, "").await {
        Ok(result) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "data": { "bounties": result.summaries, "total": result.total },
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("{}", e),
        })),
    }
}

/// GET /api/evidence/closed
///
/// Returns Archived evidence records — cases that have been closed,
/// taken down, or deleted from the active billboard.
pub async fn api_evidence_closed(
    session:  Session,
    evidence_service: web::Data<EvidenceService>,
) -> HttpResponse {
    let _user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => return HttpResponse::Unauthorized()
            .json(serde_json::json!({"success":false,"error":"Not authenticated"})),
    };
    let filters = EvidenceSearchFilters {
        status: Some("Archived".to_string()),
        query: None,
        incident_type: None,
        county: None,
        emergency_level: None,
        reported_to_police: None,
        needs_attention: None,
        signed_only: None,
        uploader_id: None,
        date_from: None,
        date_to: None,
        start_date: None,
        end_date: None,
        sort_by: Some("newest".to_string()),
        page: 1,
        limit: 100,
    };
    match evidence_service.search_evidence_with_filters(&filters, "").await {
        Ok(result) => HttpResponse::Ok().json(serde_json::json!({
            "success": true,
            "data": { "closed": result.summaries, "total": result.total },
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "success": false,
            "error": format!("{}", e),
        })),
    }
}


/// GET /api/watchlist/billboard
///
/// Returns all platform-wide flagged targets enriched with evidence
/// metadata, plus a live activity feed from the audit log.
/// The frontend Alpine component calls this on init and every 30 s.
pub async fn api_watchlist_billboard(
    session:  Session,
    database: web::Data<crate::database::Database>,
) -> HttpResponse {
    // ── Auth guard ───────────────────────────────────────────────
    let _user_id = match session.get::<String>("user_id").unwrap_or(None) {
        Some(id) => id,
        None => {
            return HttpResponse::Unauthorized()
                .json(ApiResponse::<()>::error("Not authenticated"));
        }
    };

    // ── Fetch billboard data ─────────────────────────────────────
    match database.get_billboard_data().await {
        Ok(data) => {
            HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "data":    data,
            }))
        }
        Err(e) => {
            println!("❌ BILLBOARD: DB error: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "error":   format!("{}", e),
            }))
        }
    }
}





// ==================== CONFIG FUNCTION ====================
// NOTE: flag-target routes are now owned by target_routes.rs
// (/api/evidence/{id}/flag-target and /api/evidence/{id}/flag-targets)

pub fn config(cfg: &mut web::ServiceConfig) {
    // Page routes
    cfg.route("/evidence/dashboard", web::get().to(evidence_dashboard))
       .route("/evidence/browse", web::get().to(evidence_browse_page))
       .route("/evidence/upload", web::get().to(evidence_upload_page))
       .route("/evidence/complete/{id}", web::get().to(evidence_complete_page))
       .route("/evidence/view/{id}", web::get().to(evidence_view_page))
       .route("/evidence/my", web::get().to(evidence_my_page));

    cfg.route("/api/evidence/complete", web::post().to(api_complete_evidence))
       .route("/api/evidence/stats", web::get().to(api_get_evidence_stats))
       .route("/api/evidence/{id}/update", web::post().to(api_update_evidence))
       .route("/api/evidence/upload", web::post().to(api_evidence_upload))
       .route("/api/evidence/{id}/submit", web::post().to(api_evidence_submit))
       .route("/api/evidence/{id}/sign", web::post().to(api_evidence_sign))
       .route("/api/evidence/{id}/report-police", web::post().to(api_evidence_report_to_police))
       .route("/api/evidence/{id}/delete", web::post().to(api_evidence_delete))
       .route("/api/evidence/{id}/targets", web::get().to(api_get_evidence_targets))
       .route("/api/evidence/search", web::post().to(api_search_evidence))
       .route("/api/evidence/targets/upload", web::post().to(api_upload_targets))
       .route("/api/evidence/locations", web::get().to(api_get_evidence_locations))
       .route("/targets", web::get().to(targets_page))
       .route("/maps/dashboard", web::get().to(maps_dashboard_page))
       .route("/api/maps/data", web::get().to(api_get_map_data));

    // User / wallet routes
    // NOTE: /api/user/profile/update was registered twice across two cfg blocks.
    // Consolidated here into a single registration.
    cfg.route("/user/profile", web::get().to(user_profile_page))
       .route("/api/user/profile/update", web::post().to(api_update_profile))
       .route("/api/user/password/change", web::post().to(api_change_password))
       .route("/api/user/activity", web::get().to(api_get_user_activity))
       // Audit-log endpoint (preferred by profile page; returns flat logs array)
       .route("/api/user/audit-logs", web::get().to(api_get_user_audit_logs))
       // Notifications under /api/user/ namespace (profile page calls this path)
       .route("/api/user/notifications", web::get().to(api_get_all_notifications))
       .route("/api/user/notifications/read-all", web::post().to(api_mark_all_notifications_read))
       .route("/api/user/wallets", web::get().to(api_get_wallet_connections))
       .route("/api/wallet/disconnect", web::post().to(api_wallet_disconnect));

    // Notification and linked evidence routes
    cfg.route("/api/notifications", web::get().to(api_get_notifications))
       .route("/api/notifications/count", web::get().to(api_get_notification_count))
       .route("/api/notifications/{id}/read", web::post().to(api_mark_notification_read))
       .route("/api/evidence/{id}/linked", web::get().to(api_get_linked_evidence))
       .route("/api/linked-cases", web::get().to(api_get_linked_cases))
       .route("/api/linked-cases/create", web::post().to(api_create_linked_case))
       .route("/api/linked-cases/{id}", web::delete().to(api_delete_linked_case))
       .route("/api/notifications/all", web::get().to(api_get_all_notifications))
       .route("/notifications", web::get().to(notifications_page))
       .route("/api/watchlist/billboard",    web::get().to(api_watchlist_billboard))
       .route("/api/watchlist/targets",      web::get().to(api_watchlist_targets))
       .route("/api/bounties",               web::get().to(api_bounties))
       .route("/api/evidence/closed",        web::get().to(api_evidence_closed))
       .route("/linked-cases", web::get().to(linked_cases_page));

}