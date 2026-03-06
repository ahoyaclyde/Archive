// ==================== DEBUG CONFIGURATION ====================
const DEBUG_MODE = true;
const DEBUG_PREFIX = '[EVIDENCE-CAPTURE]';

function debugLog(message, data = null, level = 'info') {
    if (!DEBUG_MODE) return;
    
    const timestamp = new Date().toISOString().split('T')[1].split('.')[0];
    const logMessage = `${DEBUG_PREFIX} [${timestamp}] ${message}`;
    
    switch(level) {
        case 'error':
            console.error(logMessage, data || '');
            break;
        case 'warn':
            console.warn(logMessage, data || '');
            break;
        case 'info':
        default:
            console.log(logMessage, data || '');
            break;
    }
}

// ==================== GLOBAL STATE ====================
let selectedFiles = [];
let mediaRecorder = null;
let recordedChunks = [];
let recordingStartTime = null;
let recordingTimerInterval = null;
let stream = null;
let recordingAttempts = 0;
const MAX_RECORDING_ATTEMPTS = 3;

// ==================== PRODUCTION-READY GEOLOCATION SYSTEM ====================

/**
 * Robust geolocation with multiple fallbacks, proxy detection, and reverse geocoding
 * NO hardcoded fallback coordinates - gracefully handles complete failure
 */
class RobustGeolocation {
    constructor() {
        this.ipServices = [
            {
                name: 'ipapi.co',
                url: 'https://ipapi.co/json/',
                parser: (data) => ({
                    latitude: data.latitude,
                    longitude: data.longitude,
                    city: data.city,
                    region: data.region,
                    country: data.country_name,
                    county: data.region,
                    ip: data.ip,
                    isProxy: data.is_proxy || false,
                    timezone: data.timezone
                })
            },
            {
                name: 'ip-api.com',
                url: 'http://ip-api.com/json/?fields=status,message,country,countryCode,region,regionName,city,lat,lon,timezone,proxy,query',
                parser: (data) => ({
                    latitude: data.lat,
                    longitude: data.lon,
                    city: data.city,
                    region: data.regionName,
                    country: data.country,
                    county: data.regionName,
                    ip: data.query,
                    isProxy: data.proxy || false,
                    timezone: data.timezone
                })
            },
            {
                name: 'ipinfo.io',
                url: 'https://ipinfo.io/json',
                parser: (data) => {
                    const [lat, lon] = data.loc ? data.loc.split(',') : [null, null];
                    return {
                        latitude: parseFloat(lat),
                        longitude: parseFloat(lon),
                        city: data.city,
                        region: data.region,
                        country: data.country,
                        county: data.region,
                        ip: data.ip,
                        isProxy: false,
                        timezone: data.timezone
                    };
                }
            },
            {
                name: 'ipgeolocation.io',
                url: 'https://api.ipgeolocation.io/ipgeo?apiKey=free',
                parser: (data) => ({
                    latitude: parseFloat(data.latitude),
                    longitude: parseFloat(data.longitude),
                    city: data.city,
                    region: data.state_prov,
                    country: data.country_name,
                    county: data.state_prov,
                    ip: data.ip,
                    isProxy: false,
                    timezone: data.time_zone?.name
                })
            }
        ];
    }

    /**
     * Tier 1: Browser GPS geolocation (most accurate)
     */
    async getBrowserGeolocation(timeout = 10000) {
        return new Promise((resolve, reject) => {
            if (!navigator.geolocation) {
                reject(new Error('Geolocation not supported by browser'));
                return;
            }
            
            const options = {
                enableHighAccuracy: true,
                timeout: timeout,
                maximumAge: 0
            };
            
            const timeoutId = setTimeout(() => {
                reject(new Error('Browser geolocation timeout'));
            }, timeout + 1000);
            
            navigator.geolocation.getCurrentPosition(
                async (position) => {
                    clearTimeout(timeoutId);
                    
                    const result = {
                        latitude: position.coords.latitude,
                        longitude: position.coords.longitude,
                        accuracy: position.coords.accuracy,
                        source: 'gps',
                        timestamp: new Date(position.timestamp).toISOString()
                    };
                    
                    // Try to get county via reverse geocoding
                    try {
                        const locationDetails = await this.reverseGeocode(
                            result.latitude, 
                            result.longitude
                        );
                        Object.assign(result, locationDetails);
                    } catch (error) {
                        debugLog('Reverse geocoding failed', error.message, 'warn');
                    }
                    
                    resolve(result);
                },
                (error) => {
                    clearTimeout(timeoutId);
                    reject(error);
                },
                options
            );
        });
    }

    /**
     * Tier 2: IP-based geolocation with multiple service fallbacks
     */
    async getIPBasedLocation() {
        debugLog('Starting IP-based geolocation with multiple services');
        
        const errors = [];
        
        for (const service of this.ipServices) {
            try {
                debugLog(`Trying IP service: ${service.name}`);
                
                const controller = new AbortController();
                const timeoutId = setTimeout(() => controller.abort(), 5000);
                
                const response = await fetch(service.url, {
                    signal: controller.signal,
                    headers: {
                        'Accept': 'application/json'
                    }
                });
                
                clearTimeout(timeoutId);
                
                if (!response.ok) {
                    throw new Error(`HTTP ${response.status}: ${response.statusText}`);
                }
                
                const data = await response.json();
                
                if (data.error || data.status === 'fail') {
                    throw new Error(data.message || data.error || 'Service returned error');
                }
                
                const location = service.parser(data);
                
                if (!location.latitude || !location.longitude || 
                    isNaN(location.latitude) || isNaN(location.longitude)) {
                    throw new Error('Invalid coordinates received');
                }
                
                debugLog(`✓ ${service.name} succeeded`, location);
                
                return {
                    ...location,
                    source: 'ip',
                    service: service.name,
                    accuracy: 'medium',
                    timestamp: new Date().toISOString()
                };
                
            } catch (error) {
                const errorMsg = `${service.name} failed: ${error.message}`;
                debugLog(errorMsg, null, 'warn');
                errors.push(errorMsg);
                continue;
            }
        }
        
        throw new Error(`All IP geolocation services failed: ${errors.join('; ')}`);
    }

    /**
     * Detect proxy/VPN by checking timezone and language consistency
     */
    async detectProxy() {
        const indicators = {
            timezoneMatch: false,
            languageMatch: false,
            possibleProxy: false
        };
        
        try {
            const browserTimezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
            
            const response = await fetch('https://ipapi.co/json/');
            const data = await response.json();
            
            if (data.timezone && browserTimezone) {
                indicators.timezoneMatch = data.timezone === browserTimezone;
                indicators.reportedTimezone = data.timezone;
                indicators.browserTimezone = browserTimezone;
            }
            
            indicators.possibleProxy = data.is_proxy || false;
            
            const browserLanguage = navigator.language || navigator.userLanguage;
            if (data.languages && browserLanguage) {
                const browserLang = browserLanguage.split('-')[0];
                indicators.languageMatch = data.languages.includes(browserLang);
            }
            
            debugLog('Proxy detection results', indicators);
            
            return indicators;
            
        } catch (error) {
            debugLog('Proxy detection failed', error.message, 'warn');
            return indicators;
        }
    }

    /**
     * Reverse geocoding to get county/administrative area
     * Uses OpenStreetMap Nominatim (free, no API key)
     */
    async reverseGeocode(latitude, longitude) {
        try {
            debugLog('Starting reverse geocoding', { latitude, longitude });
            
            const url = `https://nominatim.openstreetmap.org/reverse?` +
                `lat=${latitude}&lon=${longitude}&format=json&addressdetails=1`;
            
            const response = await fetch(url, {
                headers: {
                    'Accept': 'application/json',
                    'User-Agent': 'EvidenceCapture/1.0'
                }
            });
            
            if (!response.ok) {
                throw new Error(`Geocoding failed: ${response.status}`);
            }
            
            const data = await response.json();
            
            if (!data.address) {
                throw new Error('No address data in response');
            }
            
            const address = data.address;
            
            const county = address.county || 
                          address.state_district || 
                          address.region || 
                          address.state || 
                          address.province ||
                          'Unknown';
            
            const result = {
                county: county,
                city: address.city || address.town || address.village || '',
                region: address.state || address.region || '',
                country: address.country || '',
                formattedAddress: data.display_name || ''
            };
            
            debugLog('✓ Reverse geocoding successful', result);
            
            return result;
            
        } catch (error) {
            debugLog('Reverse geocoding failed', error.message, 'warn');
            throw error;
        }
    }

    /**
     * Main method: Try all tiers with intelligent fallbacks
     */
    async getLocation() {
        const startTime = Date.now();
        debugLog('=== Starting robust geolocation ===');
        
        const result = {
            latitude: null,
            longitude: null,
            county: 'Unknown',
            city: '',
            region: '',
            country: '',
            accuracy: 'unknown',
            source: 'none',
            timestamp: new Date().toISOString(),
            proxyDetection: null,
            errors: []
        };
        
        // Tier 1: Try browser GPS
        try {
            debugLog('Tier 1: Attempting browser GPS geolocation...');
            const gpsLocation = await this.getBrowserGeolocation(8000);
            
            Object.assign(result, gpsLocation);
            result.accuracy = 'high';
            
            debugLog('✓ GPS geolocation successful', {
                coords: `${result.latitude}, ${result.longitude}`,
                county: result.county,
                elapsedMs: Date.now() - startTime
            });
            
            // Run proxy detection in background
            this.detectProxy().then(proxyInfo => {
                result.proxyDetection = proxyInfo;
            }).catch(() => {});
            
            return result;
            
        } catch (gpsError) {
            const errorMsg = `GPS failed: ${gpsError.message}`;
            debugLog(errorMsg, null, 'warn');
            result.errors.push(errorMsg);
        }
        
        // Tier 2: Try IP-based geolocation
        try {
            debugLog('Tier 2: Attempting IP-based geolocation...');
            const ipLocation = await this.getIPBasedLocation();
            
            Object.assign(result, ipLocation);
            result.accuracy = 'medium';
            
            debugLog('✓ IP geolocation successful', {
                coords: `${result.latitude}, ${result.longitude}`,
                county: result.county,
                service: result.service,
                elapsedMs: Date.now() - startTime
            });
            
            // Try to enhance with reverse geocoding if county missing
            if (!result.county || result.county === 'Unknown') {
                try {
                    const geocoded = await this.reverseGeocode(result.latitude, result.longitude);
                    result.county = geocoded.county;
                    result.formattedAddress = geocoded.formattedAddress;
                } catch (error) {
                    debugLog('Could not enhance IP location with geocoding', null, 'warn');
                }
            }
            
            // Check proxy indicators
            try {
                result.proxyDetection = await this.detectProxy();
                if (result.proxyDetection.possibleProxy) {
                    result.accuracy = 'low';
                    debugLog('⚠ Proxy/VPN detected', result.proxyDetection, 'warn');
                }
            } catch (error) {
                debugLog('Proxy detection failed', null, 'warn');
            }
            
            return result;
            
        } catch (ipError) {
            const errorMsg = `IP geolocation failed: ${ipError.message}`;
            debugLog(errorMsg, null, 'error');
            result.errors.push(errorMsg);
        }
        
        // Tier 3: Final attempt with relaxed browser geolocation
        try {
            debugLog('Tier 3: Final attempt with relaxed browser geolocation...');
            
            const finalAttempt = await new Promise((resolve, reject) => {
                if (!navigator.geolocation) {
                    reject(new Error('Geolocation not available'));
                    return;
                }
                
                navigator.geolocation.getCurrentPosition(
                    (position) => resolve(position),
                    (error) => reject(error),
                    {
                        enableHighAccuracy: false,
                        timeout: 15000,
                        maximumAge: 60000
                    }
                );
            });
            
            result.latitude = finalAttempt.coords.latitude;
            result.longitude = finalAttempt.coords.longitude;
            result.accuracy = 'low';
            result.source = 'browser-cached';
            
            debugLog('✓ Final attempt succeeded (cached/low accuracy)', {
                coords: `${result.latitude}, ${result.longitude}`,
                elapsedMs: Date.now() - startTime
            });
            
            // Try reverse geocoding
            try {
                const geocoded = await this.reverseGeocode(result.latitude, result.longitude);
                Object.assign(result, geocoded);
            } catch (error) {
                debugLog('Reverse geocoding failed on final attempt', null, 'warn');
            }
            
            return result;
            
        } catch (finalError) {
            const errorMsg = `Final attempt failed: ${finalError.message}`;
            debugLog(errorMsg, null, 'error');
            result.errors.push(errorMsg);
        }
        
        // Complete failure
        debugLog('=== All geolocation methods failed ===', result.errors, 'error');
        throw new Error(`Geolocation completely failed: ${result.errors.join('; ')}`);
    }
}

// ==================== FILE HANDLING ====================
function setupFileDragDrop() {
    debugLog('Setting up file drag and drop');
    
    const dropZone = document.getElementById('dropZone');
    const fileInput = document.getElementById('fileInput');
    
    if (!dropZone || !fileInput) {
        debugLog('Required elements not found', {dropZone: !!dropZone, fileInput: !!fileInput}, 'error');
        return;
    }
    
    ['dragenter', 'dragover'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            e.preventDefault();
            dropZone.classList.add('drag-over');
            debugLog('Drag over drop zone');
        });
    });
    
    ['dragleave', 'drop'].forEach(eventName => {
        dropZone.addEventListener(eventName, (e) => {
            e.preventDefault();
            dropZone.classList.remove('drag-over');
            
            if (eventName === 'drop') {
                const files = Array.from(e.dataTransfer.files);
                debugLog('Files dropped', {count: files.length, names: files.map(f => f.name)});
                handleFiles(files);
            }
        });
    });
    
    fileInput.addEventListener('change', (e) => {
        const files = Array.from(e.target.files);
        debugLog('Files selected via input', {count: files.length, names: files.map(f => f.name)});
        handleFiles(files);
    });
    
    debugLog('File drag and drop setup complete');
}

function handleFiles(files) {
    debugLog('Handling files', {inputCount: files.length});
    
    const validFiles = files.filter(file => {
        const maxSize = 100 * 1024 * 1024; // 100MB
        if (file.size > maxSize) {
            debugLog('File rejected - size too large', {name: file.name, size: file.size, maxSize}, 'warn');
            showStatus(`File too large: ${file.name} (max 100MB)`, 'error');
            return false;
        }
        return true;
    });
    
    debugLog('Valid files', {count: validFiles.length});
    
    if (selectedFiles.length + validFiles.length > 3) {
        debugLog('Maximum files exceeded', {current: selectedFiles.length, tryingToAdd: validFiles.length}, 'warn');
        showStatus('Maximum 3 files allowed', 'error');
        return;
    }
    
    selectedFiles.push(...validFiles);
    updateFilePreview();
    
    if (selectedFiles.length > 0) {
        document.getElementById('filePreview').classList.remove('hidden');
    }
    
    debugLog('Files added successfully', {total: selectedFiles.length});
    showStatus(`Added ${validFiles.length} file(s)`, 'success');

    // ── Auto-trigger target extraction for any video file ─────────────────
    const videoFile = validFiles.find(f => f.type.startsWith('video/'));
    if (videoFile && typeof FrameExtractor !== 'undefined') {
        debugLog('Video file detected — opening frame extractor', videoFile.name);
        // Small delay so the file preview renders first
        setTimeout(() => FrameExtractor.openForFile(videoFile), 300);
    }
}

function updateFilePreview() {
    const fileList = document.getElementById('fileList');
    if (!fileList) {
        debugLog('File list element not found', null, 'error');
        return;
    }
    
    fileList.innerHTML = '';
    
    const hasVideo = selectedFiles.some(f => f.type.startsWith('video/'));

    selectedFiles.forEach((file, index) => {
        const fileSize = formatBytes(file.size);
        const isVideo = file.type.startsWith('video/');
        
        const fileItem = document.createElement('div');
        fileItem.className = 'flex items-center justify-between p-3 bg-gray-900 rounded-lg hover:bg-gray-800';
        fileItem.innerHTML = `
            <div class="flex items-center space-x-3">
                <div class="text-2xl">${isVideo ? '🎥' : '📁'}</div>
                <div class="flex-1 min-w-0">
                    <div class="font-medium truncate" title="${file.name}">${file.name}</div>
                    <div class="text-sm text-gray-400">${fileSize}${isVideo ? ' · <span class="text-purple-400">Video — target extraction available</span>' : ''}</div>
                </div>
            </div>
            <div class="flex items-center gap-2">
                ${isVideo ? `<button type="button" onclick="reopenExtractor(${index})"
                    title="Re-open target extractor for this video"
                    class="text-purple-400 hover:text-purple-300 p-1 rounded hover:bg-purple-900/30 text-sm">
                    🎯
                </button>` : ''}
                <button type="button" onclick="removeFile(${index})" 
                        class="text-red-400 hover:text-red-300 p-1 rounded hover:bg-red-900/30"
                        title="Remove file">
                    <i class="fas fa-times"></i>
                </button>
            </div>
        `;
        fileList.appendChild(fileItem);
    });

    // ── Target frames strip ─────────────────────────────────────────────
    updateTargetStrip();
    
    debugLog('File preview updated', {count: selectedFiles.length});
}

function reopenExtractor(fileIndex) {
    const file = selectedFiles[fileIndex];
    if (file && file.type.startsWith('video/') && typeof FrameExtractor !== 'undefined') {
        FrameExtractor.openForFile(file);
    }
}

function updateTargetStrip() {
    // Remove old strip
    const old = document.getElementById('fe-target-strip');
    if (old) old.remove();

    const blobs = typeof FrameExtractor !== 'undefined' ? FrameExtractor.getSelectedBlobs() : [];
    const filePreview = document.getElementById('filePreview');
    if (!filePreview) return;

    if (blobs.length === 0) {
        // Show placeholder only if there's a video
        const hasVideo = selectedFiles.some(f => f.type.startsWith('video/'));
        if (!hasVideo) return;

        const strip = document.createElement('div');
        strip.id = 'fe-target-strip';
        strip.className = 'mt-3 p-3 rounded-lg border border-dashed border-purple-700/40 bg-purple-900/10 text-center';
        strip.innerHTML = `
            <p class="text-xs text-purple-400 opacity-70">
                🎯 No target frames selected yet — the extractor will open automatically for video files.
            </p>`;
        filePreview.appendChild(strip);
        return;
    }

    const strip = document.createElement('div');
    strip.id = 'fe-target-strip';
    strip.className = 'mt-3 p-3 rounded-lg border border-purple-700/50 bg-purple-900/10';
    
    const thumbs = blobs.map(b => {
        const url = URL.createObjectURL(b.blob);
        return `<img src="${url}" 
                     class="w-12 h-12 object-cover rounded border border-purple-600/40"
                     title="${b.filename}" loading="lazy">`;
    }).join('');

    strip.innerHTML = `
        <div class="flex items-center justify-between mb-2">
            <span class="text-xs font-semibold text-purple-300">
                🎯 ${blobs.length} target frame(s) selected
                <span class="text-gray-500 font-normal ml-1">— will be uploaded as target photos</span>
            </span>
            <button type="button" onclick="clearTargetFrames()"
                class="text-xs text-red-400 hover:text-red-300 underline underline-offset-2">
                Clear targets
            </button>
        </div>
        <div class="flex gap-1.5 flex-wrap">${thumbs}</div>`;
    filePreview.appendChild(strip);
}

function clearTargetFrames() {
    if (typeof FrameExtractor !== 'undefined') FrameExtractor.clearSelected();
    updateTargetStrip();
    showStatus('Target frames cleared', 'info');
}

function removeFile(index) {
    if (index < 0 || index >= selectedFiles.length) {
        debugLog('Invalid file index to remove', {index, total: selectedFiles.length}, 'error');
        return;
    }
    
    const removedFile = selectedFiles[index];
    debugLog('Removing file', {index, name: removedFile.name});
    
    selectedFiles.splice(index, 1);
    updateFilePreview();
    
    if (selectedFiles.length === 0) {
        document.getElementById('filePreview').classList.add('hidden');
    }
    
    debugLog('File removed', {remaining: selectedFiles.length});
}

// ==================== LIVE RECORDING ====================

async function toggleLiveRecording() {
    debugLog('toggleLiveRecording called');
    const recordingUI = document.getElementById('liveRecordingUI');
    
    if (!recordingUI) {
        debugLog('Recording UI element not found', null, 'error');
        return;
    }
    
    const isVisible = !recordingUI.classList.contains('hidden');
    
    if (!isVisible) {
        debugLog('Showing recording UI');
        recordingUI.classList.remove('hidden');
        await initializeRecording();
    } else {
        debugLog('Hiding recording UI');
        recordingUI.classList.add('hidden');
        stopMediaStream();
    }
}

async function initializeRecording() {
    debugLog('Initializing recording');
    
    try {
        stopMediaStream();
        
        debugLog('Requesting media permissions');
        stream = await navigator.mediaDevices.getUserMedia({
            video: true,
            audio: true
        });
        
        debugLog('Stream obtained', { 
            videoTracks: stream.getVideoTracks().length,
            audioTracks: stream.getAudioTracks().length,
            tracks: stream.getTracks().map(t => `${t.kind}: ${t.label} (${t.readyState})`)
        });
        
        stream.getTracks().forEach(track => {
            const settings = track.getSettings ? track.getSettings() : {};
            debugLog(`Track ${track.kind} settings:`, settings);
        });
        
        const videoPreview = document.getElementById('videoPreview');
        if (!videoPreview) {
            debugLog('Video preview element not found', null, 'error');
            throw new Error('Video preview element missing');
        }
        
        videoPreview.srcObject = stream;
        
        videoPreview.onerror = (e) => {
            debugLog('Video preview error', e, 'warn');
        };
        
        try {
            await videoPreview.play();
            debugLog('Video preview playing successfully');
        } catch (playError) {
            debugLog('Video play failed (may require user interaction)', playError.message, 'warn');
        }
        
        const noCameraElement = document.getElementById('noCamera');
        if (noCameraElement) {
            noCameraElement.classList.add('hidden');
        }
        
        showStatus('Camera ready. Click "Start Recording" to begin.', 'success');
        
    } catch (error) {
        debugLog('Camera initialization failed', error, 'error');
        
        const noCameraElement = document.getElementById('noCamera');
        if (noCameraElement) {
            noCameraElement.classList.remove('hidden');
        }
        
        showStatus(`Camera error: ${getErrorMessage(error)}`, 'error');
    }
}

function getErrorMessage(error) {
    debugLog('Getting error message', {name: error.name, message: error.message});
    
    switch(error.name) {
        case 'NotAllowedError':
        case 'PermissionDeniedError':
            return 'Please allow camera/microphone access in browser settings';
        case 'NotFoundError':
        case 'DevicesNotFoundError':
            return 'No camera/microphone found';
        case 'NotReadableError':
        case 'TrackStartError':
            return 'Camera/microphone is in use by another application';
        case 'OverconstrainedError':
            return 'Cannot match requested video/audio settings';
        case 'AbortError':
            return 'Media device operation aborted';
        default:
            return error.message || 'Unknown error';
    }
}

function createMediaRecorder() {
    debugLog('Creating MediaRecorder');
    
    if (!stream) {
        debugLog('No stream available for MediaRecorder', null, 'error');
        throw new Error('No media stream available');
    }
    
    let options = {};
    
    const preferredTypes = [
        'video/webm',
        'video/webm;codecs=vp8,opus',
        'video/webm;codecs=vp9,opus',
        'video/mp4'
    ];
    
    debugLog('Testing MIME type support:');
    let supportedType = null;
    
    for (const mimeType of preferredTypes) {
        try {
            if (MediaRecorder.isTypeSupported(mimeType)) {
                debugLog(`✓ Supported: ${mimeType}`);
                supportedType = mimeType;
                break;
            } else {
                debugLog(`✗ Not supported: ${mimeType}`);
            }
        } catch (e) {
            debugLog(`Error testing ${mimeType}`, e.message, 'warn');
        }
    }
    
    if (supportedType) {
        options.mimeType = supportedType;
        debugLog(`Using MIME type: ${supportedType}`);
    } else {
        debugLog('No specific MIME type supported, using browser default');
    }
    
    options.videoBitsPerSecond = 1000000;
    options.audioBitsPerSecond = 64000;
    
    debugLog('MediaRecorder options', options);
    
    try {
        const recorder = new MediaRecorder(stream, options);
        
        recorder.onstart = () => {
            debugLog('MediaRecorder started event fired');
        };
        
        recorder.onpause = () => {
            debugLog('MediaRecorder paused event fired');
        };
        
        recorder.onresume = () => {
            debugLog('MediaRecorder resumed event fired');
        };
        
        debugLog('MediaRecorder created successfully', { 
            state: recorder.state,
            mimeType: recorder.mimeType || 'default'
        });
        return recorder;
    } catch (error) {
        debugLog('Failed to create MediaRecorder with options', error, 'error');
        
        try {
            debugLog('Trying MediaRecorder without options');
            const fallbackRecorder = new MediaRecorder(stream);
            debugLog('MediaRecorder created without options', { state: fallbackRecorder.state });
            return fallbackRecorder;
        } catch (fallbackError) {
            debugLog('Failed to create MediaRecorder at all', fallbackError, 'error');
            throw new Error(`Browser does not support recording: ${fallbackError.message}`);
        }
    }
}

function startRecording() {
    debugLog('=== STARTING RECORDING ===');
    
    try {
        if (!stream) {
            debugLog('No stream available', null, 'error');
            showStatus('Please initialize camera first', 'error');
            return;
        }
        
        const videoTracks = stream.getVideoTracks();
        const audioTracks = stream.getAudioTracks();
        
        debugLog('Stream health check', {
            videoTracks: videoTracks.length,
            audioTracks: audioTracks.length,
            videoReady: videoTracks.length > 0 ? videoTracks[0].readyState : 'none',
            audioReady: audioTracks.length > 0 ? audioTracks[0].readyState : 'none'
        });
        
        if (videoTracks.length === 0 && audioTracks.length === 0) {
            debugLog('No tracks available', null, 'error');
            showStatus('Camera or microphone not available', 'error');
            return;
        }
        
        recordedChunks = [];
        recordingStartTime = Date.now();
        recordingAttempts++;
        
        debugLog('Recording attempt', { attempt: recordingAttempts });
        
        if (recordingAttempts > MAX_RECORDING_ATTEMPTS) {
            debugLog('Max recording attempts reached', recordingAttempts, 'error');
            showStatus('Recording failed after multiple attempts. Please refresh the page.', 'error');
            return;
        }
        
        mediaRecorder = createMediaRecorder();
        
        if (!mediaRecorder) {
            throw new Error('Failed to create MediaRecorder');
        }
        
        mediaRecorder.ondataavailable = handleDataAvailable;
        mediaRecorder.onstop = handleRecordingStop;
        mediaRecorder.onerror = handleRecordingError;
        
        debugLog('Starting MediaRecorder');
        
        mediaRecorder.start();
        
        debugLog('Recording started', { 
            state: mediaRecorder.state,
            mimeType: mediaRecorder.mimeType || 'not specified'
        });
        
        document.getElementById('startRecordingBtn').classList.add('hidden');
        document.getElementById('stopRecordingBtn').classList.remove('hidden');
        document.getElementById('recordingIndicator').classList.remove('hidden');
        
        updateRecordingTimer();
        recordingTimerInterval = setInterval(updateRecordingTimer, 1000);
        
        showStatus('Recording... Speak clearly.', 'success');
        
        setTimeout(() => {
            if (mediaRecorder && mediaRecorder.state === 'recording') {
                debugLog('Requesting data chunk after 2s');
                mediaRecorder.requestData();
            }
        }, 2000);
        
    } catch (error) {
        debugLog('Failed to start recording', error, 'error');
        showStatus(`Recording failed: ${error.message}`, 'error');
    }
}

function handleDataAvailable(event) {
    debugLog('Data available event', {
        size: event.data?.size || 0,
        type: event.data?.type || 'unknown',
        timecode: event.timeStamp
    });
    
    if (event.data && event.data.size > 0) {
        debugLog(`Adding data chunk: ${event.data.size} bytes, type: ${event.data.type}`);
        recordedChunks.push(event.data);
    } else {
        debugLog('Empty or null data chunk received', {
            hasData: !!event.data,
            size: event.data?.size,
            type: event.data?.type,
            currentTime: Date.now()
        }, 'warn');
        
        if (mediaRecorder && mediaRecorder.state === 'recording' && recordedChunks.length === 0) {
            debugLog('Attempting to recover by requesting new data');
            setTimeout(() => {
                if (mediaRecorder && mediaRecorder.state === 'recording') {
                    mediaRecorder.requestData();
                }
            }, 1000);
        }
    }
}

function handleRecordingStop() {
    debugLog('=== RECORDING STOPPED ===');
    
    console.group('Recording Analysis');
    debugLog('Total chunks collected:', recordedChunks.length);
    
    let totalSize = 0;
    recordedChunks.forEach((chunk, index) => {
        debugLog(`Chunk ${index}:`, {
            size: chunk.size,
            type: chunk.type
        });
        totalSize += chunk.size;
    });
    
    debugLog('Total recording size:', `${totalSize} bytes (${formatBytes(totalSize)})`);
    console.groupEnd();
    
    if (totalSize === 0) {
        debugLog('CRITICAL: No data captured in recording', {
            chunks: recordedChunks.length,
            streamActive: !!stream,
            tracks: stream ? stream.getTracks().map(t => `${t.kind}:${t.readyState}`) : 'no stream',
            mediaRecorderState: mediaRecorder?.state
        }, 'error');
        
        if (stream) {
            stream.getTracks().forEach(track => {
                const settings = track.getSettings ? track.getSettings() : {};
                debugLog(`Track ${track.kind}:`, {
                    label: track.label,
                    readyState: track.readyState,
                    enabled: track.enabled,
                    muted: track.muted,
                    settings: settings
                });
            });
        }
        
        showStatus('Recording failed: No video/audio data was captured. Try a different browser (Chrome works best).', 'error');
        return;
    }
    
    const mimeType = mediaRecorder.mimeType || 'video/webm';
    debugLog('Creating final blob', { mimeType, chunkCount: recordedChunks.length });
    
    try {
        const blob = new Blob(recordedChunks, { type: mimeType });
        
        debugLog('Final blob created', {
            size: blob.size,
            type: blob.type,
            readableSize: formatBytes(blob.size)
        });
        
        createRecordingPreview(blob);
        
        document.getElementById('recordingPreview').classList.remove('hidden');
        
        recordingAttempts = 0;
        
        showStatus('Recording complete! Preview available below.', 'success');
        
    } catch (error) {
        debugLog('Failed to create blob from chunks', error, 'error');
        showStatus('Failed to process recording data', 'error');
    }
    
    if (recordingTimerInterval) {
        clearInterval(recordingTimerInterval);
        recordingTimerInterval = null;
    }
}

function handleRecordingError(event) {
    debugLog('MediaRecorder error occurred', {
        error: event.error,
        errorName: event.error?.name,
        errorMessage: event.error?.message,
        currentState: mediaRecorder?.state
    }, 'error');
    
    showStatus(`Recording error: ${event.error?.name || 'Unknown error'}`, 'error');
    
    if (mediaRecorder && mediaRecorder.state === 'recording') {
        debugLog('Attempting to stop MediaRecorder after error');
        try {
            mediaRecorder.stop();
        } catch (stopError) {
            debugLog('Failed to stop MediaRecorder after error', stopError, 'error');
        }
    }
}

function createRecordingPreview(blob) {
    debugLog('Creating recording preview');
    
    try {
        const url = URL.createObjectURL(blob);
        const recordedVideo = document.getElementById('recordedVideo');
        
        if (!recordedVideo) {
            debugLog('Recorded video element not found', null, 'error');
            throw new Error('Preview element missing');
        }
        
        if (recordedVideo.src && recordedVideo.src.startsWith('blob:')) {
            debugLog('Revoking previous blob URL');
            URL.revokeObjectURL(recordedVideo.src);
        }
        
        recordedVideo.src = url;
        
        recordedVideo.onerror = (e) => {
            debugLog('Recorded video playback error', {
                error: e,
                src: recordedVideo.src,
                networkState: recordedVideo.networkState,
                errorState: recordedVideo.error
            }, 'warn');
        };
        
        recordedVideo.onloadeddata = () => {
            debugLog('Recorded video loaded', {
                duration: recordedVideo.duration,
                videoWidth: recordedVideo.videoWidth,
                videoHeight: recordedVideo.videoHeight,
                readyState: recordedVideo.readyState
            });
        };
        
        recordedVideo.oncanplay = () => {
            debugLog('Recorded video can play');
            recordedVideo.play().catch(e => {
                debugLog('Auto-play prevented (normal for some browsers)', e.message, 'info');
            });
        };
        
        debugLog('Recording preview created successfully', { url: url.substring(0, 50) + '...' });
        
    } catch (error) {
        debugLog('Failed to create recording preview', error, 'error');
        showStatus('Preview unavailable, but recording was saved', 'warning');
    }
}

function stopRecording() {
    debugLog('stopRecording called');
    
    if (!mediaRecorder) {
        debugLog('No MediaRecorder instance found', null, 'warn');
        return;
    }
    
    debugLog('MediaRecorder state before stop:', mediaRecorder.state);
    
    if (mediaRecorder.state === 'recording') {
        debugLog('Stopping active recording...');
        
        mediaRecorder.requestData();
        
        setTimeout(() => {
            try {
                mediaRecorder.stop();
                debugLog('Stop command sent to MediaRecorder');
            } catch (stopError) {
                debugLog('Error stopping MediaRecorder', stopError, 'error');
            }
        }, 500);
    } else if (mediaRecorder.state === 'inactive') {
        debugLog('MediaRecorder already stopped');
        handleRecordingStop();
    }
    
    document.getElementById('startRecordingBtn').classList.remove('hidden');
    document.getElementById('stopRecordingBtn').classList.add('hidden');
    document.getElementById('recordingIndicator').classList.add('hidden');
}

function saveRecording() {
    debugLog('saveRecording called');
    
    const recordedVideo = document.getElementById('recordedVideo');
    
    if (!recordedVideo) {
        debugLog('Recorded video element not found', null, 'error');
        showStatus('Recording preview not available', 'error');
        return;
    }
    
    if (!recordedVideo.src || recordedVideo.src === '') {
        debugLog('No recording source available', null, 'error');
        showStatus('No recording available to save', 'error');
        return;
    }
    
    debugLog('Fetching recording from blob URL');
    
    fetch(recordedVideo.src)
        .then(response => {
            debugLog('Fetch response', { 
                ok: response.ok, 
                status: response.status,
                statusText: response.statusText 
            });
            
            if (!response.ok) {
                throw new Error(`Fetch failed: ${response.status} ${response.statusText}`);
            }
            
            return response.blob();
        })
        .then(blob => {
            debugLog('Blob obtained', {
                size: blob.size,
                type: blob.type,
                readableSize: formatBytes(blob.size)
            });
            
            if (blob.size === 0) {
                throw new Error('Recording is empty (0 bytes)');
            }
            
            const timestamp = new Date().toISOString()
                .replace(/[:.]/g, '-')
                .replace('T', '_')
                .substring(0, 19);
            
            let extension = 'webm';
            if (blob.type.includes('mp4')) extension = 'mp4';
            
            const filename = `evidence_recording_${timestamp}.${extension}`;
            
            debugLog('Creating file object', { filename, type: blob.type });
            
            const file = new File([blob], filename, { 
                type: blob.type,
                lastModified: Date.now()
            });
            
            handleFiles([file]);
            
            URL.revokeObjectURL(recordedVideo.src);
            recordedVideo.src = '';
            
            hideRecordingUI();
            
            debugLog('Recording saved successfully', { 
                name: file.name,
                size: formatBytes(file.size),
                totalFiles: selectedFiles.length 
            });
            
            showStatus('Recording added to evidence files!', 'success');
            
        })
        .catch(error => {
            debugLog('Error saving recording', error, 'error');
            showStatus(`Failed to save recording: ${error.message}`, 'error');
        });
}

function cancelRecording() {
    debugLog('cancelRecording called');
    
    if (mediaRecorder && mediaRecorder.state === 'recording') {
        debugLog('Stopping active recording on cancel');
        mediaRecorder.stop();
    }
    
    const recordedVideo = document.getElementById('recordedVideo');
    if (recordedVideo && recordedVideo.src) {
        debugLog('Cleaning up blob URL');
        URL.revokeObjectURL(recordedVideo.src);
        recordedVideo.src = '';
    }
    
    recordedChunks = [];
    hideRecordingUI();
    
    debugLog('Recording cancelled');
    showStatus('Recording cancelled', 'info');
}

function discardRecording() {
    debugLog('discardRecording called');
    
    const recordedVideo = document.getElementById('recordedVideo');
    if (recordedVideo && recordedVideo.src) {
        debugLog('Cleaning up blob URL');
        URL.revokeObjectURL(recordedVideo.src);
        recordedVideo.src = '';
    }
    
    recordedChunks = [];
    hideRecordingUI();
    
    debugLog('Recording discarded');
    showStatus('Recording discarded', 'info');
}

function hideRecordingUI() {
    debugLog('Hiding recording UI');
    
    const recordingPreview = document.getElementById('recordingPreview');
    const liveRecordingUI = document.getElementById('liveRecordingUI');
    
    if (recordingPreview) recordingPreview.classList.add('hidden');
    if (liveRecordingUI) liveRecordingUI.classList.add('hidden');
    
    document.getElementById('startRecordingBtn').classList.remove('hidden');
    document.getElementById('stopRecordingBtn').classList.add('hidden');
    document.getElementById('recordingIndicator').classList.add('hidden');
    
    stopMediaStream();
    
    if (recordingTimerInterval) {
        clearInterval(recordingTimerInterval);
        recordingTimerInterval = null;
    }
    
    recordingAttempts = 0;
    debugLog('Recording UI hidden, state reset');
}

function stopMediaStream() {
    debugLog('stopMediaStream called');
    
    if (stream) {
        debugLog('Stopping media stream tracks', { 
            trackCount: stream.getTracks().length,
            tracks: stream.getTracks().map(t => `${t.kind}:${t.readyState}`)
        });
        
        stream.getTracks().forEach(track => {
            debugLog(`Stopping ${track.kind} track`, track.label);
            track.stop();
        });
        
        stream = null;
        
        const videoPreview = document.getElementById('videoPreview');
        if (videoPreview) {
            videoPreview.srcObject = null;
        }
        
        debugLog('Media stream stopped');
    }
}

function updateRecordingTimer() {
    if (!recordingStartTime) {
        debugLog('No recording start time set', null, 'warn');
        return;
    }
    
    const elapsed = Math.floor((Date.now() - recordingStartTime) / 1000);
    const minutes = Math.floor(elapsed / 60);
    const seconds = elapsed % 60;
    
    const timerElement = document.getElementById('recordingTimer');
    if (timerElement) {
        timerElement.textContent = 
            `${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
    }
}

// ==================== EVIDENCE CAPTURE WITH ROBUST GEOLOCATION ====================

/**
 * Convert a Blob to a base64 string (data-URL stripped to raw base64).
 * Used to send target frames to /api/evidence/targets/upload as JSON.
 */
function blobToBase64(blob) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload  = () => resolve(reader.result.split(',')[1]);
        reader.onerror = reject;
        reader.readAsDataURL(blob);
    });
}

/**
 * Upload selected target frames to Storj via the existing targets API.
 * Runs AFTER the draft evidence is created so we have the evidenceId.
 * Returns number of targets successfully uploaded.
 */
async function uploadTargetsToStorj(evidenceId, targetBlobs) {
    if (!targetBlobs.length) return 0;

    debugLog(`Uploading ${targetBlobs.length} target frame(s) to Storj…`);

    // Convert all blobs to base64 in parallel
    const photos = await Promise.all(
        targetBlobs.map(async (t, i) => ({
            filename:         t.filename,
            mime_type:        t.blob.type || 'image/jpeg',
            data_base64:      await blobToBase64(t.blob),
            description:      `Auto-extracted target person #${i + 1}${t.hasFace ? ' (face detected)' : ''}`,
            category:         'person',
            confidence_score: t.hasFace ? 80 : 50,
        }))
    );

    const response = await fetch('/api/evidence/targets/upload', {
        method:  'POST',
        headers: { 'Content-Type': 'application/json' },
        body:    JSON.stringify({ evidence_id: evidenceId, photos }),
    });

    if (!response.ok) {
        const txt = await response.text().catch(() => '');
        throw new Error(`Target upload failed (${response.status}): ${txt}`);
    }

    const result = await response.json();
    const count  = result.data?.length ?? photos.length;
    debugLog(`✅ ${count} target frame(s) uploaded to Storj`);
    return count;
}

async function captureEvidence(event) {
    event.preventDefault();
    debugLog('captureEvidence called');

    // ── 1. Must have at least one evidence file ───────────────────────────
    if (selectedFiles.length === 0) {
        showStatus('Please add at least one evidence file', 'error');
        return;
    }

    // ── 2. GATE: video present but no targets confirmed ───────────────────
    const hasVideo = selectedFiles.some(f => f.type.startsWith('video/'));
    const targetBlobs = (typeof FrameExtractor !== 'undefined')
        ? FrameExtractor.getSelectedBlobs()
        : [];

    if (hasVideo && targetBlobs.length === 0) {
        // Re-open the extractor so they can select targets
        const proceed = await showTargetGate();
        if (!proceed) return;  // user chose to go back and select targets
        // If they chose to proceed anyway, targetBlobs remains empty (no targets uploaded)
    }

    debugLog('Starting evidence capture', { fileCount: selectedFiles.length, targets: targetBlobs.length });

    // ── 3. Setup UI ───────────────────────────────────────────────────────
    const captureBtn        = document.getElementById('captureBtn');
    const progressContainer = document.getElementById('uploadProgressContainer');
    const progressBar       = document.getElementById('uploadProgress');
    const progressText      = document.getElementById('progressText');

    if (captureBtn) {
        captureBtn.disabled  = true;
        captureBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Capturing…';
    }
    if (progressContainer) progressContainer.classList.remove('hidden');
    if (progressBar)       progressBar.style.width = '10%';
    if (progressText)      progressText.textContent = 'Getting location…';

    // ── 4. Geolocation ────────────────────────────────────────────────────
    let locationData = null;
    try {
        showStatus('Detecting your location…', 'info');
        locationData = await new RobustGeolocation().getLocation();

        if      (locationData.accuracy === 'high')   showStatus('✓ GPS location detected', 'success');
        else if (locationData.accuracy === 'medium')  showStatus(`✓ Location: ${locationData.city || 'approximate'}`, 'success');
        else                                          showStatus('⚠ Low-accuracy location', 'warning');

        if (locationData.proxyDetection?.possibleProxy)
            showStatus('⚠ VPN/Proxy detected — location may be inaccurate', 'warning');

    } catch (err) {
        debugLog('Geolocation failed', err, 'warn');
        showStatus('⚠ Location unavailable — evidence saved without coordinates', 'warning');
        locationData = { latitude: null, longitude: null, county: 'Unknown',
                         accuracy: 'none', source: 'failed' };
    }

    if (progressBar)  progressBar.style.width = '25%';
    if (progressText) progressText.textContent = 'Preparing upload…';

    // ── 5. Build FormData (evidence files only, no target blobs) ──────────
    const formData = new FormData();
    selectedFiles.forEach(file => formData.append('files', file, file.name));

    const now = new Date();
    formData.append('title',           'DRAFT - Evidence Pending Details');
    formData.append('description',     'Evidence captured. Details pending completion.');
    formData.append('emergency_level', 'blue');
    formData.append('incident_type',   'Other');
    formData.append('incident_date',   now.toISOString().split('T')[0]);
    formData.append('incident_time',   now.toTimeString().substring(0, 5));
    formData.append('latitude',        locationData.latitude  ? locationData.latitude.toString()  : '');
    formData.append('longitude',       locationData.longitude ? locationData.longitude.toString() : '');
    formData.append('county',          locationData.county   || 'Unknown');
    formData.append('city',            locationData.city     || '');
    formData.append('region',          locationData.region   || '');
    formData.append('country',         locationData.country  || '');
    formData.append('location_accuracy', locationData.accuracy || 'none');
    formData.append('location_source',   locationData.source   || 'failed');
    if (locationData.proxyDetection?.possibleProxy) formData.append('proxy_detected', 'true');
    formData.append('is_anonymous',     'false');
    formData.append('reported_to_police', 'false');

    const walletEl = document.getElementById('signWithWallet');
    formData.append('sign_with_wallet', walletEl ? walletEl.value : 'false');

    // ── 6. Upload evidence files → get evidenceId ─────────────────────────
    if (progressBar)  progressBar.style.width = '40%';
    if (progressText) progressText.textContent = 'Uploading evidence…';

    let evidenceData;
    try {
        const res = await fetch('/api/evidence/upload', { method: 'POST', body: formData });

        if (!res.ok) {
            const txt = await res.text();
            let msg = `Server error ${res.status}`;
            try { msg = JSON.parse(txt).message || msg; } catch (_) {}
            throw new Error(msg);
        }

        const json = await res.json();
        if (!json.success) throw new Error(json.message || 'Upload failed');
        evidenceData = json.data;
        debugLog('Evidence created', evidenceData);

    } catch (err) {
        debugLog('Evidence upload failed', err, 'error');
        showStatus(`Capture failed: ${err.message}`, 'error');
        if (captureBtn) {
            captureBtn.disabled  = false;
            captureBtn.innerHTML = '<i class="fas fa-camera mr-2"></i> Capture Evidence';
        }
        if (progressContainer) setTimeout(() => progressContainer.classList.add('hidden'), 3000);
        return;
    }

    // ── 7. Upload target frames to Storj (sequential, using existing endpoint) ──
    let targetsUploaded = 0;
    if (targetBlobs.length > 0) {
        if (progressBar)  progressBar.style.width = '65%';
        if (progressText) progressText.textContent = `Uploading ${targetBlobs.length} target frame(s) to Storj…`;

        try {
            targetsUploaded = await uploadTargetsToStorj(evidenceData.id, targetBlobs);
            showStatus(`✅ ${targetsUploaded} target frame(s) uploaded`, 'success');
        } catch (err) {
            debugLog('Target upload failed (non-fatal)', err, 'warn');
            showStatus(`⚠ Evidence saved but target upload failed: ${err.message}`, 'warning');
        }
    }

    // ── 8. Done — show success card with redirect link ────────────────────
    if (progressBar)  progressBar.style.width = '100%';
    if (progressText) progressText.textContent = 'Complete!';

    // Clear the extractor selection now that frames are on Storj
    if (typeof FrameExtractor !== 'undefined') FrameExtractor.clearSelected();

    showSuccessMessage({ ...evidenceData, targets_uploaded: targetsUploaded });
    resetForm();
}

/**
 * Gate modal shown when a video was added but no target frames were selected.
 * Resolves true  = user chose to skip targets and proceed anyway
 * Resolves false = user wants to go back and select targets (re-opens extractor)
 */
function showTargetGate() {
    return new Promise(resolve => {
        // Build a simple overlay
        const overlay = document.createElement('div');
        overlay.style.cssText = `
            position:fixed;inset:0;z-index:99999;background:rgba(0,0,0,.8);
            display:flex;align-items:center;justify-content:center;padding:1rem;`;
        overlay.innerHTML = `
        <div style="background:#111827;border:1px solid #7c3aed;border-radius:1rem;
                    max-width:28rem;width:100%;padding:1.5rem;">
            <div style="display:flex;align-items:center;gap:.75rem;margin-bottom:1rem;">
                <span style="font-size:1.5rem;">🎯</span>
                <h3 style="color:#c4b5fd;font-weight:700;font-size:1rem;margin:0;">
                    No Target Persons Selected
                </h3>
            </div>
            <p style="color:#9ca3af;font-size:.875rem;margin-bottom:1.25rem;line-height:1.5;">
                Your video was added but no person frames were selected for target identification.
                <br><br>
                Target frames help identify suspects and are uploaded securely to Storj before your
                evidence is submitted — this cannot be done later.
            </p>
            <div style="display:flex;gap:.75rem;">
                <button id="gate-back"
                    style="flex:1;padding:.625rem 1rem;border-radius:.5rem;background:#7c3aed;
                           color:white;font-size:.875rem;font-weight:600;border:none;cursor:pointer;">
                    ← Select Targets Now
                </button>
                <button id="gate-skip"
                    style="flex:1;padding:.625rem 1rem;border-radius:.5rem;background:#374151;
                           color:#9ca3af;font-size:.875rem;border:none;cursor:pointer;">
                    Skip &amp; Continue
                </button>
            </div>
        </div>`;

        document.body.appendChild(overlay);

        overlay.querySelector('#gate-back').onclick = () => {
            document.body.removeChild(overlay);
            // Re-open the extractor for the first video file
            const videoFile = selectedFiles.find(f => f.type.startsWith('video/'));
            if (videoFile && typeof FrameExtractor !== 'undefined') {
                FrameExtractor.openForFile(videoFile);
            }
            resolve(false);
        };

        overlay.querySelector('#gate-skip').onclick = () => {
            document.body.removeChild(overlay);
            resolve(true);
        };
    });
}

function showSuccessMessage(data) {
    debugLog('Showing success message', data);
    
    const captureStatus = document.getElementById('captureStatus');
    if (!captureStatus) return;

    const targetsLine = (data.targets_uploaded > 0)
        ? `<div class="flex justify-between">
               <span class="text-gray-400">Targets on Storj:</span>
               <span class="text-purple-400">✅ ${data.targets_uploaded} frame(s)</span>
           </div>`
        : `<div class="flex justify-between">
               <span class="text-gray-400">Targets:</span>
               <span class="text-gray-500">None selected</span>
           </div>`;
    
    const successHtml = `
        <div class="p-6 bg-green-900/20 border border-green-700 rounded-lg">
            <div class="flex items-center mb-4">
                <i class="fas fa-check-circle text-green-400 text-2xl mr-3"></i>
                <div>
                    <h3 class="font-bold text-lg">Evidence Captured Successfully!</h3>
                    <p class="text-gray-300">Evidence Number: ${data.evidence_number}</p>
                    <p class="text-sm text-gray-400 mt-1">Status: <span class="text-yellow-400">DRAFT</span> — complete details to publish</p>
                </div>
            </div>
            <div class="space-y-2 text-sm mb-4">
                <div class="flex justify-between">
                    <span class="text-gray-400">Files:</span>
                    <span>${data.media_files} file(s) on Storj</span>
                </div>
                ${targetsLine}
                <div class="flex justify-between">
                    <span class="text-gray-400">Captured:</span>
                    <span>${new Date().toLocaleString()}</span>
                </div>
            </div>
            <div class="flex space-x-4">
                <a href="/evidence/complete/${data.id}" 
                   class="flex-1 bg-blue-600 py-3 rounded text-center hover:bg-blue-700 font-medium">
                    <i class="fas fa-edit mr-2"></i>Complete Details →
                </a>
                <a href="/evidence/dashboard" 
                   class="flex-1 bg-gray-700 py-3 rounded text-center hover:bg-gray-600">
                    Dashboard
                </a>
            </div>
        </div>
    `;
    
    captureStatus.insertAdjacentHTML('afterbegin', successHtml);
    debugLog('Success message displayed');
}

function resetForm() {
    debugLog('Resetting form');
    
    selectedFiles = [];
    
    const filePreview = document.getElementById('filePreview');
    if (filePreview) {
        filePreview.classList.add('hidden');
    }
    
    const uploadProgressContainer = document.getElementById('uploadProgressContainer');
    if (uploadProgressContainer) {
        uploadProgressContainer.classList.add('hidden');
    }
    
    const captureBtn = document.getElementById('captureBtn');
    if (captureBtn) {
        captureBtn.disabled = false;
        captureBtn.innerHTML = '<i class="fas fa-camera mr-2"></i> Capture Evidence';
    }
    
    stopMediaStream();
    recordedChunks = [];
    
    debugLog('Form reset complete');
}

// ==================== UTILITIES ====================
function showStatus(message, type = 'info') {
    debugLog(`Showing status: ${type}`, message);
    
    const colors = {
        success: 'bg-green-900/30 border-green-700 text-green-300',
        error: 'bg-red-900/30 border-red-700 text-red-300',
        info: 'bg-blue-900/30 border-blue-700 text-blue-300',
        warning: 'bg-yellow-900/30 border-yellow-700 text-yellow-300'
    };
    
    const statusDiv = document.getElementById('captureStatus');
    if (!statusDiv) {
        debugLog('Status div not found', null, 'error');
        return;
    }
    
    const statusMsg = document.createElement('div');
    statusMsg.className = `p-3 rounded-lg border ${colors[type] || colors.info}`;
    statusMsg.innerHTML = message;
    
    statusDiv.insertAdjacentElement('afterbegin', statusMsg);
    
    // Auto-remove after 5 seconds
    setTimeout(() => {
        if (statusMsg.parentNode) {
            statusMsg.remove();
        }
    }, 5000);
}

function formatBytes(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function testMediaRecorderWorkaround() {
    debugLog('Testing browser-specific workarounds');
    
    const isChrome = !!window.chrome && (!!window.chrome.webstore || !!window.chrome.runtime);
    const isEdge = navigator.userAgent.includes('Edg');
    
    if (isChrome || isEdge) {
        debugLog('Chrome/Edge detected - using standard approach');
        return 'standard';
    }
    
    const isFirefox = typeof InstallTrigger !== 'undefined';
    if (isFirefox) {
        debugLog('Firefox detected - may need different settings');
        return 'firefox';
    }
    
    debugLog('Other browser detected - trying compatibility mode');
    return 'compatibility';
}

// ── Listen for frame extractor confirmation ───────────────────────────────
window.addEventListener('fe:confirmed', function(e) {
    const blobs = e.detail?.blobs || [];
    debugLog(`Frame extractor confirmed ${blobs.length} target frames`);
    updateTargetStrip();
    if (blobs.length > 0) {
        showStatus(`✅ ${blobs.length} target frame(s) selected for upload`, 'success');
    }
});

// ==================== INITIALIZATION ====================
function initializeApplication() {
    debugLog('Initializing Evidence Capture Application');
    
    // Check for required elements
    const requiredElements = [
        'dropZone', 'fileInput', 'filePreview', 'fileList',
        'captureBtn', 'captureStatus'
    ];
    
    const missingElements = requiredElements.filter(id => !document.getElementById(id));
    
    if (missingElements.length > 0) {
        debugLog('Missing required elements', missingElements, 'error');
        showStatus('Application initialization failed: Missing required elements', 'error');
        return;
    }
    
    // Setup file handling
    setupFileDragDrop();
    
    // Test browser compatibility
    testMediaRecorderWorkaround();
    
    // Setup cleanup on page unload
    window.addEventListener('beforeunload', () => {
        debugLog('Page unloading - cleaning up');
        stopMediaStream();
        if (mediaRecorder && mediaRecorder.state === 'recording') {
            mediaRecorder.stop();
        }
    });
    
    debugLog('Application initialization complete');
    showStatus('Evidence capture ready', 'success');
}

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    debugLog('DOM Content Loaded');
    initializeApplication();
});

// Export functions for HTML onclick handlers
window.removeFile = removeFile;
window.reopenExtractor = reopenExtractor;
window.clearTargetFrames = clearTargetFrames;
window.updateTargetStrip = updateTargetStrip;
window.toggleLiveRecording = toggleLiveRecording;
window.startRecording = startRecording;
window.stopRecording = stopRecording;
window.saveRecording = saveRecording;
window.cancelRecording = cancelRecording;
window.discardRecording = discardRecording;
window.captureEvidence = captureEvidence;

debugLog('Enhanced evidence capture script loaded and ready');