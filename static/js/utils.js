// Utility Functions for FLUG Evidence System

// ==================== GENERAL UTILITIES ====================

/**
 * Format file size from bytes to human readable string
 * @param {number} bytes - File size in bytes
 * @returns {string} Formatted file size
 */
function formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Format date to readable string
 * @param {Date|string} date - Date to format
 * @param {boolean} includeTime - Whether to include time
 * @returns {string} Formatted date
 */
function formatDate(date, includeTime = true) {
    const d = new Date(date);
    const options = {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    };
    
    if (includeTime) {
        options.hour = '2-digit';
        options.minute = '2-digit';
    }
    
    return d.toLocaleDateString('en-US', options);
}

/**
 * Truncate text with ellipsis
 * @param {string} text - Text to truncate
 * @param {number} maxLength - Maximum length
 * @returns {string} Truncated text
 */
function truncateText(text, maxLength = 100) {
    if (text.length <= maxLength) return text;
    return text.substring(0, maxLength - 3) + '...';
}

/**
 * Debounce function to limit how often a function can be called
 * @param {Function} func - Function to debounce
 * @param {number} wait - Wait time in milliseconds
 * @returns {Function} Debounced function
 */
function debounce(func, wait) {
    let timeout;
    return function executedFunction(...args) {
        const later = () => {
            clearTimeout(timeout);
            func(...args);
        };
        clearTimeout(timeout);
        timeout = setTimeout(later, wait);
    };
}

/**
 * Throttle function to limit function execution rate
 * @param {Function} func - Function to throttle
 * @param {number} limit - Time limit in milliseconds
 * @returns {Function} Throttled function
 */
function throttle(func, limit) {
    let inThrottle;
    return function() {
        const args = arguments;
        const context = this;
        if (!inThrottle) {
            func.apply(context, args);
            inThrottle = true;
            setTimeout(() => inThrottle = false, limit);
        }
    };
}

// ==================== DOM UTILITIES ====================

/**
 * Show loading spinner
 * @param {HTMLElement} element - Element to show spinner in
 * @param {string} message - Optional loading message
 */
function showLoading(element, message = 'Loading...') {
    const spinner = document.createElement('div');
    spinner.className = 'loading-spinner';
    spinner.innerHTML = `
        <div class="flex items-center justify-center space-x-2">
            <div class="w-4 h-4 border-2 border-red-600 border-t-transparent rounded-full animate-spin"></div>
            <span class="text-sm text-gray-400">${message}</span>
        </div>
    `;
    
    element.innerHTML = '';
    element.appendChild(spinner);
    element.classList.add('loading');
}

/**
 * Hide loading spinner
 * @param {HTMLElement} element - Element with spinner
 */
function hideLoading(element) {
    element.classList.remove('loading');
}

/**
 * Show toast notification
 * @param {string} message - Message to display
 * @param {string} type - Type of toast (success, error, warning, info)
 * @param {number} duration - Duration in milliseconds
 */
function showToast(message, type = 'info', duration = 3000) {
    // Remove existing toasts
    const existingToasts = document.querySelectorAll('.toast-notification');
    existingToasts.forEach(toast => {
        if (toast.parentElement) {
            toast.parentElement.removeChild(toast);
        }
    });
    
    // Create toast
    const toast = document.createElement('div');
    toast.className = `toast-notification fixed top-4 right-4 z-50 transform transition-transform duration-300 translate-x-full`;
    
    const typeClasses = {
        success: 'bg-green-900 border-green-700',
        error: 'bg-red-900 border-red-700',
        warning: 'bg-yellow-900 border-yellow-700',
        info: 'bg-blue-900 border-blue-700'
    };
    
    const icons = {
        success: 'fa-check-circle',
        error: 'fa-exclamation-circle',
        warning: 'fa-exclamation-triangle',
        info: 'fa-info-circle'
    };
    
    toast.innerHTML = `
        <div class="${typeClasses[type]} border rounded-lg p-4 max-w-sm shadow-xl">
            <div class="flex items-start">
                <div class="flex-shrink-0">
                    <i class="fas ${icons[type]} text-${type === 'success' ? 'green' : type === 'error' ? 'red' : type === 'warning' ? 'yellow' : 'blue'}-400"></i>
                </div>
                <div class="ml-3 flex-1">
                    <p class="text-sm font-medium text-white">${message}</p>
                </div>
                <button class="ml-4 flex-shrink-0 text-gray-400 hover:text-white close-toast">
                    <i class="fas fa-times"></i>
                </button>
            </div>
        </div>
    `;
    
    document.body.appendChild(toast);
    
    // Animate in
    setTimeout(() => {
        toast.classList.remove('translate-x-full');
    }, 10);
    
    // Close button
    toast.querySelector('.close-toast').addEventListener('click', () => {
        hideToast(toast);
    });
    
    // Auto-hide
    if (duration > 0) {
        setTimeout(() => {
            hideToast(toast);
        }, duration);
    }
    
    return toast;
}

/**
 * Hide toast notification
 * @param {HTMLElement} toast - Toast element to hide
 */
function hideToast(toast) {
    toast.classList.add('translate-x-full');
    setTimeout(() => {
        if (toast.parentElement) {
            toast.parentElement.removeChild(toast);
        }
    }, 300);
}

/**
 * Copy text to clipboard
 * @param {string} text - Text to copy
 * @param {string} successMessage - Message to show on success
 */
async function copyToClipboard(text, successMessage = 'Copied to clipboard!') {
    try {
        await navigator.clipboard.writeText(text);
        showToast(successMessage, 'success');
    } catch (err) {
        console.error('Failed to copy:', err);
        showToast('Failed to copy to clipboard', 'error');
    }
}

/**
 * Download file from URL
 * @param {string} url - File URL
 * @param {string} filename - Desired filename
 */
function downloadFile(url, filename) {
    const link = document.createElement('a');
    link.href = url;
    link.download = filename;
    link.target = '_blank';
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
}

// ==================== FORM UTILITIES ====================

/**
 * Validate email address
 * @param {string} email - Email to validate
 * @returns {boolean} Whether email is valid
 */
function validateEmail(email) {
    const re = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    return re.test(email);
}

/**
 * Validate phone number (Kenyan format)
 * @param {string} phone - Phone number to validate
 * @returns {boolean} Whether phone is valid
 */
function validatePhone(phone) {
    const re = /^(?:254|\+254|0)?(7[0-9]{8})$/;
    return re.test(phone.replace(/\s+/g, ''));
}

/**
 * Validate Kenyan ID number
 * @param {string} idNumber - ID number to validate
 * @returns {boolean} Whether ID is valid
 */
function validateKenyanID(idNumber) {
    if (!/^\d{8}$/.test(idNumber)) return false;
    
    // Basic validation - in production, use proper algorithm
    return true;
}

/**
 * Serialize form data to object
 * @param {HTMLFormElement} form - Form element
 * @returns {Object} Form data as object
 */
function serializeForm(form) {
    const formData = new FormData(form);
    const data = {};
    
    for (const [key, value] of formData.entries()) {
        if (data[key]) {
            if (Array.isArray(data[key])) {
                data[key].push(value);
            } else {
                data[key] = [data[key], value];
            }
        } else {
            data[key] = value;
        }
    }
    
    return data;
}

/**
 * Clear form validation errors
 * @param {HTMLFormElement} form - Form element
 */
function clearFormErrors(form) {
    const errorElements = form.querySelectorAll('.error-message, .border-red-500');
    errorElements.forEach(element => {
        if (element.classList.contains('error-message')) {
            element.remove();
        } else {
            element.classList.remove('border-red-500');
        }
    });
}

/**
 * Show form field error
 * @param {HTMLElement} field - Form field element
 * @param {string} message - Error message
 */
function showFieldError(field, message) {
    // Remove existing error
    const existingError = field.parentElement.querySelector('.error-message');
    if (existingError) {
        existingError.remove();
    }
    
    // Add error class to field
    field.classList.add('border-red-500');
    
    // Create error message
    const error = document.createElement('div');
    error.className = 'error-message text-red-400 text-sm mt-1';
    error.textContent = message;
    
    field.parentElement.appendChild(error);
    
    // Focus field
    field.focus();
}

// ==================== API UTILITIES ====================

/**
 * Make API request with error handling
 * @param {string} url - API endpoint
 * @param {Object} options - Fetch options
 * @returns {Promise} Promise with response data
 */
async function apiRequest(url, options = {}) {
    const defaultOptions = {
        headers: {
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        },
        credentials: 'same-origin'
    };
    
    const mergedOptions = { ...defaultOptions, ...options };
    
    try {
        const response = await fetch(url, mergedOptions);
        
        // Check if response is JSON
        const contentType = response.headers.get('content-type');
        const isJson = contentType && contentType.includes('application/json');
        
        if (!response.ok) {
            if (isJson) {
                const errorData = await response.json();
                throw new Error(errorData.message || `HTTP ${response.status}`);
            } else {
                throw new Error(`HTTP ${response.status}`);
            }
        }
        
        return isJson ? await response.json() : await response.text();
    } catch (error) {
        console.error('API request failed:', error);
        showToast(`Request failed: ${error.message}`, 'error');
        throw error;
    }
}

/**
 * Upload file with progress tracking
 * @param {string} url - Upload endpoint
 * @param {File} file - File to upload
 * @param {Object} additionalData - Additional form data
 * @param {Function} onProgress - Progress callback
 * @returns {Promise} Upload promise
 */
async function uploadFile(url, file, additionalData = {}, onProgress = null) {
    const formData = new FormData();
    formData.append('file', file);
    
    // Add additional data
    Object.entries(additionalData).forEach(([key, value]) => {
        formData.append(key, value);
    });
    
    return new Promise((resolve, reject) => {
        const xhr = new XMLHttpRequest();
        
        // Progress tracking
        if (onProgress) {
            xhr.upload.addEventListener('progress', (event) => {
                if (event.lengthComputable) {
                    const percentComplete = (event.loaded / event.total) * 100;
                    onProgress(percentComplete);
                }
            });
        }
        
        // Load and error handlers
        xhr.addEventListener('load', () => {
            if (xhr.status >= 200 && xhr.status < 300) {
                try {
                    const response = JSON.parse(xhr.responseText);
                    resolve(response);
                } catch (e) {
                    resolve(xhr.responseText);
                }
            } else {
                reject(new Error(`Upload failed: ${xhr.statusText}`));
            }
        });
        
        xhr.addEventListener('error', () => {
            reject(new Error('Network error during upload'));
        });
        
        xhr.open('POST', url);
        xhr.send(formData);
    });
}

// ==================== SECURITY UTILITIES ====================

/**
 * Sanitize HTML to prevent XSS
 * @param {string} html - HTML to sanitize
 * @returns {string} Sanitized HTML
 */
function sanitizeHTML(html) {
    const temp = document.createElement('div');
    temp.textContent = html;
    return temp.innerHTML;
}

/**
 * Generate random string
 * @param {number} length - Length of string
 * @returns {string} Random string
 */
function generateRandomString(length = 32) {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    const crypto = window.crypto || window.msCrypto;
    
    if (crypto && crypto.getRandomValues) {
        const values = new Uint32Array(length);
        crypto.getRandomValues(values);
        for (let i = 0; i < length; i++) {
            result += chars[values[i] % chars.length];
        }
    } else {
        // Fallback for browsers without crypto support
        for (let i = 0; i < length; i++) {
            result += chars[Math.floor(Math.random() * chars.length)];
        }
    }
    
    return result;
}

// ==================== UI UTILITIES ====================

/**
 * Toggle element visibility
 * @param {HTMLElement} element - Element to toggle
 * @param {boolean} force - Force show/hide (optional)
 */
function toggleVisibility(element, force) {
    if (force !== undefined) {
        element.classList.toggle('hidden', !force);
    } else {
        element.classList.toggle('hidden');
    }
}

/**
 * Animate element with fade in
 * @param {HTMLElement} element - Element to animate
 * @param {string} animationClass - CSS animation class
 */
function animateElement(element, animationClass = 'fade-in') {
    element.classList.add(animationClass);
    
    // Remove animation class after animation completes
    const animationDuration = 500; // Match CSS animation duration
    setTimeout(() => {
        element.classList.remove(animationClass);
    }, animationDuration);
}

/**
 * Create modal dialog
 * @param {string} title - Modal title
 * @param {string} content - Modal content (HTML)
 * @param {Object} options - Modal options
 * @returns {HTMLElement} Modal element
 */
function createModal(title, content, options = {}) {
    const {
        size = 'md',
        showCloseButton = true,
        showFooter = true,
        buttons = []
    } = options;
    
    const modal = document.createElement('div');
    modal.className = 'fixed inset-0 bg-black/70 flex items-center justify-center z-50 modal-overlay';
    modal.innerHTML = `
        <div class="bg-gray-800 rounded-lg border border-gray-700 modal-content w-full max-w-${size} mx-4">
            <div class="p-4 border-b border-gray-700 flex justify-between items-center">
                <h3 class="text-xl font-bold">${title}</h3>
                ${showCloseButton ? 
                    `<button class="text-gray-400 hover:text-white close-modal">
                        <i class="fas fa-times text-2xl"></i>
                    </button>` : ''
                }
            </div>
            <div class="p-4">${content}</div>
            ${showFooter ? `
                <div class="p-4 border-t border-gray-700 flex justify-end space-x-3">
                    ${buttons.map(btn => 
                        `<button class="px-4 py-2 rounded-lg ${btn.className || 'bg-gray-700 hover:bg-gray-600'}">
                            ${btn.text}
                        </button>`
                    ).join('')}
                </div>
            ` : ''}
        </div>
    `;
    
    document.body.appendChild(modal);
    
    // Close button functionality
    if (showCloseButton) {
        modal.querySelector('.close-modal').addEventListener('click', () => {
            modal.remove();
        });
    }
    
    // Close on background click
    modal.addEventListener('click', (e) => {
        if (e.target === modal) {
            modal.remove();
        }
    });
    
    // Close on Escape key
    const closeOnEscape = (e) => {
        if (e.key === 'Escape') {
            modal.remove();
            document.removeEventListener('keydown', closeOnEscape);
        }
    };
    document.addEventListener('keydown', closeOnEscape);
    
    return modal;
}

// ==================== EVIDENCE-SPECIFIC UTILITIES ====================

/**
 * Get emergency level color class
 * @param {string} level - Emergency level
 * @returns {Object} Background and text color classes
 */
function getEmergencyLevelColors(level) {
    const colors = {
        red: { bg: 'bg-red-900', text: 'text-red-200', border: 'border-red-700' },
        orange: { bg: 'bg-orange-900', text: 'text-orange-200', border: 'border-orange-700' },
        yellow: { bg: 'bg-yellow-900', text: 'text-yellow-200', border: 'border-yellow-700' },
        blue: { bg: 'bg-blue-900', text: 'text-blue-200', border: 'border-blue-700' }
    };
    
    return colors[level.toLowerCase()] || colors.blue;
}

/**
 * Get evidence status color class
 * @param {string} status - Evidence status
 * @returns {Object} Background and text color classes
 */
function getEvidenceStatusColors(status) {
    const colors = {
        draft: { bg: 'bg-gray-700', text: 'text-gray-300', border: 'border-gray-600' },
        submitted: { bg: 'bg-blue-900', text: 'text-blue-200', border: 'border-blue-700' },
        reported: { bg: 'bg-green-900', text: 'text-green-200', border: 'border-green-700' },
        under_review: { bg: 'bg-yellow-900', text: 'text-yellow-200', border: 'border-yellow-700' },
        archived: { bg: 'bg-purple-900', text: 'text-purple-200', border: 'border-purple-700' },
        rejected: { bg: 'bg-red-700', text: 'text-red-200', border: 'border-red-600' }
    };
    
    const key = status.toLowerCase().replace(/\s+/g, '_');
    return colors[key] || colors.draft;
}

/**
 * Generate evidence card HTML
 * @param {Object} evidence - Evidence data
 * @returns {string} HTML string for evidence card
 */
function generateEvidenceCardHTML(evidence) {
    const emergencyColors = getEmergencyLevelColors(evidence.emergency_level);
    const statusColors = getEvidenceStatusColors(evidence.status);
    
    return `
        <div class="bg-gray-800 rounded-lg border border-gray-700 overflow-hidden hover:shadow-lg transition-shadow">
            <div class="h-48 bg-gradient-to-br from-gray-900 to-gray-800 flex items-center justify-center relative">
                <div class="absolute top-3 left-3">
                    <span class="px-2 py-1 text-xs rounded font-semibold ${statusColors.bg} ${statusColors.text}">
                        ${evidence.status}
                    </span>
                </div>
                <div class="absolute top-3 right-3">
                    <span class="px-2 py-1 text-xs rounded font-semibold ${emergencyColors.bg} ${emergencyColors.text}">
                        ${evidence.emergency_level}
                    </span>
                </div>
                <div class="text-center">
                    <i class="fas fa-file-alt text-4xl text-gray-600 mb-3"></i>
                    <h3 class="font-bold text-lg px-4">${truncateText(evidence.title, 50)}</h3>
                </div>
            </div>
            <div class="p-4">
                <div class="flex justify-between text-sm text-gray-400 mb-3">
                    <span class="flex items-center">
                        <i class="fas fa-map-marker-alt mr-1"></i>
                        ${evidence.county}
                    </span>
                    <span class="flex items-center">
                        <i class="fas fa-clock mr-1"></i>
                        ${formatDate(evidence.incident_time, false)}
                    </span>
                </div>
                <div class="text-sm text-gray-300 mb-4 line-clamp-2">
                    ${evidence.description || 'No description'}
                </div>
                <div class="flex justify-between items-center">
                    <span class="text-xs text-gray-500">
                        #${evidence.evidence_number}
                    </span>
                    <a href="/evidence/view/${evidence.id}" 
                       class="px-3 py-1 bg-red-600 hover:bg-red-700 rounded text-sm transition-colors">
                        <i class="fas fa-eye mr-1"></i>View
                    </a>
                </div>
            </div>
        </div>
    `;
}

// ==================== EXPORTS ====================

// Make functions available globally
window.FLUGUtils = {
    // General utilities
    formatFileSize,
    formatDate,
    truncateText,
    debounce,
    throttle,
    
    // DOM utilities
    showLoading,
    hideLoading,
    showToast,
    hideToast,
    copyToClipboard,
    downloadFile,
    
    // Form utilities
    validateEmail,
    validatePhone,
    validateKenyanID,
    serializeForm,
    clearFormErrors,
    showFieldError,
    
    // API utilities
    apiRequest,
    uploadFile,
    
    // Security utilities
    sanitizeHTML,
    generateRandomString,
    
    // UI utilities
    toggleVisibility,
    animateElement,
    createModal,
    
    // Evidence utilities
    getEmergencyLevelColors,
    getEvidenceStatusColors,
    generateEvidenceCardHTML
};

// Auto-initialize utilities when DOM is loaded
document.addEventListener('DOMContentLoaded', function() {
    console.log('FLUG Utilities loaded');
    
    // Add global error handler
    window.addEventListener('error', function(e) {
        console.error('Global error:', e.error);
        showToast('An unexpected error occurred', 'error');
    });
    
    // Add unhandled promise rejection handler
    window.addEventListener('unhandledrejection', function(e) {
        console.error('Unhandled promise rejection:', e.reason);
        showToast('An unexpected error occurred', 'error');
    });
});