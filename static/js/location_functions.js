// ==================== LOCATION FUNCTIONS ====================

const kenyaCounties = [
    "Nairobi", "Mombasa", "Kisumu", "Nakuru", "Eldoret", "Thika", "Malindi", "Kitale",
    "Garissa", "Kakamega", "Kisii", "Meru", "Nyeri", "Machakos", "Kiambu", "Kilifi",
    "Bungoma", "Busia", "Embu", "Homa Bay", "Isiolo", "Kajiado", "Kericho", "Kirinyaga",
    "Kitui", "Kwale", "Laikipia", "Lamu", "Mandera", "Marsabit", "Migori", "Murang'a",
    "Nyamira", "Nyandarua", "Narok", "Samburu", "Siaya", "Taita Taveta", "Tana River",
    "Trans Nzoia", "Turkana", "Uasin Gishu", "Vihiga", "Wajir", "West Pokot"
];

// Show/hide location status
function showLocationStatus(message, type = 'info') {
    const statusDiv = document.getElementById('locationStatus');
    if (!statusDiv) return;
    
    const colors = {
        success: 'bg-green-900/20 border-green-700',
        error: 'bg-red-900/20 border-red-700',
        info: 'bg-blue-900/20 border-blue-700',
        warning: 'bg-yellow-900/20 border-yellow-700'
    };
    
    const icons = {
        success: 'fa-check-circle',
        error: 'fa-exclamation-triangle',
        info: 'fa-spinner fa-spin',
        warning: 'fa-exclamation-circle'
    };
    
    const colorClass = colors[type] || colors.info;
    const iconClass = icons[type] || icons.info;
    const textColor = type === 'success' ? 'text-green-400' : 
                     type === 'error' ? 'text-red-400' : 
                     type === 'warning' ? 'text-yellow-400' : 'text-blue-400';
    const title = type === 'success' ? 'Location Found!' : 
                  type === 'error' ? 'Location Error' : 
                  type === 'warning' ? 'Location Warning' : 'Detecting Location';
    
    statusDiv.className = 'mb-6 p-4 ' + colorClass + ' border rounded-lg';
    statusDiv.innerHTML = '<div class="flex items-center">' +
                          '<i class="fas ' + iconClass + ' ' + textColor + ' mr-3"></i>' +
                          '<div>' +
                          '<div class="font-medium">' + title + '</div>' +
                          '<div class="text-sm text-gray-300">' + message + '</div>' +
                          '</div>' +
                          '</div>';
    statusDiv.classList.remove('hidden');
}

// Hide location status
function hideLocationStatus() {
    const statusDiv = document.getElementById('locationStatus');
    if (statusDiv) {
        statusDiv.classList.add('hidden');
    }
}

// Find best matching county from Kenya counties
function findMatchingCounty(countyName) {
    if (!countyName) return '';
    
    const countyLower = countyName.toLowerCase().trim();
    
    // Exact match
    for (const county of kenyaCounties) {
        if (county.toLowerCase() === countyLower) {
            return county;
        }
    }
    
    // Partial match
    for (const county of kenyaCounties) {
        if (countyLower.includes(county.toLowerCase()) || 
            county.toLowerCase().includes(countyLower)) {
            return county;
        }
    }
    
    // Check for common aliases
    const aliases = {
        'nairobi city': 'Nairobi',
        'mombasa island': 'Mombasa',
        'kisumu town': 'Kisumu',
        'nakuru town': 'Nakuru',
        'eldoret town': 'Eldoret'
    };
    
    for (const alias in aliases) {
        if (countyLower.includes(alias)) {
            return aliases[alias];
        }
    }
    
    return '';
}

// Update form fields with location data AND populate hidden fields
function updateLocationFields(lat, lng, county, constituency, ward, landmark, source, city, region, country, accuracy) {
    county = county || '';
    constituency = constituency || '';
    ward = ward || '';
    landmark = landmark || '';
    source = source || '';
    city = city || constituency || ''; // Use constituency as fallback for city
    region = region || county || ''; // Use county as fallback for region
    country = country || 'Kenya'; // Default to Kenya
    accuracy = accuracy || 'medium';
    
    console.log('📍 Updating location fields:', { 
        lat: lat, lng: lng, county: county, constituency: constituency, 
        ward: ward, landmark: landmark, source: source,
        city: city, region: region, country: country, accuracy: accuracy
    });
    
    // Update visible coordinates
    const latInput = document.getElementById('latitudeInput');
    const lngInput = document.getElementById('longitudeInput');
    if (latInput) latInput.value = lat.toFixed(6);
    if (lngInput) lngInput.value = lng.toFixed(6);
    
    // Update county (find best match from Kenya counties)
    if (county) {
        const matchedCounty = findMatchingCounty(county);
        if (matchedCounty) {
            const countySelect = document.getElementById('countySelect');
            if (countySelect) countySelect.value = matchedCounty;
        }
    }
    
    // Update constituency
    if (constituency) {
        const constituencyInput = document.getElementById('constituencyInput');
        if (constituencyInput) constituencyInput.value = constituency;
    }
    
    // Update ward
    if (ward) {
        const wardInput = document.getElementById('wardInput');
        if (wardInput) wardInput.value = ward;
    }
    
    // Update landmark
    if (landmark) {
        const landmarkInput = document.getElementById('landmarkInput');
        if (landmarkInput) landmarkInput.value = landmark;
    }
    
    // ✅ UPDATE HIDDEN FIELDS FOR BACKEND
    const setHidden = (id, val) => { 
        const el = document.getElementById(id); 
        if (el) {
            el.value = val || '';
            console.log(`Set ${id} = ${val}`);
        } else {
            console.warn(`Hidden field ${id} not found`);
        }
    };
    
    setHidden('hiddenCity', city);
    setHidden('hiddenRegion', region);
    setHidden('hiddenCountry', country);
    setHidden('hiddenLocationAccuracy', accuracy);
    setHidden('hiddenLocationSource', source);
    setHidden('hiddenProxyDetected', 'false'); // Can be enhanced later
    
    // Show success message
    const locationText = county ? county + ' - ' + (constituency || 'Area') : 'Location: ' + lat.toFixed(6) + ', ' + lng.toFixed(6);
    showLocationStatus(locationText + ' (Source: ' + source + ', Country: ' + country + ')', 'success');
}

// Clear location fields
function clearLocationFields() {
    const countySelect = document.getElementById('countySelect');
    const constituencyInput = document.getElementById('constituencyInput');
    const wardInput = document.getElementById('wardInput');
    const landmarkInput = document.getElementById('landmarkInput');
    const latInput = document.getElementById('latitudeInput');
    const lngInput = document.getElementById('longitudeInput');
    
    if (countySelect) countySelect.value = '';
    if (constituencyInput) constituencyInput.value = '';
    if (wardInput) wardInput.value = '';
    if (landmarkInput) landmarkInput.value = '';
    if (latInput) latInput.value = '0.0';
    if (lngInput) lngInput.value = '0.0';
    
    // Clear hidden fields
    ['hiddenCity','hiddenRegion','hiddenCountry','hiddenLocationAccuracy','hiddenLocationSource','hiddenProxyDetected']
        .forEach(id => { 
            const el = document.getElementById(id); 
            if (el) el.value = ''; 
        });
    
    hideLocationStatus();
}

// Get real location using IP-based geolocation
async function fetchRealLocation() {
    showLocationStatus('Detecting your location from IP address...', 'info');
    
    try {
        console.log('🌍 Fetching real location from IP...');
        
        // Use IP-based geolocation
        const ipResponse = await fetch('https://ipapi.co/json/');
        
        if (!ipResponse.ok) {
            throw new Error('Failed to fetch IP location');
        }
        
        const ipData = await ipResponse.json();
        console.log('IP Location data:', ipData);
        
        const ipLatitude = ipData.latitude;
        const ipLongitude = ipData.longitude;
        const ipCity = ipData.city;
        const ipRegion = ipData.region;
        const ipCountry = ipData.country_name || 'Kenya';
        
        if (ipLatitude && ipLongitude) {
            // Update form with IP location AND hidden fields
            updateLocationFields(
                ipLatitude,
                ipLongitude,
                ipRegion,        // County
                ipCity,          // Constituency
                '',              // Ward
                '',              // Landmark
                'IP Address',    // Source
                ipCity,          // City (for hidden field)
                ipRegion,        // Region (for hidden field)
                ipCountry,       // Country (for hidden field)
                'medium'         // Accuracy
            );
            
            // Try to get more details via reverse geocoding
            await reverseGeocodeDetails(ipLatitude, ipLongitude, 'ip', ipCity, ipRegion, ipCountry);
            
        } else {
            throw new Error('No coordinates from IP');
        }
        
    } catch (error) {
        console.error('IP-based location failed:', error);
        showLocationStatus('IP location failed. Trying device GPS...', 'warning');
        // Fallback to browser geolocation
        getBrowserLocation();
    }
}

// Get location using browser geolocation
function getBrowserLocation() {
    showLocationStatus('Getting precise location from your device...', 'info');
    
    if (!navigator.geolocation) {
        showLocationStatus('Device geolocation not supported by browser', 'error');
        return;
    }
    
    navigator.geolocation.getCurrentPosition(
        async function(position) {
            const lat = position.coords.latitude;
            const lng = position.coords.longitude;
            const accuracy = position.coords.accuracy;
            
            console.log('Device GPS Location:', { lat: lat, lng: lng, accuracy: accuracy });
            
            // Update form with device location
            updateLocationFields(
                lat,
                lng,
                '',  // County (will be filled by reverse geocode)
                '',  // Constituency
                '',  // Ward
                '',  // Landmark
                'Device GPS', // Source
                '',  // City (will be filled by reverse geocode)
                '',  // Region (will be filled by reverse geocode)
                'Kenya', // Country (default)
                accuracy.toString() // Accuracy in meters
            );
            
            // Get detailed address via reverse geocoding
            await reverseGeocodeDetails(lat, lng, 'device', '', '', 'Kenya');
            
        },
        function(error) {
            console.error('Geolocation error:', error);
            const errorMessage = 
                error.code === 1 ? 'Permission denied. Please allow location access in browser settings.' :
                error.code === 2 ? 'Location unavailable. Please check your internet connection.' :
                error.code === 3 ? 'Location request timeout.' : 
                'Could not get device location.';
            
            showLocationStatus(errorMessage, 'error');
        },
        { 
            enableHighAccuracy: true,
            timeout: 10000,
            maximumAge: 0
        }
    );
}

// Reverse geocode to get detailed address
async function reverseGeocodeDetails(lat, lng, source, currentCity, currentRegion, currentCountry) {
    try {
        console.log('📍 Reverse geocoding for details...');
        
        // Try OpenStreetMap first (good for Kenya)
        const osmResponse = await fetch(
            'https://nominatim.openstreetmap.org/reverse?format=json&lat=' + lat + '&lon=' + lng + '&zoom=18&addressdetails=1'
        );
        
        if (osmResponse.ok) {
            const data = await osmResponse.json();
            console.log('OSM Geocoding result:', data);
            
            const address = data.address || {};
            
            // Extract Kenya-specific fields
            let county = address.county || address.state_district || address.state || currentRegion || '';
            let constituency = address.suburb || address.city_district || address.city || address.town || address.village || currentCity || '';
            let ward = address.neighbourhood || address.suburb || '';
            let landmark = address.road || address.house_number || '';
            let city = address.city || address.town || address.village || currentCity || constituency;
            let region = address.county || address.state || currentRegion || county;
            let country = address.country || currentCountry || 'Kenya';
            
            // Update form with detailed address AND hidden fields
            updateLocationFields(
                lat,
                lng,
                county,
                constituency,
                ward,
                landmark,
                source + ' + OSM',
                city,           // City for hidden field
                region,         // Region for hidden field
                country,        // Country for hidden field
                'high'          // Better accuracy with OSM
            );
            
            return;
        }
        
    } catch (error) {
        console.log('OSM geocoding failed:', error);
    }
    
    // Fallback to generic geocoding
    try {
        const response = await fetch(
            'https://api.bigdatacloud.net/data/reverse-geocode-client?latitude=' + lat + '&longitude=' + lng + '&localityLanguage=en'
        );
        
        if (response.ok) {
            const data = await response.json();
            console.log('Fallback geocoding result:', data);
            
            const county = data.principalSubdivision || currentRegion || '';
            const constituency = data.city || data.locality || currentCity || '';
            const city = data.city || data.locality || currentCity || constituency;
            const region = data.principalSubdivision || currentRegion || county;
            const country = data.countryName || currentCountry || 'Kenya';
            
            updateLocationFields(
                lat,
                lng,
                county,
                constituency,
                '',
                '',
                source + ' + Fallback',
                city,
                region,
                country,
                'medium'
            );
        }
    } catch (error) {
        console.log('Fallback geocoding failed:', error);
    }
}

// ==================== TARGET PHOTOS FUNCTIONS ====================

let targetPhotos = []; // Array to store ALL target photos
const MAX_TARGETS = 15;
let currentTargetId = 0;

function addTargetPhoto() {
    if (targetPhotos.length >= MAX_TARGETS) {
        alert('Maximum ' + MAX_TARGETS + ' target photos allowed');
        return;
    }
    
    currentTargetId++;
    const targetNumber = targetPhotos.length + 1;
    
    const targetsContainer = document.getElementById('targetsContainer');
    const targetId = 'target_' + Date.now() + '_' + currentTargetId;
    
    const targetHtml = `
    <div id="${targetId}" class="target-photo-item mb-6 p-4 bg-gray-900 rounded-lg border border-gray-700" data-target-index="${targetNumber}">
        <div class="flex items-center justify-between mb-4">
            <h4 class="font-bold text-lg">
                <i class="fas fa-bullseye mr-2"></i>
                Target Photo #${targetNumber}
            </h4>
            <button type="button" onclick="removeTargetPhoto('${targetId}')" 
                    class="text-red-400 hover:text-red-300 p-1">
                <i class="fas fa-times"></i>
            </button>
        </div>
        
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
                <label class="block text-sm font-medium mb-2">Target Description *</label>
                <input type="text" id="description_${targetId}" 
                       class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-purple-500" 
                       placeholder="e.g., 'Blue sedan car', 'Suspect in red jacket'" 
                       required>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Category *</label>
                <select id="category_${targetId}" 
                        class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-purple-500" 
                        required>
                    <option value="">Select Category</option>
                    <option value="person">👤 Person</option>
                    <option value="vehicle">🚗 Vehicle</option>
                    <option value="object">📦 Object</option>
                    <option value="location">📍 Location</option>
                    <option value="other">❓ Other</option>
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Confidence Level *</label>
                <select id="confidence_${targetId}" 
                        class="w-full px-4 py-3 bg-gray-800 border border-gray-700 rounded-lg focus:outline-none focus:border-purple-500" 
                        required>
                    <option value="">How confident are you?</option>
                    <option value="90">🔴 Very High (90%)</option>
                    <option value="75" selected>🟠 High (75%)</option>
                    <option value="50">🟡 Medium (50%)</option>
                    <option value="25">🟢 Low (25%)</option>
                    <option value="10">🔵 Very Low (10%)</option>
                </select>
            </div>
            
            <div>
                <label class="block text-sm font-medium mb-2">Photo File *</label>
                <div class="relative">
                    <input type="file" 
                           id="photo_${targetId}"
                           accept="image/*" 
                           class="hidden" 
                           onchange="previewTargetPhoto(this, '${targetId}')" 
                           required>
                    <div class="flex space-x-3">
                        <button type="button" 
                                onclick="document.getElementById('photo_${targetId}').click()" 
                                class="flex-1 bg-gray-700 px-4 py-3 rounded-lg hover:bg-gray-600">
                            <i class="fas fa-folder-open mr-2"></i> Select Photo
                        </button>
                        <div class="flex-1">
                            <div class="text-sm text-gray-400 mb-1">Max size: 5MB</div>
                            <div class="text-xs text-gray-500">PNG, JPG, JPEG, WEBP</div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
        
        <!-- Preview Area -->
        <div id="preview_${targetId}" class="hidden mt-4">
            <div class="bg-black rounded-lg p-2">
                <img id="img_preview_${targetId}" 
                     class="max-h-48 mx-auto rounded" 
                     alt="Preview">
                <div class="mt-2 text-center text-sm text-gray-400" id="file_info_${targetId}"></div>
                <div class="mt-2 flex justify-center space-x-2">
                    <button type="button" onclick="removeTargetPhoto('${targetId}')" 
                            class="text-xs bg-red-600 px-2 py-1 rounded hover:bg-red-700">
                        Remove
                    </button>
                    <button type="button" onclick="replaceTargetPhoto('${targetId}')" 
                            class="text-xs bg-blue-600 px-2 py-1 rounded hover:bg-blue-700">
                        Replace
                    </button>
                </div>
            </div>
        </div>
        
        <input type="hidden" id="photo_data_${targetId}" name="photo_data_${targetNumber}">
    </div>`;
    
    targetsContainer.insertAdjacentHTML('beforeend', targetHtml);
    
    // Add to tracking array
    targetPhotos.push({
        id: targetId,
        index: targetNumber,
        file: null,
        description: '',
        category: '',
        confidence: 75,
        dataUrl: '',
        fileName: '',
        fileSize: 0,
        mimeType: ''
    });
    
    console.log(`✅ Added target photo #${targetNumber}, total: ${targetPhotos.length}`);
}

function removeTargetPhoto(targetId) {
    console.log(`Removing target photo: ${targetId}`);
    
    // Remove from DOM
    const element = document.getElementById(targetId);
    if (element) {
        element.remove();
    }
    
    // Remove from tracking array
    const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
    if (targetIndex !== -1) {
        targetPhotos.splice(targetIndex, 1);
    }
    
    // Re-index remaining targets
    reindexTargets();
    
    console.log(`Remaining target photos: ${targetPhotos.length}`);
}

function replaceTargetPhoto(targetId) {
    const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
    if (targetIndex !== -1) {
        // Reset the file input
        const fileInput = document.getElementById(`photo_${targetId}`);
        if (fileInput) {
            fileInput.value = '';
        }
        
        // Reset preview
        const previewDiv = document.getElementById(`preview_${targetId}`);
        if (previewDiv) {
            previewDiv.classList.add('hidden');
        }
        
        // Reset in tracking array
        targetPhotos[targetIndex].file = null;
        targetPhotos[targetIndex].dataUrl = '';
        targetPhotos[targetIndex].fileName = '';
        targetPhotos[targetIndex].fileSize = 0;
        targetPhotos[targetIndex].mimeType = '';
        
        console.log(`Target photo ${targetId} reset for replacement`);
    }
}

function reindexTargets() {
    const targetItems = document.querySelectorAll('.target-photo-item');
    
    targetItems.forEach((item, index) => {
        const newIndex = index + 1;
        const targetId = item.id;
        
        // Update title
        const title = item.querySelector('h4');
        if (title) {
            title.innerHTML = `<i class="fas fa-bullseye mr-2"></i>Target Photo #${newIndex}`;
        }
        
        // Update data attribute
        item.setAttribute('data-target-index', newIndex.toString());
        
        // Update tracking array
        const targetInArray = targetPhotos.find(t => t.id === targetId);
        if (targetInArray) {
            targetInArray.index = newIndex;
        }
    });
    
    console.log(`Re-indexed ${targetItems.length} target photos`);
}

async function previewTargetPhoto(input, targetId) {
    const file = input.files[0];
    if (!file) {
        console.log('No file selected');
        return;
    }
    
    console.log(`Processing target photo: ${file.name} (${formatBytes(file.size)})`);
    
    // Validate file size (5MB max)
    const maxSize = 5 * 1024 * 1024; // 5MB
    if (file.size > maxSize) {
        alert('File too large. Maximum size is 5MB.');
        input.value = '';
        return;
    }
    
    // Validate file type
    const validTypes = ['image/jpeg', 'image/png', 'image/jpg', 'image/webp', 'image/gif'];
    if (!validTypes.includes(file.type.toLowerCase())) {
        alert('Please select a valid image file (PNG, JPG, JPEG, WEBP, GIF)');
        input.value = '';
        return;
    }
    
    // Find target in tracking array
    const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
    if (targetIndex === -1) {
        console.error(`Target ${targetId} not found in tracking array`);
        return;
    }
    
    // Update tracking array
    targetPhotos[targetIndex].file = file;
    targetPhotos[targetIndex].fileName = file.name;
    targetPhotos[targetIndex].fileSize = file.size;
    targetPhotos[targetIndex].mimeType = file.type;
    
    // Get description, category, and confidence from form
    const descriptionInput = document.getElementById(`description_${targetId}`);
    const categorySelect = document.getElementById(`category_${targetId}`);
    const confidenceSelect = document.getElementById(`confidence_${targetId}`);
    
    if (descriptionInput) {
        targetPhotos[targetIndex].description = descriptionInput.value;
    }
    if (categorySelect) {
        targetPhotos[targetIndex].category = categorySelect.value;
    }
    if (confidenceSelect) {
        targetPhotos[targetIndex].confidence = parseInt(confidenceSelect.value) || 75;
    }
    
    // Show preview
    const previewDiv = document.getElementById(`preview_${targetId}`);
    const previewImg = document.getElementById(`img_preview_${targetId}`);
    const fileInfo = document.getElementById(`file_info_${targetId}`);
    
    if (previewDiv && previewImg && fileInfo) {
        const reader = new FileReader();
        reader.onload = function(e) {
            targetPhotos[targetIndex].dataUrl = e.target.result;
            previewImg.src = e.target.result;
            previewDiv.classList.remove('hidden');
            fileInfo.textContent = `${file.name} (${formatBytes(file.size)})`;
            
            console.log(`✅ Target photo #${targetPhotos[targetIndex].index} processed successfully`);
            console.log(`   Description: ${targetPhotos[targetIndex].description}`);
            console.log(`   Category: ${targetPhotos[targetIndex].category}`);
            console.log(`   Confidence: ${targetPhotos[targetIndex].confidence}%`);
            console.log(`   File: ${targetPhotos[targetIndex].fileName}`);
        };
        reader.onerror = function(error) {
            console.error('Error reading file:', error);
            alert('Error reading file. Please try another image.');
        };
        reader.readAsDataURL(file);
    }
    
    // Validate required fields
    validateTargetPhoto(targetId);
}

function validateTargetPhoto(targetId) {
    const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
    if (targetIndex === -1) return false;
    
    const target = targetPhotos[targetIndex];
    const descriptionInput = document.getElementById(`description_${targetId}`);
    const categorySelect = document.getElementById(`category_${targetId}`);
    
    let isValid = true;
    
    if (!target.file) {
        isValid = false;
        console.log(`Target ${targetId}: Missing file`);
    }
    
    if (descriptionInput && !descriptionInput.value.trim()) {
        isValid = false;
        console.log(`Target ${targetId}: Missing description`);
    }
    
    if (categorySelect && !categorySelect.value) {
        isValid = false;
        console.log(`Target ${targetId}: Missing category`);
    }
    
    return isValid;
}

function validateAllTargets() {
    console.log('Validating all target photos...');
    
    let allValid = true;
    const invalidTargets = [];
    
    targetPhotos.forEach((target, index) => {
        const targetNumber = index + 1;
        
        if (!validateTargetPhoto(target.id)) {
            allValid = false;
            invalidTargets.push(targetNumber);
            
            // Highlight invalid fields
            const targetElement = document.getElementById(target.id);
            if (targetElement) {
                targetElement.classList.add('border-red-500');
                
                setTimeout(() => {
                    targetElement.classList.remove('border-red-500');
                }, 2000);
            }
        }
    });
    
    if (!allValid && invalidTargets.length > 0) {
        console.warn(`Invalid target photos: ${invalidTargets.join(', ')}`);
    }
    
    console.log(`Validation result: ${allValid ? 'All valid' : `${invalidTargets.length} invalid target(s)`}`);
    
    return {
        allValid,
        invalidTargets,
        totalTargets: targetPhotos.length
    };
}

// Helper function to convert file to base64 (data URL to raw base64)
async function fileToBase64(file) {
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.readAsDataURL(file);
        reader.onload = () => {
            // Convert data URL to raw base64 (remove "data:image/png;base64," prefix)
            const dataUrl = reader.result;
            const base64 = dataUrl.split(',')[1];
            resolve(base64);
        };
        reader.onerror = error => reject(error);
    });
}

function formatBytes(bytes) {
    if (bytes < 1024) return bytes + ' B';
    else if (bytes < 1048576) return (bytes / 1024).toFixed(2) + ' KB';
    else return (bytes / 1048576).toFixed(2) + ' MB';
}

// Get all target photos as an array
async function getAllTargetPhotos() {
    console.log('Collecting all target photos...');
    
    const targetPhotosArray = [];
    let successCount = 0;
    let errorCount = 0;
    
    for (let i = 0; i < targetPhotos.length; i++) {
        const target = targetPhotos[i];
        
        try {
            // Validate required fields
            if (!target.file) {
                console.error(`Target #${target.index}: No file selected`);
                errorCount++;
                continue;
            }
            
            if (!target.description || !target.category) {
                console.error(`Target #${target.index}: Missing description or category`);
                errorCount++;
                continue;
            }
            
            // Convert file to base64
            const base64Data = await fileToBase64(target.file);
            
            // Create target photo object
            const targetPhoto = {
                filename: target.fileName,
                mime_type: target.mimeType,
                data_base64: base64Data,
                description: target.description,
                category: target.category,
                confidence_score: target.confidence
            };
            
            targetPhotosArray.push(targetPhoto);
            successCount++;
            
            console.log(`✅ Target #${target.index} prepared: ${target.fileName}`);
            
        } catch (error) {
            console.error(`❌ Error preparing target #${target.index}:`, error);
            errorCount++;
        }
    }
    
    console.log(`Target photos collection complete:`);
    console.log(`  Success: ${successCount}`);
    console.log(`  Errors: ${errorCount}`);
    console.log(`  Total: ${targetPhotos.length}`);
    
    return {
        photos: targetPhotosArray,
        successCount,
        errorCount,
        totalCount: targetPhotos.length
    };
}

// Clear all target photos
function clearAllTargetPhotos() {
    console.log('Clearing all target photos...');
    
    // Clear tracking array
    targetPhotos = [];
    currentTargetId = 0;
    
    // Clear DOM
    const targetsContainer = document.getElementById('targetsContainer');
    if (targetsContainer) {
        targetsContainer.innerHTML = '';
    }
    
    // Hide upload progress
    const progressContainer = document.getElementById('targetsUploadProgress');
    if (progressContainer) {
        progressContainer.classList.add('hidden');
    }
    
    console.log('All target photos cleared');
}

// Show target upload status
function showTargetUploadStatus(message, type = 'info') {
    const statusDiv = document.getElementById('targetsUploadStatus');
    if (!statusDiv) return;
    
    const colors = {
        success: 'text-green-400',
        error: 'text-red-400',
        info: 'text-blue-400',
        warning: 'text-yellow-400'
    };
    
    const icons = {
        success: 'fa-check-circle',
        error: 'fa-exclamation-circle',
        info: 'fa-info-circle',
        warning: 'fa-exclamation-triangle'
    };
    
    statusDiv.className = `mt-2 text-sm ${colors[type] || colors.info}`;
    statusDiv.innerHTML = `<i class="fas ${icons[type] || icons.info} mr-2"></i>${message}`;
}

// Update target upload progress
function updateTargetProgress(progress, message) {
    const progressBar = document.getElementById('targetsProgressBar');
    const progressPercent = document.getElementById('targetsProgressPercent');
    const statusDiv = document.getElementById('targetsUploadStatus');
    
    if (progressBar) {
        progressBar.style.width = `${progress}%`;
    }
    
    if (progressPercent) {
        progressPercent.textContent = `${Math.round(progress)}%`;
    }
    
    if (statusDiv) {
        showTargetUploadStatus(message, 'info');
    }
}

// Initialize target photos
function initializeTargetPhotos() {
    console.log('Initializing target photos system...');
    
    // Clear any existing state
    clearAllTargetPhotos();
    
    // Add event listeners for form validation
    document.addEventListener('input', function(event) {
        if (event.target.id && event.target.id.startsWith('description_')) {
            const targetId = event.target.id.replace('description_', '');
            const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
            if (targetIndex !== -1) {
                targetPhotos[targetIndex].description = event.target.value;
            }
        }
    });
    
    document.addEventListener('change', function(event) {
        if (event.target.id && event.target.id.startsWith('category_')) {
            const targetId = event.target.id.replace('category_', '');
            const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
            if (targetIndex !== -1) {
                targetPhotos[targetIndex].category = event.target.value;
            }
        }
        
        if (event.target.id && event.target.id.startsWith('confidence_')) {
            const targetId = event.target.id.replace('confidence_', '');
            const targetIndex = targetPhotos.findIndex(t => t.id === targetId);
            if (targetIndex !== -1) {
                targetPhotos[targetIndex].confidence = parseInt(event.target.value) || 75;
            }
        }
    });
    
    console.log('Target photos system initialized');
}

// ==================== COMPLETE EVIDENCE FUNCTION ====================

async function completeEvidence(event) {
    event.preventDefault();
    
    console.log('=== COMPLETE EVIDENCE PROCESS STARTED ===');
    
    const form = document.getElementById('completeForm');
    const formData = new FormData(form);
    const evidenceId = formData.get('evidence_id');
    
    console.log(`Evidence ID: ${evidenceId}`);
    console.log(`Target photos in tracking array: ${targetPhotos.length}`);
    
    // Step 1: Validate main form
    if (!form.checkValidity()) {
        alert('Please fill in all required fields in the main form.');
        form.reportValidity();
        return;
    }
    
    // Step 2: Validate target photos (if any)
    const targetValidation = validateAllTargets();
    if (targetPhotos.length > 0 && !targetValidation.allValid) {
        const confirmed = confirm(`You have ${targetValidation.invalidTargets.length} incomplete target photo(s). Do you want to proceed without them?`);
        if (!confirmed) {
            return;
        }
    }
    
    // Step 3: Prepare target photos (if any)
    let targetPhotosData = [];
    if (targetPhotos.length > 0) {
        console.log('Preparing target photos for upload...');
        
        // Show upload progress
        const progressContainer = document.getElementById('targetsUploadProgress');
        if (progressContainer) {
            progressContainer.classList.remove('hidden');
        }
        
        updateTargetProgress(10, 'Preparing target photos...');
        
        const result = await getAllTargetPhotos();
        
        if (result.photos.length === 0) {
            console.warn('No valid target photos to upload');
            showTargetUploadStatus('No valid target photos to upload', 'warning');
        } else {
            targetPhotosData = result.photos;
            updateTargetProgress(30, `${result.photos.length} target photos prepared`);
            console.log(`✅ ${result.photos.length} target photos prepared for upload`);
        }
    }
    
    // Step 4: Submit main evidence form
    const submitBtn = document.getElementById('submitBtn');
    if (!submitBtn) {
        console.error('Submit button not found');
        return;
    }
    
    // Save original button state
    const originalBtnText = submitBtn.innerHTML;
    const originalBtnDisabled = submitBtn.disabled;
    
    // Update UI
    submitBtn.disabled = true;
    submitBtn.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i> Submitting Evidence...';
    
    try {
        console.log('Submitting main evidence form...');
        
        // Step 4a: Submit main evidence
        const response = await fetch('/api/evidence/complete', {
            method: 'POST',
            body: formData
        });
        
        if (!response.ok) {
            throw new Error(`Server error: ${response.status} ${response.statusText}`);
        }
        
        const result = await response.json();
        console.log('Main evidence response:', result);
        
        if (!result.success) {
            throw new Error(result.message || 'Evidence submission failed');
        }
        
        console.log('✅ Main evidence submitted successfully');
        console.log(`   Evidence ID: ${result.data.id}`);
        console.log(`   Evidence Number: ${result.data.evidence_number}`);
        
        // Step 4b: Upload target photos (if any)
        if (targetPhotosData.length > 0) {
            console.log(`📤 Uploading ${targetPhotosData.length} target photos...`);
            updateTargetProgress(50, 'Uploading target photos...');
            
            try {
                const targetsResponse = await fetch('/api/evidence/targets/upload', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        evidence_id: evidenceId,
                        photos: targetPhotosData
                    })
                });
                
                if (!targetsResponse.ok) {
                    throw new Error(`Target upload error: ${targetsResponse.status}`);
                }
                
                const targetsResult = await targetsResponse.json();
                console.log('Target photos response:', targetsResult);
                
                if (targetsResult.success) {
                    updateTargetProgress(100, `${targetPhotosData.length} target photos uploaded successfully!`);
                    showTargetUploadStatus(`✅ ${targetPhotosData.length} target photos uploaded`, 'success');
                    
                    console.log(`✅ ${targetPhotosData.length} target photos uploaded successfully`);
                    console.log('   First target preview:', targetsResult.data[0]?.storj_url || 'No preview URL');
                } else {
                    console.warn('Target photos upload failed:', targetsResult.message);
                    showTargetUploadStatus(`⚠️ Evidence submitted but target photos failed: ${targetsResult.message}`, 'warning');
                    updateTargetProgress(80, 'Target photos upload failed');
                }
                
            } catch (targetError) {
                console.error('Target photos upload error:', targetError);
                showTargetUploadStatus(`⚠️ Evidence submitted but target photos upload failed: ${targetError.message}`, 'warning');
                // Don't fail the entire submission because of target photos
            }
        } else {
            console.log('No target photos to upload');
            updateTargetProgress(100, 'No target photos to upload');
        }
        
        // Step 5: Show success message
        const successHtml = `
        <div class="p-6 bg-green-900/20 border border-green-700 rounded-lg">
            <div class="flex items-center mb-4">
                <i class="fas fa-check-circle text-green-400 text-2xl mr-3"></i>
                <div>
                    <h3 class="font-bold text-lg">Evidence Submitted Successfully!</h3>
                    <p class="text-gray-300">Evidence Number: ${result.data.evidence_number}</p>
                    <p class="text-sm text-gray-400 mt-1">Status: ${result.data.status || 'Submitted'}</p>
                    ${targetPhotosData.length > 0 ? 
                        `<p class="text-sm text-purple-400 mt-1">
                            <i class="fas fa-bullseye mr-1"></i>
                            ${targetPhotosData.length} target photos uploaded
                        </p>` : ''}
                </div>
            </div>
            <div class="space-y-2 text-sm mb-4">
                <div class="flex justify-between">
                    <span class="text-gray-400">Title:</span>
                    <span>${result.data.title}</span>
                </div>
                <div class="flex justify-between">
                    <span class="text-gray-400">Location:</span>
                    <span>${result.data.location?.county || 'Unknown'}</span>
                </div>
                <div class="flex justify-between">
                    <span class="text-gray-400">Submitted:</span>
                    <span>${new Date().toLocaleString()}</span>
                </div>
            </div>
            <div class="flex space-x-4">
                <a href="/evidence/view/${result.data.id}" 
                   class="flex-1 bg-blue-600 py-3 rounded text-center hover:bg-blue-700 font-medium">
                    <i class="fas fa-eye mr-2"></i>View Evidence
                </a>
                <a href="/evidence/my" 
                   class="flex-1 bg-gray-700 py-3 rounded text-center hover:bg-gray-600">
                    <i class="fas fa-list mr-2"></i>My Evidence
                </a>
            </div>
        </div>`;
        
        const statusMessages = document.getElementById('statusMessages');
        if (statusMessages) {
            statusMessages.innerHTML = successHtml;
        }
        
        // Hide submit button
        submitBtn.classList.add('hidden');
        
        // Log final success
        console.log('=== EVIDENCE SUBMISSION COMPLETE ===');
        console.log(`✅ Evidence: ${result.data.evidence_number}`);
        console.log(`✅ Title: ${result.data.title}`);
        console.log(`✅ Location: ${result.data.location?.county || 'Unknown'}`);
        console.log(`✅ Target Photos: ${targetPhotosData.length}`);
        console.log('✅ Redirecting to view page in 5 seconds...');
        
        // Redirect after 5 seconds
        setTimeout(function() {
            window.location.href = `/evidence/view/${result.data.id}`;
        }, 5000);
        
    } catch (error) {
        console.error('Evidence submission failed:', error);
        
        // Show error message
        const errorHtml = `
        <div class="p-4 bg-red-900/20 border border-red-700 rounded-lg">
            <div class="flex items-center">
                <i class="fas fa-exclamation-circle text-red-400 mr-3"></i>
                <div>
                    <h4 class="font-bold">Submission Failed</h4>
                    <p class="text-sm text-gray-300">${error.message}</p>
                </div>
            </div>
            <div class="mt-3">
                <button onclick="retrySubmission()" 
                        class="bg-red-600 px-4 py-2 rounded hover:bg-red-700 mr-2">
                    <i class="fas fa-redo mr-1"></i>Retry
                </button>
                <button onclick="clearForm()" 
                        class="bg-gray-600 px-4 py-2 rounded hover:bg-gray-700">
                    <i class="fas fa-times mr-1"></i>Clear Form
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
    if (form) {
        form.reset();
    }
    clearAllTargetPhotos();
    
    const statusMessages = document.getElementById('statusMessages');
    if (statusMessages) {
        statusMessages.innerHTML = '';
    }
    
    const submitBtn = document.getElementById('submitBtn');
    if (submitBtn) {
        submitBtn.classList.remove('hidden');
        submitBtn.disabled = false;
        submitBtn.innerHTML = '<i class="fas fa-paper-plane mr-2"></i> Submit Evidence';
    }
}

// ==================== INITIALIZATION ====================

// Initialize when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    console.log('DOM loaded, initializing systems...');
    
    // Initialize location detection
    setTimeout(function() {
        fetchRealLocation();
    }, 1000);
    
    // Initialize target photos system
    initializeTargetPhotos();
    
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
window.addTargetPhoto = addTargetPhoto;
window.removeTargetPhoto = removeTargetPhoto;
window.previewTargetPhoto = previewTargetPhoto;
window.replaceTargetPhoto = replaceTargetPhoto;
window.clearAllTargetPhotos = clearAllTargetPhotos;
window.completeEvidence = completeEvidence;
window.retrySubmission = retrySubmission;
window.clearForm = clearForm;