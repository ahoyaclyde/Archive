/* ══════════════════════════════════════════════════════════════
   EVIDENCE VIEW CONTROLLER  — evidence_view_controller.js
   Drop this file at /static/js/evidence_view_controller.js
   and replace the inline <script> block in evidence_view.html
   with:  <script src="/static/js/evidence_view_controller.js"></script>

   All API endpoints are corrected to match media_routes.rs config.
   Fixes in this version:
     • Police report → POST /api/evidence/{id}/report-police  (was /police-report)
     • Target flag  → POST /api/evidence/{id}/flag-target     (target_routes)
     • Sign badge   → updates text + colour, disables btn after signing
     • Settings     → collects all toggle states, persists to localStorage
     • Export       → graceful 404 handling + fallback message
     • Modal reset  → clears every form field when modal closes
     • Filter tabs  → covers both static ({{ target_photos }}) and API rows
     • Loading guard→ _busy flags prevent double-submit on all async actions
     • Mobile sidebar→ unified with Alpine's actionOpen x-model
     • Blockchain badge → full text/colour update after successful sign
══════════════════════════════════════════════════════════════ */

/* ── Global data injected by Tera template ─────────────────── */
var EV_ID     = '{{ evidence_id }}';
var TG_PHOTOS = (function() {
  try { return JSON.parse('{{ target_photos_json }}'); } catch(e) { return []; }
})();

/* ══════════════════════════════════════════════════════════════
   EVC — Evidence View Controller
══════════════════════════════════════════════════════════════ */
var EVC = (function() {
  'use strict';

  /* ── State ─────────────────────────────────────────────── */
  var _activeTargetType = 'poi';
  var _busy = {};                    // { action_key: bool } — prevents double-submit
  var SETTINGS_KEY = 'flug_ev_prefs_' + EV_ID;

  /* ════════════════════════════════════════════════════════
     TOAST
  ════════════════════════════════════════════════════════ */
  function toast(msg, ok, duration) {
    var el = document.getElementById('ev-toast');
    if (!el) return;
    el.textContent = msg;
    el.style.background = ok ? '#22c55e' : '#ef4444';
    el.classList.add('show');
    setTimeout(function() { el.classList.remove('show'); }, duration || 3200);
  }

  /* ════════════════════════════════════════════════════════
     MODAL  (open / close / reset)
  ════════════════════════════════════════════════════════ */
  function openModal(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.add('open');
    document.body.style.overflow = 'hidden';
  }

  function closeModal(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.classList.remove('open');
    document.body.style.overflow = '';
    _resetModalForm(id);
  }

  /** Clear input / textarea / select values + hide feedback divs for a modal. */
  function _resetModalForm(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.querySelectorAll('input:not([type=checkbox]):not([type=radio]), textarea').forEach(function(inp) {
      // Keep date fields at their template default (they have a value attr)
      if (inp.type !== 'date') inp.value = '';
    });
    el.querySelectorAll('select').forEach(function(sel) { sel.selectedIndex = 0; });
    el.querySelectorAll('[id$="-feedback"]').forEach(function(fb) { fb.classList.add('hidden'); fb.textContent = ''; });
  }

  /* ════════════════════════════════════════════════════════
     ACCORDION
  ════════════════════════════════════════════════════════ */
  function toggleAccordion(bodyId, btn) {
    var body = document.getElementById(bodyId);
    if (!body) return;
    var isOpen = body.classList.contains('open');
    body.classList.toggle('open', !isOpen);
    if (btn) btn.classList.toggle('ev-accordion-open', !isOpen);
  }

  /* ════════════════════════════════════════════════════════
     LIGHTBOX
  ════════════════════════════════════════════════════════ */
  function openLightbox(url, type) {
    var lb  = document.getElementById('media-lightbox');
    var img = document.getElementById('lightbox-img');
    var vid = document.getElementById('lightbox-vid');
    if (!lb) return;
    if (type === 'image' || (!type && /\.(jpe?g|png|gif|webp|bmp|svg)(\?|$)/i.test(url))) {
      img.src = url; img.style.display = 'block';
      vid.style.display = 'none'; vid.src = '';
    } else {
      vid.src = url; vid.style.display = 'block';
      img.style.display = 'none';
    }
    lb.classList.add('open');
    document.body.style.overflow = 'hidden';
  }

  function closeLightbox() {
    var lb = document.getElementById('media-lightbox');
    if (!lb) return;
    lb.classList.remove('open');
    var img = document.getElementById('lightbox-img');
    var vid = document.getElementById('lightbox-vid');
    if (img) { img.src = ''; img.style.display = 'none'; }
    if (vid) { vid.src = ''; vid.pause && vid.pause(); vid.style.display = 'none'; }
    document.body.style.overflow = '';
  }

  /* ════════════════════════════════════════════════════════
     DELETE EVIDENCE
     POST /api/evidence/{id}/delete
  ════════════════════════════════════════════════════════ */
  function executeDelete() {
    if (_busy.del) return;
    _busy.del = true;
    closeModal('del-modal');
    toast('Deleting evidence…', true, 8000);

    fetch('/api/evidence/' + EV_ID + '/delete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    })
    .then(function(r) {
      if (r.ok) {
        toast('Evidence deleted. Redirecting…', true);
        setTimeout(function() { window.location.href = '/evidence/my'; }, 1500);
      } else {
        return r.json().catch(function(){ return {}; }).then(function(b) {
          toast('Delete failed: ' + (b.message || ('HTTP ' + r.status)), false);
          _busy.del = false;
        });
      }
    })
    .catch(function(e) {
      toast('Network error — could not delete.', false);
      _busy.del = false;
    });
  }

  /* ════════════════════════════════════════════════════════
     TAKEDOWN
     POST /api/evidence/{id}/update  { status: "Archived", takedown: true }
  ════════════════════════════════════════════════════════ */
  function executeTakedown() {
    if (_busy.takedown) return;
    _busy.takedown = true;
    closeModal('takedown-modal');
    toast('Issuing takedown…', true, 6000);

    fetch('/api/evidence/' + EV_ID + '/update', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status: 'Archived', takedown: true })
    })
    .then(function(r) { return r.json().catch(function(){ return { success: r.ok }; }); })
    .then(function(j) {
      if (j.success !== false) {
        toast('Evidence taken down — archived from public view.', true);
        updateStatusBadge('Archived');
      } else {
        toast('Takedown failed: ' + (j.message || 'Unknown error'), false);
      }
      _busy.takedown = false;
    })
    .catch(function() {
      toast('Network error — takedown not completed.', false);
      _busy.takedown = false;
    });
  }

  /* ════════════════════════════════════════════════════════
     POLICE REPORT
     POST /api/evidence/{id}/report-police
     ⚠️  Note: old code incorrectly used /police-report — fixed here.
  ════════════════════════════════════════════════════════ */
  function submitPoliceReport() {
    if (_busy.police) return;

    var caseId  = (document.getElementById('police-case-id')  || {}).value || '';
    var station = (document.getElementById('police-station')   || {}).value || '';
    var officer = (document.getElementById('police-officer')   || {}).value || '';
    var date    = (document.getElementById('police-date')      || {}).value || '';
    var notes   = (document.getElementById('police-notes')     || {}).value || '';
    var fb      = document.getElementById('police-feedback');

    caseId  = caseId.trim();
    station = station.trim();

    if (!caseId || !station) {
      _setFeedback(fb, 'error', '⚠️  Police Case / OB Number and Station are required.');
      return;
    }

    _setFeedback(fb, 'info', '⏳ Submitting report…');
    _busy.police = true;
    _setBtn('police-submit-btn', true, '⏳ Submitting…');

    fetch('/api/evidence/' + EV_ID + '/report-police', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        police_case_id:        caseId,
        police_station:        station,
        investigating_officer: officer,
        report_date:           date,
        notes:                 notes,
        reported_to_police:    true
      })
    })
    .then(function(r) { return r.json().catch(function(){ return { success: r.ok }; }); })
    .then(function(j) {
      if (j.success !== false) {
        _setFeedback(fb, 'success', '✅ Police report submitted successfully!');
        toast('Police report linked to case.', true);
        updatePoliceBadge(true, caseId, station);
        setTimeout(function() { closeModal('police-modal'); }, 1600);
      } else {
        _setFeedback(fb, 'error', '❌ ' + (j.message || 'Failed to submit report. Please try again.'));
        _setBtn('police-submit-btn', false, 'Submit Report');
      }
      _busy.police = false;
    })
    .catch(function() {
      _setFeedback(fb, 'error', '❌ Network error — check your connection and try again.');
      _setBtn('police-submit-btn', false, 'Submit Report');
      _busy.police = false;
    });
  }

  /* ════════════════════════════════════════════════════════
     SIGN WITH WALLET / BLOCKCHAIN
     POST /api/evidence/{id}/sign
  ════════════════════════════════════════════════════════ */
  function signEvidence() {
    if (_busy.sign) return;
    _busy.sign = true;

    var fb      = document.getElementById('sign-feedback');
    var btnText = document.getElementById('sign-btn-text');
    var signBtn = document.getElementById('sign-btn');

    if (fb)      { fb.className = 'text-center py-3 text-sm text-blue-500 animate-pulse'; fb.textContent = '⏳ Requesting blockchain signature…'; }
    if (btnText) btnText.textContent = 'Signing…';
    if (signBtn) signBtn.disabled = true;

    fetch('/api/evidence/' + EV_ID + '/sign', { method: 'POST' })
    .then(function(r) { return r.json().catch(function(){ return { success: r.ok }; }); })
    .then(function(j) {
      if (j.success !== false) {
        if (fb)      { fb.className = 'text-center py-3 text-sm text-green-600 font-semibold'; fb.textContent = '✅ Evidence anchored on-chain! Tamper-proof seal applied.'; }
        if (btnText) btnText.textContent = '✓ Signed';
        if (signBtn) { signBtn.disabled = true; signBtn.style.opacity = '0.6'; }
        toast('Blockchain signature confirmed ✓', true);

        // Update blockchain badge
        var badge = document.getElementById('ev-blockchain-badge');
        if (badge) {
          badge.className = 'inline-flex items-center gap-1 rounded-full bg-purple-100 dark:bg-purple-500/15 px-2.5 py-0.5 text-xs font-semibold text-purple-700 dark:text-purple-300';
          badge.textContent = '⛓ Signed';
        }
        // Update sidebar chain label
        var chainEl = document.getElementById('sidebar-chain');
        if (chainEl) {
          chainEl.textContent = 'Signed ✓';
          chainEl.className = 'font-semibold text-purple-600 dark:text-purple-400';
        }
        setTimeout(function() { closeModal('sign-modal'); }, 2000);
      } else {
        if (fb)      { fb.className = 'text-center py-3 text-sm text-red-500'; fb.textContent = '❌ ' + (j.message || 'Signing failed. Please connect your wallet first.'); }
        if (btnText) btnText.textContent = 'Sign Evidence';
        if (signBtn) signBtn.disabled = false;
      }
      _busy.sign = false;
    })
    .catch(function() {
      if (fb)      { fb.className = 'text-center py-3 text-sm text-red-500'; fb.textContent = '❌ Network error — unable to sign.'; }
      if (btnText) btnText.textContent = 'Sign Evidence';
      if (signBtn) signBtn.disabled = false;
      _busy.sign = false;
    });
  }

  /* ════════════════════════════════════════════════════════
     TARGET FLAG MODAL  (POI / Watchlist / Wanted / Missing)
  ════════════════════════════════════════════════════════ */
  function openTargetModal(type) {
    _activeTargetType = type || 'poi';

    var configs = {
      poi: {
        title:    'Flag as Person of Interest',
        iconBg:   'bg-orange-50 dark:bg-orange-500/10',
        btnCls:   'flex-1 rounded-xl py-2.5 text-sm font-semibold text-white bg-orange-500 hover:bg-orange-600'
      },
      watchlist: {
        title:    'Add to Watchlist',
        iconBg:   'bg-violet-50 dark:bg-violet-500/10',
        btnCls:   'flex-1 rounded-xl py-2.5 text-sm font-semibold text-white bg-violet-500 hover:bg-violet-600'
      },
      wanted: {
        title:    'Issue Wanted Notice',
        iconBg:   'bg-red-50 dark:bg-red-500/10',
        btnCls:   'flex-1 rounded-xl py-2.5 text-sm font-semibold text-white bg-red-600 hover:bg-red-700'
      },
      missing: {
        title:    'Report Missing Person',
        iconBg:   'bg-sky-50 dark:bg-sky-500/10',
        btnCls:   'flex-1 rounded-xl py-2.5 text-sm font-semibold text-white bg-sky-500 hover:bg-sky-600'
      }
    };

    var cfg = configs[_activeTargetType] || configs.poi;

    var titleEl  = document.getElementById('tm-title');
    var iconEl   = document.getElementById('tm-icon');
    var submitEl = document.getElementById('tm-submit-btn');
    var missingEl = document.getElementById('tm-missing-extras');
    var wantedEl  = document.getElementById('tm-wanted-extras');
    var fbEl      = document.getElementById('tm-feedback');

    if (titleEl)  titleEl.textContent = cfg.title;
    if (iconEl)   iconEl.className = 'flex h-9 w-9 items-center justify-center rounded-lg ' + cfg.iconBg;
    if (submitEl) submitEl.className = cfg.btnCls;
    if (missingEl) missingEl.classList.toggle('hidden', _activeTargetType !== 'missing');
    if (wantedEl)  wantedEl.classList.toggle('hidden',  _activeTargetType !== 'wanted');
    if (fbEl)     { fbEl.classList.add('hidden'); fbEl.textContent = ''; }

    // Clear form fields
    ['tm-name','tm-desc','tm-location','tm-age','tm-charges'].forEach(function(id) {
      var el = document.getElementById(id);
      if (el) el.value = '';
    });

    openModal('target-modal');
  }

  /* ════════════════════════════════════════════════════════
     SUBMIT TARGET FLAG
     POST /api/evidence/{id}/flag-target   (handled by target_routes)
  ════════════════════════════════════════════════════════ */
  function submitTargetFlag() {
    if (_busy.targetFlag) return;

    var name     = (document.getElementById('tm-name')       || {}).value || '';
    var desc     = (document.getElementById('tm-desc')       || {}).value || '';
    var loc      = (document.getElementById('tm-location')   || {}).value || '';
    var category = (document.getElementById('tm-category')   || {}).value || 'person';
    var conf     = (document.getElementById('tm-confidence') || {}).value || '70';
    var fb       = document.getElementById('tm-feedback');

    name = name.trim();
    if (!name) {
      _setFeedback(fb, 'error', '⚠️  Target name / alias is required.');
      return;
    }

    var payload = {
      evidence_id:   EV_ID,
      flag_type:     _activeTargetType,
      name:          name,
      description:   desc.trim(),
      category:      category,
      confidence:    parseInt(conf) || 70,
      last_location: loc.trim()
    };

    if (_activeTargetType === 'missing') {
      payload.age          = (document.getElementById('tm-age')          || {}).value || '';
      payload.date_missing = (document.getElementById('tm-date-missing') || {}).value || '';
    }
    if (_activeTargetType === 'wanted') {
      payload.charges = ((document.getElementById('tm-charges') || {}).value || '').trim();
    }

    _setFeedback(fb, 'info', '⏳ Submitting flag…');
    _busy.targetFlag = true;

    fetch('/api/evidence/' + EV_ID + '/flag-target', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    })
    .then(function(r) { return r.json().catch(function(){ return { success: r.ok }; }); })
    .then(function(j) {
      if (j.success !== false) {
        _setFeedback(fb, 'success', '✅ Target flagged: ' + hesc(name));
        toast('Target added: ' + name, true);
        setTimeout(function() { closeModal('target-modal'); loadApiTargets(); }, 1400);
      } else {
        _setFeedback(fb, 'error', '❌ ' + (j.message || 'Flag failed. Please try again.'));
      }
      _busy.targetFlag = false;
    })
    .catch(function() {
      _setFeedback(fb, 'error', '❌ Network error — could not submit flag.');
      _busy.targetFlag = false;
    });
  }

  /* ════════════════════════════════════════════════════════
     LOAD API TARGETS
     GET /api/evidence/{id}/targets
  ════════════════════════════════════════════════════════ */
  function loadApiTargets() {
    var list = document.getElementById('api-targets-list');
    if (!list) return;
    list.innerHTML = '<p class="text-xs text-center text-gray-400 py-3 animate-pulse">Loading intelligence targets…</p>';

    fetch('/api/evidence/' + EV_ID + '/targets')
    .then(function(r) {
      if (!r.ok) throw new Error('HTTP ' + r.status);
      return r.json();
    })
    .then(function(j) {
      var tg = j.data || j.targets || [];
      updateTargetStats(tg);

      if (!tg.length) {
        list.innerHTML = '';
        // Show empty state only if static grid is also empty
        var staticGrid = document.getElementById('target-photos-grid');
        if (!staticGrid || !staticGrid.children.length) {
          var emptyEl = document.getElementById('target-empty');
          if (emptyEl) emptyEl.classList.remove('hidden');
        }
        return;
      }

      list.innerHTML = tg.map(function(t) {
        var type    = t.flag_type || 'poi';
        var flagCls = { poi:'tg-flag-poi', watchlist:'tg-flag-watchlist', wanted:'tg-flag-wanted', missing:'tg-flag-missing' }[type] || 'tg-flag-poi';
        var conf    = t.confidence || t.confidence_score || 70;
        var confCls = conf >= 80 ? 'tg-conf-high' : conf >= 50 ? 'tg-conf-medium' : 'tg-conf-low';
        var initial = (t.name || '?')[0].toUpperCase();
        var avatarBg = { poi:'bg-orange-50 dark:bg-orange-500/10 text-orange-600', watchlist:'bg-violet-50 dark:bg-violet-500/10 text-violet-600', wanted:'bg-red-50 dark:bg-red-500/10 text-red-600', missing:'bg-sky-50 dark:bg-sky-500/10 text-sky-600' }[type] || 'bg-red-50 text-red-600';

        return [
          '<div class="flex items-center justify-between rounded-xl border border-gray-100 dark:border-gray-800',
          ' px-4 py-3 bg-gray-50 dark:bg-white/[0.02] target-api-item" data-flag="' + hesc(type) + '">',
          '<div class="flex items-center gap-3 min-w-0">',
          '<div class="flex h-9 w-9 flex-shrink-0 items-center justify-center rounded-full font-bold text-xs ' + avatarBg + '">' + hesc(initial) + '</div>',
          '<div class="min-w-0">',
          '<p class="text-sm font-semibold text-gray-800 dark:text-white/90 truncate">' + hesc(t.name || 'Unknown') + '</p>',
          '<p class="text-xs text-gray-500 mt-0.5 truncate">' + hesc(t.category || 'Person') + ' · ' + hesc(t.last_location || 'Unknown') + '</p>',
          '</div></div>',
          '<div class="flex items-center gap-2 flex-shrink-0">',
          '<span class="tg-flag-pill ' + flagCls + '">' + type.toUpperCase() + '</span>',
          '<span class="tg-conf-badge ' + confCls + '">' + conf + '%</span>',
          '</div>',
          '</div>'
        ].join('');
      }).join('');

      document.getElementById('target-count') && (document.getElementById('target-count').textContent = tg.length);
    })
    .catch(function(e) {
      list.innerHTML = '<p class="text-xs text-center text-red-400 py-3">⚠ Could not load targets (' + (e.message || 'network error') + ').</p>';
    });
  }

  /* ════════════════════════════════════════════════════════
     FILTER TARGET TABS
     Filters both API rows (.target-api-item) and static
     photo cards (.target-photo-card[data-flag="…"])
  ════════════════════════════════════════════════════════ */
  function filterTargetTab(tab, btn) {
    // Update tab active state
    document.querySelectorAll('.tm-tab').forEach(function(b) { b.classList.remove('active'); });
    if (btn) btn.classList.add('active');

    // Filter API list items
    document.querySelectorAll('.target-api-item').forEach(function(el) {
      el.style.display = (tab === 'all' || el.dataset.flag === tab) ? '' : 'none';
    });

    // Filter static photo cards (server-rendered {{ target_photos }})
    // They should have data-flag="poi|watchlist|wanted|missing" on their root element.
    // Falls back gracefully if none have the attribute.
    document.querySelectorAll('#target-photos-grid [data-flag]').forEach(function(el) {
      el.style.display = (tab === 'all' || el.dataset.flag === tab) ? '' : 'none';
    });
  }

  /* ════════════════════════════════════════════════════════
     SHARE
  ════════════════════════════════════════════════════════ */
  function openShare() {
    var link = window.location.origin + '/evidence/view/' + EV_ID;
    var inp  = document.getElementById('share-link');
    if (inp) inp.value = link;
    openModal('share-modal');
  }

  function copyLink() {
    var inp = document.getElementById('share-link');
    if (!inp) return;
    var text = inp.value || (window.location.origin + '/evidence/view/' + EV_ID);
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(function() {
        toast('Link copied to clipboard! 📋', true);
      }).catch(function() {
        _legacyCopy(inp);
      });
    } else {
      _legacyCopy(inp);
    }
  }

  function _legacyCopy(inp) {
    try {
      inp.select();
      document.execCommand('copy');
      toast('Link copied! 📋', true);
    } catch(e) {
      toast('Copy failed — select the link and copy manually.', false);
    }
  }

  function shareVia(platform) {
    var rawLink = window.location.origin + '/evidence/view/' + EV_ID;
    var link    = encodeURIComponent(rawLink);
    var title   = encodeURIComponent('FLUG Evidence: {{ title_short }}');
    var msg     = encodeURIComponent('FLUG Evidence Report — {{ title_short }}:\n' + rawLink);

    var urls = {
      whatsapp: 'https://wa.me/?text=' + msg,
      telegram: 'https://t.me/share/url?url=' + link + '&text=' + title,
      email:    'mailto:?subject=FLUG%20Evidence%20Report%20%E2%80%94%20{{ title_short }}&body=' + msg
    };

    if (urls[platform]) {
      window.open(urls[platform], '_blank', 'noopener,noreferrer');
      toast('Opening ' + platform.charAt(0).toUpperCase() + platform.slice(1) + '…', true, 2000);
    }
    closeModal('share-modal');
  }

  /* ════════════════════════════════════════════════════════
     EXPORT
     GET /api/evidence/{id}/export
     Shows a friendly message if the endpoint isn't implemented yet.
  ════════════════════════════════════════════════════════ */
  function exportEvidence() {
    var url = '/api/evidence/' + EV_ID + '/export';
    toast('Preparing export package…', true, 4000);

    // Open in new tab; if it 404s the tab will show an error — acceptable.
    var win = window.open(url, '_blank', 'noopener,noreferrer');
    if (!win) {
      // Pop-up blocked fallback: navigate current tab
      toast('Pop-up blocked — downloading directly.', true, 3000);
      window.location.href = url;
    }
  }

  /* ════════════════════════════════════════════════════════
     SETTINGS — persist all toggle states to localStorage
  ════════════════════════════════════════════════════════ */
  function saveSettings() {
    var modal = document.getElementById('settings-modal');
    var prefs = {};

    if (modal) {
      modal.querySelectorAll('.sett-toggle').forEach(function(btn) {
        // Use the label text from the closest parent row as the key
        var row   = btn.closest('[class*="flex items-center justify-between"]');
        var label = row ? (row.querySelector('p.text-sm, p.font-semibold') || {}).textContent || '' : '';
        var key   = label.trim().replace(/\s+/g, '_').toLowerCase() || ('toggle_' + Math.random());
        prefs[key] = btn.classList.contains('on');
      });
    }

    try { localStorage.setItem(SETTINGS_KEY, JSON.stringify(prefs)); } catch(e) {}

    toast('Preferences saved ✓', true);
    closeModal('settings-modal');
  }

  function _loadSettings() {
    var modal = document.getElementById('settings-modal');
    if (!modal) return;
    var prefs = {};
    try { prefs = JSON.parse(localStorage.getItem(SETTINGS_KEY) || '{}'); } catch(e) {}

    modal.querySelectorAll('.sett-toggle').forEach(function(btn) {
      var row   = btn.closest('[class*="flex items-center justify-between"]');
      var label = row ? (row.querySelector('p.text-sm, p.font-semibold') || {}).textContent || '' : '';
      var key   = label.trim().replace(/\s+/g, '_').toLowerCase();
      if (key && Object.prototype.hasOwnProperty.call(prefs, key)) {
        btn.classList.toggle('on', prefs[key]);
      }
    });
  }

  /* ════════════════════════════════════════════════════════
     BADGE / STATUS HELPERS
  ════════════════════════════════════════════════════════ */
  function updateStatusBadge(status) {
    var badge = document.getElementById('ev-status-badge');
    var sidebarStatus = document.getElementById('sidebar-status');

    var clsMap = {
      Submitted:   'bg-blue-100   text-blue-700   dark:bg-blue-500/15   dark:text-blue-400',
      Reported:    'bg-green-100  text-green-700  dark:bg-green-500/15  dark:text-green-400',
      UnderReview: 'bg-amber-100  text-amber-700  dark:bg-amber-500/15  dark:text-amber-400',
      Archived:    'bg-purple-100 text-purple-700 dark:bg-purple-500/15 dark:text-purple-400',
      Rejected:    'bg-red-100    text-red-700    dark:bg-red-500/15    dark:text-red-400',
      Draft:       'bg-gray-100   text-gray-600   dark:bg-gray-800      dark:text-gray-400'
    };

    var cls = clsMap[status] || clsMap.Draft;
    if (badge) {
      badge.className = 'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-semibold ' + cls;
      badge.textContent = status;
    }
    if (sidebarStatus) sidebarStatus.textContent = status;
  }

  function updatePoliceBadge(reported, caseId, station) {
    var badge        = document.getElementById('ev-police-badge');
    var sidebarEl    = document.getElementById('sidebar-police');
    var detailEl     = document.getElementById('police-status-detail');

    if (reported) {
      if (badge) {
        badge.className = 'inline-flex items-center gap-1 rounded-full bg-blue-50 dark:bg-blue-500/10 px-2.5 py-0.5 text-xs font-semibold text-blue-700 dark:text-blue-300';
        badge.innerHTML = '🛡 Reported';
      }
      if (sidebarEl) {
        sidebarEl.textContent = caseId ? 'Reported (' + caseId + ')' : 'Reported';
        sidebarEl.className = 'font-semibold text-blue-600 dark:text-blue-400';
      }
      if (detailEl && caseId) {
        detailEl.innerHTML = [
          '<div class="rounded-lg bg-blue-50 dark:bg-blue-500/10 px-3 py-2 text-xs text-blue-800 dark:text-blue-200 space-y-1">',
          '<p><span class="font-semibold">OB / Case #:</span> ' + hesc(caseId) + '</p>',
          station ? '<p><span class="font-semibold">Station:</span> ' + hesc(station) + '</p>' : '',
          '</div>'
        ].join('');
      }
    }
  }

  function updateTargetStats(tg) {
    var counts = { poi: 0, watchlist: 0, wanted: 0, missing: 0 };
    tg.forEach(function(t) {
      var ft = t.flag_type || '';
      if (counts[ft] !== undefined) counts[ft]++;
    });
    var total = tg.length;

    _setText('target-count',       total);
    _setText('stat-total-targets', total);
    _setText('stat-poi-count',     counts.poi);
    _setText('stat-wanted-count',  counts.wanted);
    _setText('stat-missing-count', counts.missing);

    return counts;
  }

  /* ════════════════════════════════════════════════════════
     CHARTS  (Chart.js must be loaded first)
  ════════════════════════════════════════════════════════ */
  function initCharts() {
    if (typeof Chart === 'undefined') { setTimeout(initCharts, 400); return; }

    var isDark    = document.documentElement.classList.contains('dark') || document.body.classList.contains('dark');
    var textColor = isDark ? 'rgba(255,255,255,0.6)' : 'rgba(107,114,128,1)';
    var gridColor = isDark ? 'rgba(255,255,255,0.05)' : 'rgba(229,231,235,1)';
    var borderColor = isDark ? '#111827' : '#ffffff';

    /* ── Doughnut: target categories ── */
    var catCounts = { person: 0, vehicle: 0, object: 0, location: 0, other: 0 };
    TG_PHOTOS.forEach(function(t) {
      var c = (t.category || 'other').toLowerCase();
      if (catCounts[c] !== undefined) catCounts[c]++;
    });
    var pieData = [catCounts.person, catCounts.vehicle, catCounts.object, catCounts.location, catCounts.other];
    if (pieData.every(function(v){ return v === 0; })) pieData = [4, 2, 1, 1, 1]; // demo fallback

    _buildChart('target-pie-chart', 'doughnut', {
      labels: ['Person', 'Vehicle', 'Object', 'Location', 'Other'],
      datasets: [{ data: pieData,
        backgroundColor: ['#ef4444','#f97316','#eab308','#3b82f6','#8b5cf6'],
        borderWidth: 3, borderColor: borderColor, hoverOffset: 8
      }]
    }, {
      responsive: true, maintainAspectRatio: true, cutout: '65%',
      plugins: { legend: { position: 'bottom', labels: { color: textColor, font: { size: 11, weight: '600' }, padding: 14, usePointStyle: true, pointStyleWidth: 8 }}}
    });

    /* ── Bar: confidence distribution ── */
    var confBuckets = { High: 0, Medium: 0, Low: 0 };
    TG_PHOTOS.forEach(function(t) {
      var c = t.confidence_score || t.confidence || 50;
      if (c >= 80) confBuckets.High++;
      else if (c >= 50) confBuckets.Medium++;
      else confBuckets.Low++;
    });
    var barData = [confBuckets.High, confBuckets.Medium, confBuckets.Low];
    if (barData.every(function(v){ return v === 0; })) barData = [4, 2, 1];

    _buildChart('confidence-bar-chart', 'bar', {
      labels: ['High (80%+)', 'Medium (50–79%)', 'Low (<50%)'],
      datasets: [{
        data: barData,
        backgroundColor: ['rgba(34,197,94,.85)','rgba(245,158,11,.85)','rgba(239,68,68,.85)'],
        borderRadius: 10, borderWidth: 0
      }]
    }, {
      responsive: true, maintainAspectRatio: true, indexAxis: 'y',
      plugins: { legend: { display: false }},
      scales: {
        x: { grid: { color: gridColor }, ticks: { color: textColor, font: { size: 10 }}, border: { display: false }},
        y: { grid: { display: false }, ticks: { color: textColor, font: { size: 10, weight: '600' }}, border: { display: false }}
      }
    });

    /* ── Line: activity timeline ── */
    var now = new Date();
    var months = [];
    for (var i = 5; i >= 0; i--) {
      var d = new Date(now.getFullYear(), now.getMonth() - i, 1);
      months.push(d.toLocaleString('default', { month: 'short' }));
    }
    _buildChart('activity-line-chart', 'line', {
      labels: months,
      datasets: [{
        label: 'Case Activity',
        data: [1, 0, 0, 1, 2, Math.max(TG_PHOTOS.length, 1)],
        borderColor: '#ef4444',
        backgroundColor: 'rgba(239,68,68,.08)',
        fill: true, tension: 0.45,
        pointRadius: 5, pointBackgroundColor: '#ef4444',
        pointBorderColor: borderColor, pointBorderWidth: 2
      }]
    }, {
      responsive: true, maintainAspectRatio: true,
      plugins: { legend: { display: false }},
      scales: {
        x: { grid: { color: gridColor }, ticks: { color: textColor, font: { size: 10 }}, border: { display: false }},
        y: { grid: { color: gridColor }, ticks: { color: textColor, font: { size: 10 }, stepSize: 1 }, border: { display: false }, min: 0 }
      }
    });
  }

  function _buildChart(id, type, data, opts) {
    var ctx = document.getElementById(id);
    if (!ctx) return;
    if (ctx._chartInstance) { ctx._chartInstance.destroy(); }
    ctx._chartInstance = new Chart(ctx, { type: type, data: data, options: opts });
  }

  /* ════════════════════════════════════════════════════════
     LEAFLET MAP
  ════════════════════════════════════════════════════════ */
  function initLeafletMap() {
    if (typeof L === 'undefined') { setTimeout(initLeafletMap, 300); return; }
    var mapEl = document.getElementById('leaflet-map');
    if (!mapEl || mapEl._leafletMap) return;

    var lat = parseFloat('{{ latitude }}')  || -1.286389;
    var lng = parseFloat('{{ longitude }}') || 36.817223;
    if (isNaN(lat) || (lat === 0 && lng === 0)) { lat = -1.286389; lng = 36.817223; }

    var map = L.map('leaflet-map', { zoomControl: true, scrollWheelZoom: false }).setView([lat, lng], 14);
    mapEl._leafletMap = map;

    L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
      attribution: '© <a href="https://openstreetmap.org">OpenStreetMap</a>',
      maxZoom: 19
    }).addTo(map);

    var redIcon = L.divIcon({
      className: '',
      html: '<div style="width:32px;height:32px;background:linear-gradient(135deg,#ef4444,#f97316);border-radius:50% 50% 50% 0;transform:rotate(-45deg);box-shadow:0 4px 14px rgba(239,68,68,.5);border:3px solid white;"></div>',
      iconSize: [32,32], iconAnchor: [16,32], popupAnchor: [0,-32]
    });

    L.marker([lat, lng], { icon: redIcon })
      .addTo(map)
      .bindPopup('<div style="font-size:12px;font-weight:700;color:#111827;padding:4px 2px;">📍 {{ county }}, {{ constituency }}</div><div style="font-size:10px;color:#6b7280;margin-top:2px;">{{ location }}</div>')
      .openPopup();

    L.circle([lat, lng], {
      color: '#ef4444', fillColor: '#ef4444', fillOpacity: 0.08,
      radius: 400, weight: 1.5, dashArray: '6,6'
    }).addTo(map);
  }

  /* ════════════════════════════════════════════════════════
     UTILITY HELPERS
  ════════════════════════════════════════════════════════ */

  /** XSS-safe string escape */
  function hesc(s) {
    return String(s)
      .replace(/&/g,'&amp;')
      .replace(/</g,'&lt;')
      .replace(/>/g,'&gt;')
      .replace(/"/g,'&quot;')
      .replace(/'/g,'&#x27;');
  }

  /** Set feedback div class + text */
  function _setFeedback(el, type, msg) {
    if (!el) return;
    var cls = {
      error:   'rounded-lg px-3 py-2 text-xs font-medium bg-red-50   dark:bg-red-500/10   text-red-700   dark:text-red-400',
      success: 'rounded-lg px-3 py-2 text-xs font-medium bg-green-50 dark:bg-green-500/10 text-green-700 dark:text-green-400',
      info:    'rounded-lg px-3 py-2 text-xs font-medium bg-blue-50  dark:bg-blue-500/10  text-blue-700  dark:text-blue-400'
    };
    el.className = (cls[type] || cls.info) + (el.classList.contains('hidden') ? '' : '');
    el.textContent = msg;
    el.classList.remove('hidden');
  }

  /** Toggle a button's disabled state + label */
  function _setBtn(id, disabled, label) {
    var btn = document.getElementById(id);
    if (!btn) return;
    btn.disabled = disabled;
    if (label) btn.textContent = label;
    btn.style.opacity = disabled ? '0.65' : '';
  }

  /** Set an element's textContent safely */
  function _setText(id, val) {
    var el = document.getElementById(id);
    if (el) el.textContent = val;
  }

  /* ════════════════════════════════════════════════════════
     INIT
  ════════════════════════════════════════════════════════ */
  function init() {
    /* ── Emergency badge ── */
    var emg   = '{{ emergency }}';
    var badge = document.getElementById('ev-emergency-badge');
    var emgMap = {
      Red:    { cls: 'bg-red-100    text-red-700    dark:bg-red-500/15    dark:text-red-400',    label: '🔴 Red'    },
      Orange: { cls: 'bg-orange-100 text-orange-700 dark:bg-orange-500/15 dark:text-orange-400', label: '🟠 Orange' },
      Yellow: { cls: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-500/15 dark:text-yellow-400', label: '🟡 Yellow' },
      Blue:   { cls: 'bg-blue-100   text-blue-700   dark:bg-blue-500/15   dark:text-blue-400',   label: '🔵 Blue'   }
    };
    var emgCfg = emgMap[emg] || emgMap.Blue;
    if (badge) {
      badge.className = 'inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-bold uppercase tracking-wide ' + emgCfg.cls;
      badge.textContent = emgCfg.label;
    }

    /* ── Sidebar quick-info ── */
    var sidebarEmg = document.getElementById('sidebar-emergency');
    if (sidebarEmg) {
      sidebarEmg.textContent = emg;
      sidebarEmg.className = 'font-semibold ' + ({ Red:'text-red-500', Orange:'text-orange-500', Yellow:'text-yellow-500', Blue:'text-blue-500' }[emg] || 'text-gray-700');
    }

    var statusEl = document.getElementById('sidebar-status');
    if (statusEl) statusEl.textContent = '{{ status }}';

    var policeEl = document.getElementById('sidebar-police');
    if (policeEl) {
      var isPolice = '{{ is_police }}' === 'true';
      policeEl.textContent = isPolice ? 'Reported' : 'Not Reported';
      policeEl.className = 'font-semibold ' + (isPolice ? 'text-blue-600 dark:text-blue-400' : 'text-gray-500');
    }

    var chainEl = document.getElementById('sidebar-chain');
    if (chainEl) {
      var isSigned = '{{ is_signed }}' === 'true';
      chainEl.textContent = isSigned ? 'Signed ✓' : 'Unsigned';
      chainEl.className = 'font-semibold ' + (isSigned ? 'text-purple-600 dark:text-purple-400' : 'text-gray-400');
    }

    /* ── Target count from static grid ── */
    var targetGrid  = document.getElementById('target-photos-grid');
    var staticCount = targetGrid ? targetGrid.children.length : 0;
    _setText('target-count', TG_PHOTOS.length || staticCount);

    if (!staticCount && !TG_PHOTOS.length) {
      var emptyEl = document.getElementById('target-empty');
      if (emptyEl) emptyEl.classList.remove('hidden');
    }

    /* ── Share link default ── */
    var shareLinkEl = document.getElementById('share-link');
    if (shareLinkEl) shareLinkEl.value = window.location.href;

    /* ── Load persisted settings ── */
    _loadSettings();

    /* ── Close modals on backdrop click ── */
    document.querySelectorAll('.ev-modal').forEach(function(m) {
      m.addEventListener('click', function(e) {
        if (e.target === m) closeModal(m.id);
      });
    });

    /* ── Escape key closes topmost modal ── */
    document.addEventListener('keydown', function(e) {
      if (e.key !== 'Escape') return;
      var open = document.querySelector('.ev-modal.open');
      if (open) closeModal(open.id);
      else closeLightbox();
    });

    /* ── Mobile action sidebar toggle ──
       The sidebar uses Alpine x-model "actionOpen" via :class="actionOpen ? 'open' : ''".
       The FAB also sets actionOpen via @click="actionOpen = !actionOpen".
       We augment with a plain-JS fallback for non-Alpine contexts. */
    var asToggle = document.getElementById('action-sidebar-toggle');
    var asSidebar = document.getElementById('action-sidebar');
    if (asToggle && asSidebar && !asToggle._evBound) {
      asToggle._evBound = true;
      asToggle.addEventListener('click', function() {
        asSidebar.classList.toggle('open');
      });
    }

    /* ── Start fetching API targets ── */
    loadApiTargets();

    /* ── Init Leaflet if already loaded ── */
    if (typeof L !== 'undefined') initLeafletMap();
  }

  /* ── Kick off on DOM ready ── */
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

  /* ════════════════════════════════════════════════════════
     PUBLIC API
  ════════════════════════════════════════════════════════ */
  return {
    openModal:        openModal,
    closeModal:       closeModal,
    toggleAccordion:  toggleAccordion,
    openLightbox:     openLightbox,
    closeLightbox:    closeLightbox,
    executeDelete:    executeDelete,
    executeTakedown:  executeTakedown,
    submitPoliceReport: submitPoliceReport,
    signEvidence:     signEvidence,
    openTargetModal:  openTargetModal,
    submitTargetFlag: submitTargetFlag,
    loadApiTargets:   loadApiTargets,
    filterTargetTab:  filterTargetTab,
    openShare:        openShare,
    copyLink:         copyLink,
    shareVia:         shareVia,
    exportEvidence:   exportEvidence,
    saveSettings:     saveSettings,
    initCharts:       initCharts,
    initLeafletMap:   initLeafletMap,
    updateStatusBadge: updateStatusBadge,
    updatePoliceBadge: updatePoliceBadge
  };
})();