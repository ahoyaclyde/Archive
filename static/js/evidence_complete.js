// Evidence Completion Form JavaScript
document.addEventListener('DOMContentLoaded', function() {
    console.log('📝 Evidence completion page loaded');

    // DOM Elements
    const form = document.getElementById('completeForm');
    const submitButton = document.getElementById('submitButton');
    const loadingOverlay = document.getElementById('loadingOverlay');
    const getLocationBtn = document.getElementById('getLocationBtn');
    const locationStatus = document.getElementById('locationStatus');
    const latitudeInput = document.querySelector('input[name="latitude"]');
    const longitudeInput = document.querySelector('input[name="longitude"]');
    const policeCheckbox = document.getElementById('policeReportCheckbox');
    const policeDetails = document.getElementById('policeDetails');
    const incidentTypeSelect = document.querySelector('select[name="incident_type"]');
    const vehicleSection = document.getElementById('vehicleSection');
    const descriptionTextarea = document.querySelector('textarea[name="description"]');

    // Initialize
    toggleVehicleSection();
    
    // Toggle police report details
    if (policeCheckbox) {
        policeCheckbox.addEventListener('change', function() {
            policeDetails.classList.toggle('hidden', !this.checked);
            if (this.checked) {
                policeDetails.querySelector('input').focus();
            }
        });
        
        // Show police details if already checked
        if (policeCheckbox.checked) {
            policeDetails.classList.remove('hidden');
        }
    }

    // Toggle vehicle section based on incident type
    if (incidentTypeSelect) {
        incidentTypeSelect.addEventListener('change', toggleVehicleSection);
    }
    
    function toggleVehicleSection() {
        if (!incidentTypeSelect || !vehicleSection) return;
        
        const selectedValue = incidentTypeSelect.value;
        const showVehicle = ['HitAndRun', 'Assault', 'Theft', 'PropertyDamage'].includes(selectedValue);
        
        vehicleSection.classList.toggle('hidden', !showVehicle);
        
        // If showing vehicle section, make required fields optional
        if (showVehicle) {
            const vehicleInputs = vehicleSection.querySelectorAll('input, select');
            vehicleInputs.forEach(input => {
                input.required = false;
            });
        }
    }

    // Get current location
    if (getLocationBtn) {
        getLocationBtn.addEventListener('click', function() {
            if (!navigator.geolocation) {
                showLocationStatus('Geolocation is not supported by your browser', 'error');
                return;
            }
            
            showLocationStatus('Getting location...', 'loading');
            
            navigator.geolocation.getCurrentPosition(
                // Success callback
                function(position) {
                    const lat = position.coords.latitude;
                    const lng = position.coords.longitude;
                    
                    // Set values in inputs
                    if (latitudeInput) latitudeInput.value = lat.toFixed(6);
                    if (longitudeInput) longitudeInput.value = lng.toFixed(6);
                    
                    // Update status
                    showLocationStatus(`Location found: ${lat.toFixed(4)}, ${lng.toFixed(4)}`, 'success');
                    
                    // Auto-fill county based on coordinates
                    autoFillCountyFromCoordinates(lat, lng);
                },
                // Error callback
                function(error) {
                    let message = 'Unable to get location: ';
                    switch(error.code) {
                        case error.PERMISSION_DENIED:
                            message += 'Permission denied. Please enable location services.';
                            break;
                        case error.POSITION_UNAVAILABLE:
                            message += 'Location information unavailable.';
                            break;
                        case error.TIMEOUT:
                            message += 'Location request timed out.';
                            break;
                        default:
                            message += 'Unknown error.';
                    }
                    showLocationStatus(message, 'error');
                },
                // Options
                {
                    enableHighAccuracy: true,
                    timeout: 10000,
                    maximumAge: 0
                }
            );
        });
    }

    // Show location status with appropriate styling
    function showLocationStatus(message, type) {
        if (!locationStatus) return;
        
        locationStatus.textContent = message;
        locationStatus.className = 'ml-4 text-sm';
        
        switch(type) {
            case 'success':
                locationStatus.classList.add('text-green-600');
                break;
            case 'error':
                locationStatus.classList.add('text-red-600');
                break;
            case 'loading':
                locationStatus.classList.add('text-yellow-600');
                break;
            default:
                locationStatus.classList.add('text-gray-500');
        }
    }

    // Auto-fill county based on coordinates (simplified)
    function autoFillCountyFromCoordinates(lat, lng) {
        const countySelect = document.querySelector('select[name="county"]');
        if (!countySelect) return;
        
        // This is a very basic implementation
        // In production, you would use a reverse geocoding service
        
        // Nairobi approximate bounds
        if (lat > -1.6 && lat < -1.0 && lng > 36.6 && lng < 37.0) {
            countySelect.value = 'Nairobi';
        }
        // Mombasa approximate bounds
        else if (lat > -4.2 && lat < -3.8 && lng > 39.5 && lng < 40.0) {
            countySelect.value = 'Mombasa';
        }
        // Kisumu approximate bounds
        else if (lat > -0.2 && lat < 0.1 && lng > 34.6 && lng < 34.9) {
            countySelect.value = 'Kisumu';
        }
        // Nakuru approximate bounds
        else if (lat > -0.5 && lat < -0.1 && lng > 36.0 && lng < 36.3) {
            countySelect.value = 'Nakuru';
        }
        
        // Trigger change event to update dependent fields
        countySelect.dispatchEvent(new Event('change'));
    }

    // Auto-detect incident type from description
    if (descriptionTextarea && incidentTypeSelect) {
        descriptionTextarea.addEventListener('blur', function() {
            const text = this.value.toLowerCase();
            
            if (text.includes('hit') && text.includes('run')) {
                incidentTypeSelect.value = 'HitAndRun';
            } else if (text.includes('assault') || text.includes('attack') || text.includes('beat')) {
                incidentTypeSelect.value = 'Assault';
            } else if (text.includes('threat') || text.includes('kill') || text.includes('murder')) {
                incidentTypeSelect.value = 'ThreatToLife';
            } else if (text.includes('damage') || text.includes('destroy') || text.includes('vandal')) {
                incidentTypeSelect.value = 'PropertyDamage';
            } else if (text.includes('theft') || text.includes('steal') || text.includes('rob')) {
                incidentTypeSelect.value = 'Theft';
            }
            
            // Update vehicle section visibility
            toggleVehicleSection();
        });
    }

    // Form validation
    function validateForm() {
        // Required fields
        const requiredFields = form.querySelectorAll('[required]');
        let isValid = true;
        
        for (const field of requiredFields) {
            if (!field.value.trim()) {
                field.classList.add('border-red-500');
                isValid = false;
                
                // Show error message
                let errorMsg = field.parentElement.querySelector('.error-message');
                if (!errorMsg) {
                    errorMsg = document.createElement('div');
                    errorMsg.className = 'error-message text-red-600 text-sm mt-1';
                    errorMsg.textContent = 'This field is required';
                    field.parentElement.appendChild(errorMsg);
                }
            } else {
                field.classList.remove('border-red-500');
                
                // Remove error message
                const errorMsg = field.parentElement.querySelector('.error-message');
                if (errorMsg) {
                    errorMsg.remove();
                }
            }
        }
        
        // Validate date
        const incidentDate = document.querySelector('input[name="incident_date"]');
        if (incidentDate && incidentDate.value) {
            const selectedDate = new Date(incidentDate.value);
            const today = new Date();
            today.setHours(0, 0, 0, 0);
            
            if (selectedDate > today) {
                alert('Incident date cannot be in the future');
                incidentDate.classList.add('border-red-500');
                isValid = false;
            }
        }
        
        // Validate coordinates
        if (latitudeInput && longitudeInput && 
            (latitudeInput.value || longitudeInput.value)) {
            const lat = parseFloat(latitudeInput.value);
            const lng = parseFloat(longitudeInput.value);
            
            if (isNaN(lat) || isNaN(lng)) {
                alert('Please enter valid coordinates');
                isValid = false;
            } else if (lat < -4.9 || lat > 5.0 || lng < 33.0 || lng > 42.0) {
                if (!confirm('Coordinates appear to be outside Kenya. Are you sure?')) {
                    isValid = false;
                }
            }
        }
        
        return isValid;
    }

    // Form submission
    if (form) {
        form.addEventListener('submit', async function(e) {
            e.preventDefault();
            
            if (!validateForm()) {
                alert('Please fill all required fields correctly');
                return;
            }
            
            // Show loading
            if (loadingOverlay) loadingOverlay.classList.remove('hidden');
            if (submitButton) submitButton.disabled = true;
            
            try {
                const formData = new FormData(form);
                
                // Add timestamp for incident datetime
                const date = formData.get('incident_date');
                const time = formData.get('incident_time') || '12:00';
                if (date) {
                    formData.set('incident_datetime', `${date}T${time}:00`);
                }
                
                const response = await fetch(form.action, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/x-www-form-urlencoded',
                    },
                    body: new URLSearchParams(formData)
                });
                
                const result = await response.json();
                
                if (result.success) {
                    // Redirect to evidence view page
                    window.location.href = result.data.redirect;
                } else {
                    throw new Error(result.message || 'Submission failed');
                }
            } catch (error) {
                console.error('Error:', error);
                alert('Error: ' + error.message);
                
                // Reset UI
                if (loadingOverlay) loadingOverlay.classList.add('hidden');
                if (submitButton) submitButton.disabled = false;
            }
        });
    }

    // Real-time form validation
    const inputs = form.querySelectorAll('input, select, textarea');
    inputs.forEach(input => {
        input.addEventListener('blur', function() {
            if (this.hasAttribute('required') && !this.value.trim()) {
                this.classList.add('border-red-500');
            } else {
                this.classList.remove('border-red-500');
            }
        });
        
        input.addEventListener('input', function() {
            this.classList.remove('border-red-500');
        });
    });

    // County change event - could load constituencies/wards
    const countySelect = document.querySelector('select[name="county"]');
    if (countySelect) {
        countySelect.addEventListener('change', function() {
            const county = this.value;
            
            // In a real app, you might load constituencies/wards for this county
            // from an API
            console.log('County selected:', county);
            
            // Clear constituency and ward if county changes
            const constituencyInput = document.querySelector('input[name="constituency"]');
            const wardInput = document.querySelector('input[name="ward"]');
            
            if (constituencyInput && !constituencyInput.value) {
                // Could auto-suggest based on county
            }
        });
    }

    // Emergency level color coding
    const emergencyLevelRadios = document.querySelectorAll('input[name="emergency_level"]');
    emergencyLevelRadios.forEach(radio => {
        radio.addEventListener('change', function() {
            // Update form header color based on emergency level
            const formHeader = document.querySelector('.bg-gray-800.rounded-lg');
            if (formHeader) {
                // Remove all emergency color classes
                formHeader.classList.remove(
                    'border-red-700', 'border-orange-700', 'border-yellow-700', 'border-blue-700'
                );
                
                // Add appropriate border color
                switch(this.value) {
                    case 'red':
                        formHeader.classList.add('border-red-700');
                        break;
                    case 'orange':
                        formHeader.classList.add('border-orange-700');
                        break;
                    case 'yellow':
                        formHeader.classList.add('border-yellow-700');
                        break;
                    case 'blue':
                        formHeader.classList.add('border-blue-700');
                        break;
                }
            }
        });
    });

    // Character counters for text areas
    const textAreas = form.querySelectorAll('textarea');
    textAreas.forEach(textarea => {
        const maxLength = textarea.getAttribute('maxlength') || 1000;
        
        // Create counter element
        const counter = document.createElement('div');
        counter.className = 'text-xs text-gray-500 text-right mt-1';
        counter.textContent = `0/${maxLength}`;
        textarea.parentElement.appendChild(counter);
        
        // Update counter on input
        textarea.addEventListener('input', function() {
            const length = this.value.length;
            counter.textContent = `${length}/${maxLength}`;
            
            if (length > maxLength * 0.9) {
                counter.classList.add('text-yellow-600');
                counter.classList.remove('text-red-600');
            } else {
                counter.classList.remove('text-yellow-600');
                counter.classList.remove('text-red-600');
            }
            
            if (length > maxLength) {
                counter.classList.add('text-red-600');
                this.value = this.value.substring(0, maxLength);
            } else {
                counter.classList.remove('text-red-600');
            }
        });
        
        // Initial update
        textarea.dispatchEvent(new Event('input'));
    });

    // Auto-save draft (every 30 seconds)
    let autoSaveInterval;
    function startAutoSave() {
        autoSaveInterval = setInterval(() => {
            saveDraft();
        }, 30000); // 30 seconds
    }
    
    function saveDraft() {
        // In a real implementation, you would save to localStorage
        // or send to server for draft preservation
        console.log('Auto-saving draft...');
    }
    
    function stopAutoSave() {
        if (autoSaveInterval) {
            clearInterval(autoSaveInterval);
        }
    }
    
    // Start auto-save when page loads
    startAutoSave();
    
    // Stop auto-save when form submits
    form.addEventListener('submit', stopAutoSave);
    
    // Warn before leaving if form has changes
    let formChanged = false;
    inputs.forEach(input => {
        input.addEventListener('input', () => {
            formChanged = true;
        });
    });
    
    window.addEventListener('beforeunload', (e) => {
        if (formChanged) {
            e.preventDefault();
            e.returnValue = 'You have unsaved changes. Are you sure you want to leave?';
        }
    });
});


// ==================== TARGET PHOTOS FUNCTIONS ====================

/* ══════════════════════════════════════════════════════════════════════════
   TARGET PHOTOS — READ-ONLY VIEWER
   Targets were extracted and uploaded on the capture page.
   Here we just fetch their Storj URLs from the API and render a grid.
══════════════════════════════════════════════════════════════════════════ */

/**
 * Fetch target photos for this evidence record and render a read-only grid.
 * Called once on DOMContentLoaded (below).
 * @param {string} evidenceId
 */
async function loadTargetPhotos(evidenceId) {
    const container = document.getElementById('targetsViewGrid');
    const loader    = document.getElementById('targetsViewLoader');
    const emptyMsg  = document.getElementById('targetsViewEmpty');
    if (!container) return;

    try {
        const res = await fetch(`/api/evidence/${evidenceId}/targets`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        const data = await res.json();

        if (loader) loader.classList.add('hidden');

        const photos = data.targets || data.photos || data.data || [];
        if (photos.length === 0) {
            if (emptyMsg) emptyMsg.classList.remove('hidden');
            return;
        }

        container.innerHTML = '';

        photos.forEach((photo, idx) => {
            const url  = photo.storj_url || photo.url || photo.file_url || '';
            const desc = photo.description || `Target ${idx + 1}`;
            const conf = photo.confidence_score || photo.confidence || '—';

            const card = document.createElement('div');
            card.style.cssText = 'position:relative;border-radius:.625rem;overflow:hidden;border:1.5px solid #e5e7eb;background:#fff;box-shadow:0 1px 3px rgba(0,0,0,.06);transition:box-shadow .15s,transform .15s;cursor:pointer;';
            card.onmouseover = function(){ this.style.transform='scale(1.03)'; this.style.boxShadow='0 6px 18px rgba(0,0,0,.10)'; };
            card.onmouseout  = function(){ this.style.transform=''; this.style.boxShadow='0 1px 3px rgba(0,0,0,.06)'; };
            card.innerHTML = `
                <div style="aspect-ratio:16/9;background:#f2f3f6;display:flex;align-items:center;justify-content:center;overflow:hidden;">
                    ${url
                        ? `<img src="${url}" alt="${desc}"
                               style="width:100%;height:100%;object-fit:cover;display:block;transition:transform .2s;"
                               loading="lazy"
                               onmouseover="this.style.transform='scale(1.06)'" onmouseout="this.style.transform=''"
                               onerror="this.parentElement.innerHTML='<span style=\\'color:#9ca3af;font-size:.75rem;\\'>Failed to load</span>'">`
                        : `<span style="color:#9ca3af;font-size:.75rem;">No image</span>`
                    }
                </div>
                <div style="padding:.5rem .75rem;background:#fff;border-top:1px solid #f3f4f6;">
                    <p style="font-size:.75rem;font-weight:600;color:#111827;margin:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:'DM Sans',sans-serif;" title="${desc}">${desc}</p>
                    <p style="font-size:.65rem;color:#9ca3af;margin:.2rem 0 0;">
                        Confidence: <span style="color:#dc2626;font-weight:700;">${conf}%</span>
                    </p>
                </div>
                <div style="position:absolute;top:5px;left:6px;font-size:.6rem;font-weight:700;color:rgba(255,255,255,.9);background:rgba(0,0,0,.5);border-radius:3px;padding:1px 5px;backdrop-filter:blur(2px);">T${idx + 1}</div>
                <a href="${url}" target="_blank" rel="noopener"
                   style="position:absolute;inset:0;opacity:0;" aria-label="Open ${desc}"></a>`;
            container.appendChild(card);
        });

        // Show count badge
        const badge = document.getElementById('targetsViewCount');
        if (badge) {
            badge.textContent = `${photos.length} target${photos.length !== 1 ? 's' : ''} extracted`;
            badge.classList.remove('hidden');
        }

        console.log(`✅ Loaded ${photos.length} target photo(s) from Storj`);

    } catch (err) {
        if (loader) loader.classList.add('hidden');
        if (emptyMsg) {
            emptyMsg.textContent = '⚠️ Could not load target photos.';
            emptyMsg.classList.remove('hidden');
        }
        console.warn('loadTargetPhotos failed:', err.message);
    }
}

// ── Utility ──────────────────────────────────────────────────────────────────
function formatBytes(bytes) {
    if (bytes < 1024) return bytes + ' B';
    else if (bytes < 1048576) return (bytes / 1024).toFixed(2) + ' KB';
    else return (bytes / 1048576).toFixed(2) + ' MB';
}

// ==================== COMPLETE EVIDENCE FUNCTION ====================

async function completeEvidence(event) {
    event.preventDefault();
    
    console.log('=== COMPLETE EVIDENCE PROCESS STARTED ===');
    
    const form = document.getElementById('completeForm');
    const formData = new FormData(form);
    const evidenceId = formData.get('evidence_id');
    
    console.log(`Evidence ID: ${evidenceId}`);
    
    // Validate main form
    if (!form.checkValidity()) {
        alert('Please fill in all required fields in the main form.');
        form.reportValidity();
        return;
    }
    
    // Update submit button
    const submitBtn = document.getElementById('submitBtn');
    if (!submitBtn) { console.error('Submit button not found'); return; }
    const originalBtnText = submitBtn.innerHTML;
    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Submitting Evidence...';
    
    try {
        console.log('Submitting evidence form...');
        
        const response = await fetch('/api/evidence/complete', {
            method: 'POST',
            body: formData
        });
        
        if (!response.ok) {
            throw new Error(`Server error: ${response.status} ${response.statusText}`);
        }
        
        const result = await response.json();
        console.log('Evidence response:', result);
        
        if (!result.success) {
            throw new Error(result.message || 'Evidence submission failed');
        }
        
        console.log('✅ Evidence submitted successfully');
        
        // Show success message
        const successHtml = `
        <div style="border-radius:1rem;border:1.5px solid #e5e7eb;background:#fff;overflow:hidden;box-shadow:0 4px 18px rgba(0,0,0,.06);font-family:'DM Sans',sans-serif;">

            <div style="display:flex;align-items:center;gap:.875rem;padding:1.25rem 1.5rem;border-bottom:1.5px solid #f3f4f6;">
                <div style="width:44px;height:44px;border-radius:.875rem;background:#f0fdf4;border:1.5px solid #bbf7d0;display:flex;align-items:center;justify-content:center;flex-shrink:0;">
                    <i class="fas fa-check" style="color:#16a34a;font-size:1.125rem"></i>
                </div>
                <div style="flex:1;min-width:0;">
                    <p style="font-size:.9375rem;font-weight:700;color:#111827;margin:0;">Evidence Submitted Successfully!</p>
                    <p style="font-size:.75rem;color:#6b7280;margin:.2rem 0 0;">Evidence Number: ${result.data.evidence_number}</p>
                    <p style="font-size:.7rem;color:#9ca3af;margin:.15rem 0 0;">Status: ${result.data.status || 'Submitted'}</p>
                </div>
                <span style="padding:.3rem .875rem;border-radius:9999px;background:#f0fdf4;border:1px solid #bbf7d0;color:#15803d;font-size:.7rem;font-weight:700;text-transform:uppercase;letter-spacing:.06em;white-space:nowrap;flex-shrink:0;">Published</span>
            </div>

            <div style="display:grid;grid-template-columns:repeat(3,1fr);border-bottom:1.5px solid #f3f4f6;">
                ${[
                    ['Title',      result.data.title],
                    ['Location',   result.data.location?.county || 'Unknown'],
                    ['Submitted',  new Date().toLocaleDateString('en-KE', { day:'2-digit', month:'short', year:'numeric' })],
                ].map(([k, v], i) => `
                    <div style="padding:.875rem 1rem;${i < 2 ? 'border-right:1.5px solid #f3f4f6;' : ''}">
                        <p style="font-size:.65rem;color:#9ca3af;text-transform:uppercase;letter-spacing:.06em;margin:0 0 .2rem;">${k}</p>
                        <p style="font-size:.8125rem;font-weight:700;color:#111827;margin:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">${v || '—'}</p>
                    </div>`).join('')}
            </div>

            <div style="display:flex;align-items:flex-start;gap:.625rem;padding:.875rem 1.25rem;background:#eff6ff;border-bottom:1.5px solid #dbeafe;">
                <i class="fas fa-info-circle" style="color:#3b82f6;margin-top:.125rem;flex-shrink:0;font-size:.8125rem"></i>
                <p style="font-size:.8125rem;color:#1e40af;margin:0;line-height:1.5;">Your evidence is encrypted, GPS-tagged, and timestamped. It is now visible to verified FLUG investigators.</p>
            </div>

            <div style="display:flex;">
                <a href="/evidence/view/${result.data.id}"
                   style="flex:1;display:flex;align-items:center;justify-content:center;gap:.5rem;padding:1rem;text-decoration:none;font-size:.875rem;font-weight:700;color:#fff;background:#111827;border-right:1px solid #1f2937;transition:background .15s;"
                   onmouseover="this.style.background='#374151'" onmouseout="this.style.background='#111827'">
                    <i class="fas fa-eye" style="font-size:.7rem"></i> View Evidence
                </a>
                <a href="/evidence/my"
                   style="flex:1;display:flex;align-items:center;justify-content:center;gap:.5rem;padding:1rem;text-decoration:none;font-size:.875rem;font-weight:600;color:#374151;background:#fff;transition:background .15s;"
                   onmouseover="this.style.background='#f9fafb'" onmouseout="this.style.background='#fff'">
                    <i class="fas fa-list" style="font-size:.7rem;color:#9ca3af"></i> My Evidence
                </a>
            </div>
        </div>`;
        
        const statusMessages = document.getElementById('statusMessages');
        if (statusMessages) statusMessages.innerHTML = successHtml;
        
        submitBtn.classList.add('hidden');
        
        console.log('=== EVIDENCE SUBMISSION COMPLETE ===');
        console.log('✅ Redirecting to view page in 5 seconds...');
        
        setTimeout(function() {
            window.location.href = `/evidence/view/${result.data.id}`;
        }, 5000);
        
    } catch (error) {
        console.error('Evidence submission failed:', error);
        
        // Show error message
        const errorHtml = `
        <div style="border-radius:.875rem;border:1.5px solid #fecaca;background:#fef2f2;overflow:hidden;font-family:'DM Sans',sans-serif;">
            <div style="display:flex;align-items:flex-start;gap:.75rem;padding:1rem 1.25rem;">
                <div style="width:36px;height:36px;border-radius:.625rem;background:#fff;border:1.5px solid #fecaca;display:flex;align-items:center;justify-content:center;flex-shrink:0;">
                    <i class="fas fa-exclamation-circle" style="color:#ef4444;font-size:.875rem"></i>
                </div>
                <div style="flex:1;min-width:0;">
                    <p style="font-size:.875rem;font-weight:700;color:#b91c1c;margin:0;">Submission Failed</p>
                    <p style="font-size:.8125rem;color:#dc2626;margin:.25rem 0 0;line-height:1.5;">${error.message}</p>
                </div>
            </div>
            <div style="display:flex;gap:.625rem;padding:.75rem 1.25rem;border-top:1.5px solid #fecaca;background:#fff;">
                <button onclick="retrySubmission()"
                        style="display:inline-flex;align-items:center;gap:.375rem;padding:.5rem 1rem;border-radius:.5rem;background:#111827;color:#fff;font-size:.8125rem;font-weight:600;border:none;cursor:pointer;font-family:'DM Sans',sans-serif;transition:background .15s;"
                        onmouseover="this.style.background='#374151'" onmouseout="this.style.background='#111827'">
                    <i class="fas fa-redo" style="font-size:.65rem"></i> Retry
                </button>
                <button onclick="clearForm()"
                        style="display:inline-flex;align-items:center;gap:.375rem;padding:.5rem 1rem;border-radius:.5rem;background:#fff;color:#374151;font-size:.8125rem;font-weight:600;border:1.5px solid #e5e7eb;cursor:pointer;font-family:'DM Sans',sans-serif;transition:background .15s;"
                        onmouseover="this.style.background='#f9fafb'" onmouseout="this.style.background='#fff'">
                    <i class="fas fa-times" style="font-size:.65rem"></i> Clear Form
                </button>
            </div>
        </div>`;
        
        const statusMessages = document.getElementById('statusMessages');
        if (statusMessages) {
            statusMessages.innerHTML = errorHtml;
        }
        
        // Restore button
        submitBtn.disabled = false;
        submitBtn.innerHTML = originalBtnText;
        
    } finally {
        // Hide progress if still showing
        const progressContainer = document.getElementById('targetsUploadProgress');
        if (progressContainer) {
            setTimeout(() => {
                progressContainer.classList.add('hidden');
            }, 3000);
        }
    }
}

// Helper functions for error handling
function retrySubmission() {
    console.log('Retrying submission...');
    window.location.reload();
}

function clearForm() {
    console.log('Clearing form...');
    const form = document.getElementById('completeForm');
    if (form) form.reset();
    
    const statusMessages = document.getElementById('statusMessages');
    if (statusMessages) statusMessages.innerHTML = '';
    
    const submitBtn = document.getElementById('submitBtn');
    if (submitBtn) {
        submitBtn.classList.remove('hidden');
        submitBtn.disabled = false;
        submitBtn.innerHTML = '<i class="fas fa-paper-plane mr-2"></i> Submit Evidence';
    }
}

// ==================== INITIALIZATION ====================

// Read evidenceId from a data attribute or hidden input on the page
function getEvidenceId() {
    const el = document.querySelector('[data-evidence-id]') ||
               document.querySelector('input[name="evidence_id"]') ||
               document.getElementById('evidenceIdInput');
    return el ? (el.dataset?.evidenceId || el.value || '') : '';
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    console.log('DOM loaded, initializing systems...');
    
    // Initialize location detection
    setTimeout(function() {
        fetchRealLocation();
    }, 1000);
    
    // Load target photos (read-only, from Storj via API)
    const evidenceId = getEvidenceId();
    if (evidenceId) {
        loadTargetPhotos(evidenceId);
    } else {
        console.warn('No evidence ID found — cannot load target photos');
        const emptyMsg = document.getElementById('targetsViewEmpty');
        if (emptyMsg) {
            emptyMsg.textContent = 'Target photos will appear here once the evidence record is saved.';
            emptyMsg.classList.remove('hidden');
        }
    }
    
    // Add event listeners for manual editing
    const locationInputs = ['countySelect', 'constituencyInput', 'wardInput', 'landmarkInput', 'latitudeInput', 'longitudeInput'];
    locationInputs.forEach(function(id) {
        const element = document.getElementById(id);
        if (element) {
            element.addEventListener('input', function() {
                hideLocationStatus();
            });
        }
    });
    
    console.log('All systems initialized');
});

// Export functions to window object
window.fetchRealLocation = fetchRealLocation;
window.getBrowserLocation = getBrowserLocation;
window.clearLocationFields = clearLocationFields;
window.loadTargetPhotos = loadTargetPhotos;
window.completeEvidence = completeEvidence;
window.retrySubmission = retrySubmission;
window.clearForm = clearForm;