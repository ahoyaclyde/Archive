// ══════════════════════════════════════════════════════════════════════════
//  evidence_view.js  —  injected as {{ view_js }} into evidence_view.html
//
//  Changes from original:
//  • Removed duplicate `const targetPhotosData` declaration at bottom
//  • Renamed photo-viewer openTargetModal → openTargetPhotoModal
//  • Added SubjectSearch — live search with cross-case count dropdown
//  • Added PhotoPicker — thumbnail row linking flags to uploaded target photos
// ══════════════════════════════════════════════════════════════════════════

// ── Target photos (server-injected via <script id="targetPhotosData"> tag) ──
const targetPhotosData = JSON.parse(
    document.getElementById('targetPhotosData')?.textContent || '[]'
);
let currentTargetIndex = 0;

console.log('✅ Target photos loaded:', targetPhotosData.length);

// ── Helpers ───────────────────────────────────────────────────────────────

function formatBytes(bytes) {
    if (bytes < 1024)            return bytes + ' B';
    if (bytes < 1024 * 1024)     return (bytes / 1024).toFixed(2) + ' KB';
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(2) + ' MB';
    return (bytes / (1024 * 1024 * 1024)).toFixed(2) + ' GB';
}

function formatCategory(category) {
    return { person:'👤 Person', vehicle:'🚗 Vehicle', object:'📦 Object',
             location:'📍 Location', other:'❓ Other' }[category.toLowerCase()] || category;
}

function updateConfidenceColor(confidence) {
    const el = document.getElementById('targetModalConfidenceValue');
    if (!el) return;
    const cls = confidence >= 80 ? 'text-green-400'
              : confidence >= 60 ? 'text-yellow-400'
              : confidence >= 40 ? 'text-orange-400' : 'text-red-400';
    el.className = 'font-bold text-lg ' + cls;
}

// ══════════════════════════════════════════════════════════════════════════
//  TARGET PHOTO VIEWER  (renamed from openTargetModal to avoid collision
//  with EVC's flag-type modal which is also called openTargetModal)
// ══════════════════════════════════════════════════════════════════════════

function openTargetPhotoModal(targetIndex) {
    currentTargetIndex = targetIndex;
    const target = targetPhotosData[targetIndex];
    if (!target) { console.error('Target not found at index:', targetIndex); return; }

    const imageUrl = target.image_url;
    if (!imageUrl) { console.error('No image URL for target'); return; }

    document.getElementById('targetModalImage').src = imageUrl;
    document.getElementById('targetModalTitle').textContent = target.description || 'Unnamed Target';
    document.getElementById('targetModalIndex').textContent = 'Target #' + (targetIndex + 1);
    document.getElementById('targetModalCategory').textContent =
        target.category ? formatCategory(target.category) : 'Unknown';

    const confidence = target.confidence_score || target.confidence || 75;
    document.getElementById('targetModalConfidence').textContent = confidence + '% confidence';

    const descEl = document.getElementById('targetModalDescription');
    if (descEl) descEl.value = target.description || '';

    const catSel = document.getElementById('targetModalCategorySelect');
    if (catSel && target.category) catSel.value = target.category.toLowerCase();

    const slider = document.getElementById('targetModalConfidenceSlider');
    const valEl  = document.getElementById('targetModalConfidenceValue');
    if (slider && valEl) {
        slider.value = confidence;
        valEl.textContent = confidence + '%';
        updateConfidenceColor(confidence);
    }

    document.getElementById('targetModalFilename').textContent  = target.filename  || 'Unknown';
    document.getElementById('targetModalFilesize').textContent  = formatBytes(target.file_size || 0);
    document.getElementById('targetModalMimetype').textContent  = target.mime_type || 'Unknown';

    let createdText = 'Unknown';
    if (target.created_at) {
        try {
            const d = new Date(target.created_at);
            createdText = d.toLocaleDateString() + ' ' + d.toLocaleTimeString();
        } catch(e) { createdText = target.created_at; }
    }
    document.getElementById('targetModalCreated').textContent = createdText;

    const navDiv = document.getElementById('targetModalNavigation');
    if (navDiv) navDiv.classList.toggle('hidden', targetPhotosData.length <= 1);

    document.getElementById('targetModal').classList.remove('hidden');
    document.body.style.overflow = 'hidden';
}

function closeTargetModal() {
    const m = document.getElementById('targetModal');
    if (m) m.classList.add('hidden');
    document.body.style.overflow = 'auto';
}

function navigateTarget(direction) {
    let idx = currentTargetIndex + direction;
    if (idx < 0) idx = targetPhotosData.length - 1;
    else if (idx >= targetPhotosData.length) idx = 0;
    if (targetPhotosData[idx]) {
        closeTargetModal();
        setTimeout(() => openTargetPhotoModal(idx), 50);
    }
}

// ══════════════════════════════════════════════════════════════════════════
//  SUBJECT SEARCH  —  live search on the flag modal's name field
//
//  Attaches to #tm-name input (inside the EVC flag modal).
//  Calls GET /api/subjects/search?q=<query>
//  Shows a dropdown with existing subjects + cross-case count.
//  On selection populates #tm-subject-id (hidden) and shows a chip.
// ══════════════════════════════════════════════════════════════════════════

const SubjectSearch = (function () {
    let debounceTimer = null;
    let selectedSubjectId = null;

    function init() {
        const input = document.getElementById('tm-name');
        if (!input) return;

        input.addEventListener('input', function () {
            clearTimeout(debounceTimer);
            const q = this.value.trim();
            clearSelection(false);   // keep text, clear subject_id
            if (q.length < 2) { hideDropdown(); return; }
            debounceTimer = setTimeout(() => search(q), 280);
        });

        // Close dropdown when clicking outside
        document.addEventListener('click', function (e) {
            if (!e.target.closest('#tm-name') && !e.target.closest('#tm-subject-results')) {
                hideDropdown();
            }
        });
    }

    function search(q) {
        const evId = typeof EV_ID !== 'undefined' ? EV_ID : '';
        fetch('/api/subjects/search?q=' + encodeURIComponent(q) + '&limit=6', {
            credentials: 'include'
        })
        .then(r => r.json())
        .then(j => showDropdown(j.data || []))
        .catch(() => hideDropdown());
    }

    function showDropdown(subjects) {
        const container = document.getElementById('tm-subject-results');
        if (!container) return;

        if (!subjects.length) { hideDropdown(); return; }

        container.innerHTML = subjects.map(s => {
            const flagBadges = (s.flag_types || []).slice(0, 3).map(ft =>
                `<span class="inline-block rounded px-1 py-0.5 text-[10px] font-semibold
                             ${ft === 'poi' ? 'bg-orange-100 text-orange-600' :
                               ft === 'wanted' ? 'bg-red-100 text-red-600' :
                               ft === 'watchlist' ? 'bg-purple-100 text-purple-600' :
                               'bg-blue-100 text-blue-600'}">${ft.toUpperCase()}</span>`
            ).join(' ');

            const caseWord = s.appearance_count === 1 ? 'case' : 'cases';

            return `<button type="button"
                            class="w-full text-left px-3 py-2.5 hover:bg-gray-50 dark:hover:bg-white/5
                                   border-b border-gray-100 dark:border-gray-800 last:border-0
                                   flex items-center gap-2.5 transition-colors"
                            onclick="SubjectSearch.select(${JSON.stringify(s)})">
                <div class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-full
                            bg-gray-100 dark:bg-gray-800 text-gray-600 dark:text-gray-300
                            text-xs font-bold">${escHtml((s.name || '?')[0].toUpperCase())}</div>
                <div class="flex-1 min-w-0">
                    <p class="text-sm font-semibold text-gray-800 dark:text-white/90 truncate">${escHtml(s.name)}</p>
                    <p class="text-xs text-gray-500 mt-0.5">
                        ${escHtml(s.category || 'person')} ·
                        <span class="font-medium text-gray-700 dark:text-gray-300">${s.appearance_count} ${caseWord}</span>
                        ${flagBadges ? '· ' + flagBadges : ''}
                    </p>
                </div>
            </button>`;
        }).join('');

        container.classList.remove('hidden');
    }

    function hideDropdown() {
        const container = document.getElementById('tm-subject-results');
        if (container) container.classList.add('hidden');
    }

    function select(subject) {
        selectedSubjectId = subject.id;

        // Fill hidden field
        const hiddenInput = document.getElementById('tm-subject-id');
        if (hiddenInput) hiddenInput.value = subject.id;

        // Fill name field and lock it
        const nameInput = document.getElementById('tm-name');
        if (nameInput) nameInput.value = subject.name;

        // Fill category
        const catSel = document.getElementById('tm-category');
        if (catSel && subject.category) catSel.value = subject.category;

        // Fill description if empty
        const descEl = document.getElementById('tm-desc');
        if (descEl && !descEl.value && subject.description) {
            descEl.value = subject.description;
        }

        // Fill location if empty
        const locEl = document.getElementById('tm-location');
        if (locEl && !locEl.value && subject.last_known_location) {
            locEl.value = subject.last_known_location;
        }

        // Show selected chip
        const chip = document.getElementById('tm-selected-subject');
        if (chip) {
            const caseWord = subject.appearance_count === 1 ? 'case' : 'cases';
            chip.classList.remove('hidden');
            chip.querySelector('#tm-chip-name').textContent = subject.name;
            chip.querySelector('#tm-chip-cases').textContent =
                `Flagged in ${subject.appearance_count} ${caseWord}`;
        }

        hideDropdown();
        console.log('👤 Subject selected:', subject.name, 'id:', subject.id);
    }

    function clearSelection(clearText = true) {
        selectedSubjectId = null;

        const hiddenInput = document.getElementById('tm-subject-id');
        if (hiddenInput) hiddenInput.value = '';

        if (clearText) {
            const nameInput = document.getElementById('tm-name');
            if (nameInput) nameInput.value = '';
        }

        const chip = document.getElementById('tm-selected-subject');
        if (chip) chip.classList.add('hidden');

        hideDropdown();
    }

    function getSelectedId() { return selectedSubjectId; }

    function reset() { clearSelection(true); }

    return { init, select, clearSelection, getSelectedId, reset };
})();

// ══════════════════════════════════════════════════════════════════════════
//  PHOTO PICKER  —  thumbnail row inside the flag modal
//
//  Renders thumbnails from targetPhotosData (already loaded from server).
//  Clicking a thumbnail sets #tm-target-photo-id (hidden).
// ══════════════════════════════════════════════════════════════════════════

const PhotoPicker = (function () {
    let selectedPhotoId = null;

    function init() {
        const section = document.getElementById('tm-photo-picker');
        if (!section) return;

        if (!targetPhotosData.length) {
            section.classList.add('hidden');
            return;
        }

        const row = document.getElementById('tm-photo-thumbnails');
        if (!row) return;

        row.innerHTML = targetPhotosData.map((t, i) =>
            `<button type="button"
                     id="tm-photo-thumb-${i}"
                     class="flex-shrink-0 h-14 w-14 rounded-lg overflow-hidden border-2 border-transparent
                            hover:border-blue-400 transition-all focus:outline-none"
                     onclick="PhotoPicker.select('${escHtml(t.id || '')}', ${i})"
                     title="${escHtml(t.description || 'Target ' + (i + 1))}">
                <img src="${escHtml(t.image_url || '')}"
                     class="h-full w-full object-cover"
                     onerror="this.parentElement.classList.add('hidden')">
             </button>`
        ).join('');

        section.classList.remove('hidden');
    }

    function select(photoId, index) {
        selectedPhotoId = photoId;

        const hiddenInput = document.getElementById('tm-target-photo-id');
        if (hiddenInput) hiddenInput.value = photoId;

        // Highlight selected
        targetPhotosData.forEach((_, i) => {
            const btn = document.getElementById('tm-photo-thumb-' + i);
            if (!btn) return;
            btn.classList.toggle('border-blue-500', i === index);
            btn.classList.toggle('border-transparent', i !== index);
            btn.classList.toggle('ring-2', i === index);
            btn.classList.toggle('ring-blue-400', i === index);
        });

        // Show label
        const label = document.getElementById('tm-photo-selected-label');
        if (label) {
            const t = targetPhotosData[index];
            label.textContent = 'Linked: ' + (t?.description || 'Target ' + (index + 1));
            label.classList.remove('hidden');
        }

        console.log('📸 Photo linked:', photoId);
    }

    function deselect() {
        selectedPhotoId = null;
        const hiddenInput = document.getElementById('tm-target-photo-id');
        if (hiddenInput) hiddenInput.value = '';

        targetPhotosData.forEach((_, i) => {
            const btn = document.getElementById('tm-photo-thumb-' + i);
            if (btn) {
                btn.classList.remove('border-blue-500', 'ring-2', 'ring-blue-400');
                btn.classList.add('border-transparent');
            }
        });

        const label = document.getElementById('tm-photo-selected-label');
        if (label) label.classList.add('hidden');

        selectedPhotoId = null;
    }

    function getSelectedId() { return selectedPhotoId; }

    function reset() { deselect(); }

    return { init, select, deselect, getSelectedId, reset };
})();

// ══════════════════════════════════════════════════════════════════════════
//  WALLET MODAL FUNCTIONS
// ══════════════════════════════════════════════════════════════════════════

function showWalletSignModal() {
    document.getElementById('walletSignModal')?.classList.remove('hidden');
    document.getElementById('signStep1')?.classList.remove('hidden');
    document.getElementById('signStep2')?.classList.add('hidden');
    document.body.style.overflow = 'hidden';
}

function closeWalletModal() {
    document.getElementById('walletSignModal')?.classList.add('hidden');
    document.body.style.overflow = 'auto';
}

// ══════════════════════════════════════════════════════════════════════════
//  POLICE MODAL FUNCTIONS
// ══════════════════════════════════════════════════════════════════════════

function showPoliceCaseModal() {
    document.getElementById('policeCaseModal')?.classList.remove('hidden');
    document.getElementById('policeStep1')?.classList.remove('hidden');
    document.getElementById('policeStep2')?.classList.add('hidden');
    document.body.style.overflow = 'hidden';
}

function closePoliceModal() {
    document.getElementById('policeCaseModal')?.classList.add('hidden');
    document.body.style.overflow = 'auto';
}

// ══════════════════════════════════════════════════════════════════════════
//  MEDIA MODAL HELPER
// ══════════════════════════════════════════════════════════════════════════

function openMediaModal(url, type) {
    const modal = document.createElement('div');
    modal.className = 'fixed inset-0 bg-black/90 z-50 flex items-center justify-center p-4';
    modal.innerHTML = type === 'image'
        ? `<div class="relative max-w-4xl max-h-full">
               <button onclick="this.parentElement.parentElement.remove()"
                       class="absolute top-4 right-4 text-white text-2xl z-10 hover:text-gray-300">
                   <i class="fas fa-times"></i></button>
               <img src="${url}" class="w-full h-auto max-h-[80vh] object-contain rounded-lg">
           </div>`
        : `<div class="relative max-w-4xl w-full">
               <button onclick="this.parentElement.parentElement.remove()"
                       class="absolute top-4 right-4 text-white text-2xl z-10 hover:text-gray-300">
                   <i class="fas fa-times"></i></button>
               <video controls class="w-full h-auto max-h-[80vh]" autoplay>
                   <source src="${url}" type="video/mp4">
               </video>
           </div>`;
    document.body.appendChild(modal);
}

function downloadFile(url, filename) {
    const link = document.createElement('a');
    link.href = url; link.download = filename;
    document.body.appendChild(link); link.click(); document.body.removeChild(link);
}

function closeAndRefresh() {
    closePoliceModal();
    setTimeout(() => window.location.reload(), 300);
}

// ══════════════════════════════════════════════════════════════════════════
//  PRODUCTION BLOCKCHAIN SIGNING
// ══════════════════════════════════════════════════════════════════════════

class ProductionEvidenceSigner {
    constructor(evidenceId, evidenceNumber, title) {
        this.evidenceId     = evidenceId;
        this.evidenceNumber = evidenceNumber;
        this.title          = title;
        this.walletAddress  = null;
        this.chain          = 'ethereum';
    }

    async connectWallet() {
        if (typeof window.ethereum === 'undefined')
            throw new Error('MetaMask is not installed.');
        const accounts = await window.ethereum.request({ method: 'eth_accounts' });
        if (accounts.length > 0) { this.walletAddress = accounts[0]; return this.walletAddress; }
        const newAccounts = await window.ethereum.request({ method: 'eth_requestAccounts' });
        if (!newAccounts.length) throw new Error('No accounts found.');
        this.walletAddress = newAccounts[0];
        return this.walletAddress;
    }

    async getSigningMessage() {
        const res = await fetch(`/evidence/${this.evidenceId}/sign/request`, { credentials: 'include' });
        if (!res.ok) throw new Error(`Server ${res.status}`);
        const result = await res.json();
        if (!result.success) throw new Error(result.message || 'Failed to get signing message');
        return result.data;
    }

    async signMessageWithWallet(message) {
        if (!this.walletAddress) throw new Error('Wallet not connected.');
        const sig = await window.ethereum.request({
            method: 'personal_sign',
            params: [message, this.walletAddress],
        });
        return sig;
    }

    async submitSignatureToServer(signature) {
        const res = await fetch(`/evidence/${this.evidenceId}/sign/production`, {
            method: 'POST',
            credentials: 'include',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                evidence_id: this.evidenceId,
                wallet_address: this.walletAddress,
                chain: this.chain,
                signature,
                message: 'Production evidence signing completed',
            }),
        });
        if (!res.ok) throw new Error(`Server ${res.status}`);
        const result = await res.json();
        if (!result.success) throw new Error(result.message || 'Server rejected signature');
        return result.data;
    }

    async signEvidence() {
        const signBtn = document.getElementById('signWalletBtn');
        const originalHtml = signBtn?.innerHTML;
        try {
            if (signBtn) { signBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Connecting…'; signBtn.disabled = true; }
            await this.connectWallet();
            if (signBtn) signBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Preparing…';
            const msg = await this.getSigningMessage();
            if (signBtn) signBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Sign in Wallet…';
            const sig = await this.signMessageWithWallet(msg);
            if (signBtn) signBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Verifying…';
            const result = await this.submitSignatureToServer(sig);
            if (signBtn) {
                signBtn.innerHTML = '<i class="fas fa-check mr-2"></i> Signed!';
                signBtn.classList.replace('from-yellow-600', 'from-green-600');
                signBtn.classList.replace('to-orange-600', 'to-green-700');
                signBtn.disabled = true;
            }
            return { success: true, signature: result?.signature, transactionHash: result?.transaction_id, walletAddress: this.walletAddress, timestamp: result?.timestamp };
        } catch (error) {
            if (signBtn) { signBtn.innerHTML = originalHtml; signBtn.disabled = false; }
            alert('Signing failed: ' + error.message);
            return { success: false, error: error.message };
        }
    }
}

let productionSigner = null;

function initProductionSigner() {
    const evidenceId     = document.getElementById('evidenceId')?.value || '{{ evidence_id }}';
    const evidenceNumber = document.getElementById('evidenceNumber')?.textContent || '';
    const title          = document.getElementById('evidenceTitle')?.textContent || '';
    productionSigner = new ProductionEvidenceSigner(evidenceId, evidenceNumber, title);
}

async function performProductionSign() {
    if (!productionSigner) { alert('Signer not ready. Refresh the page.'); return; }
    const result = await productionSigner.signEvidence();
    if (result.success) {
        document.getElementById('signStep1')?.classList.add('hidden');
        document.getElementById('signStep2')?.classList.remove('hidden');
        const txEl = document.getElementById('transactionHash');
        if (txEl) txEl.textContent = result.transactionHash || 'On-chain verification pending';
        updateEvidenceSignedStatus(result.walletAddress, result.timestamp);
    }
}

function updateEvidenceSignedStatus(walletAddress, timestamp) {
    document.querySelectorAll('[onclick*="showWalletSignModal"]').forEach(btn => {
        btn.innerHTML = '<i class="fas fa-check-circle mr-2"></i> Signed';
        btn.disabled  = true;
        btn.classList.remove('bg-yellow-600', 'hover:bg-yellow-700');
        btn.classList.add('bg-green-600', 'cursor-default');
    });
}

// ══════════════════════════════════════════════════════════════════════════
//  ESCAPE HELPER  (used by SubjectSearch dropdown HTML builder)
// ══════════════════════════════════════════════════════════════════════════

function escHtml(str) {
    if (!str) return '';
    return String(str)
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;');
}

// ══════════════════════════════════════════════════════════════════════════
//  KEYBOARD & OUTSIDE-CLICK HANDLERS
// ══════════════════════════════════════════════════════════════════════════

document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
        closeWalletModal();
        closePoliceModal();
        closeTargetModal();
    }
});

document.addEventListener('click', (e) => {
    if (e.target.classList.contains('modal-overlay')) {
        closeWalletModal();
        closePoliceModal();
        closeTargetModal();
    }
});

// ══════════════════════════════════════════════════════════════════════════
//  INIT
// ══════════════════════════════════════════════════════════════════════════

document.addEventListener('DOMContentLoaded', function () {
    // Confidence slider in photo viewer modal
    const slider = document.getElementById('targetModalConfidenceSlider');
    if (slider) {
        slider.addEventListener('input', function () {
            const valEl = document.getElementById('targetModalConfidenceValue');
            if (valEl) valEl.textContent = this.value + '%';
            updateConfidenceColor(Number(this.value));
        });
    }

    // Production signer
    initProductionSigner();

    // Wire wallet sign button
    const walletSignBtn = document.getElementById('signWalletBtn');
    if (walletSignBtn) walletSignBtn.onclick = performProductionSign;

    // Subject search — attaches to EVC's flag modal name field
    SubjectSearch.init();

    // Photo picker — renders thumbnails in the flag modal
    PhotoPicker.init();

    console.log('✅ evidence_view.js initialised | photos:', targetPhotosData.length);
});