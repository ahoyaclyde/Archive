// Media Routes JavaScript - Handles media-related functionality

document.addEventListener('DOMContentLoaded', function() {
    console.log('🎬 Media routes functionality loaded');

    // Initialize all media-related functionality
    initializeMediaUploads();
    initializeMediaViewers();
    initializeMediaDownloads();
    initializeMediaSharing();
    initializeMediaDeletion();

    // ==================== MEDIA UPLOAD ====================

    function initializeMediaUploads() {
        const uploadForms = document.querySelectorAll('form[enctype="multipart/form-data"]');
        
        uploadForms.forEach(form => {
            // File input change handler
            const fileInputs = form.querySelectorAll('input[type="file"]');
            fileInputs.forEach(input => {
                input.addEventListener('change', handleFileSelection);
            });
            
            // Form submission handler
            form.addEventListener('submit', handleMediaUpload);
        });
        
        // Drag and drop areas
        const dropZones = document.querySelectorAll('.upload-area');
        dropZones.forEach(zone => {
            zone.addEventListener('dragover', handleDragOver);
            zone.addEventListener('dragleave', handleDragLeave);
            zone.addEventListener('drop', handleDrop);
        });
    }

    function handleFileSelection(e) {
        const files = Array.from(e.target.files);
        const container = e.target.closest('.mb-6')?.querySelector('#fileList');
        
        if (container) {
            updateFileList(container, files);
        }
    }

    function updateFileList(container, files) {
        container.innerHTML = '';
        
        if (files.length === 0) {
            container.innerHTML = `
                <div class="text-center py-4 text-gray-500">
                    <i class="fas fa-folder-open text-2xl mb-2"></i>
                    <p>No files selected</p>
                </div>
            `;
            return;
        }
        
        files.forEach((file, index) => {
            const fileElement = createFileElement(file, index);
            container.appendChild(fileElement);
        });
    }

    function createFileElement(file, index) {
        const element = document.createElement('div');
        element.className = 'flex items-center justify-between bg-gray-900 rounded-lg p-3 mb-2';
        element.innerHTML = `
            <div class="flex items-center space-x-3">
                <div class="w-10 h-10 bg-gray-800 rounded-lg flex items-center justify-center">
                    <i class="fas ${getFileIcon(file.name)} text-lg ${getFileIconColor(file.name)}"></i>
                </div>
                <div>
                    <div class="font-medium truncate max-w-xs" title="${file.name}">${file.name}</div>
                    <div class="text-sm text-gray-400">${formatFileSize(file.size)}</div>
                </div>
            </div>
            <div class="flex items-center space-x-2">
                <div class="text-xs text-gray-500">
                    ${Math.round(file.size / 1024)} KB
                </div>
                <button type="button" 
                        class="text-red-400 hover:text-red-300 remove-file"
                        data-index="${index}">
                    <i class="fas fa-times"></i>
                </button>
            </div>
        `;
        
        return element;
    }

    function getFileIcon(filename) {
        const ext = filename.split('.').pop().toLowerCase();
        const iconMap = {
            // Images
            'jpg': 'fa-file-image', 'jpeg': 'fa-file-image', 'png': 'fa-file-image',
            'gif': 'fa-file-image', 'bmp': 'fa-file-image', 'webp': 'fa-file-image',
            'svg': 'fa-file-image',
            
            // Videos
            'mp4': 'fa-file-video', 'mov': 'fa-file-video', 'avi': 'fa-file-video',
            'mkv': 'fa-file-video', 'webm': 'fa-file-video', 'flv': 'fa-file-video',
            'wmv': 'fa-file-video',
            
            // Audio
            'mp3': 'fa-file-audio', 'wav': 'fa-file-audio', 'ogg': 'fa-file-audio',
            'm4a': 'fa-file-audio', 'flac': 'fa-file-audio',
            
            // Documents
            'pdf': 'fa-file-pdf',
            'doc': 'fa-file-word', 'docx': 'fa-file-word',
            'xls': 'fa-file-excel', 'xlsx': 'fa-file-excel',
            'ppt': 'fa-file-powerpoint', 'pptx': 'fa-file-powerpoint',
            'txt': 'fa-file-alt', 'rtf': 'fa-file-alt',
            
            // Archives
            'zip': 'fa-file-archive', 'rar': 'fa-file-archive', 'tar': 'fa-file-archive',
            'gz': 'fa-file-archive', '7z': 'fa-file-archive'
        };
        
        return iconMap[ext] || 'fa-file';
    }

    function getFileIconColor(filename) {
        const ext = filename.split('.').pop().toLowerCase();
        const colorMap = {
            'jpg': 'text-green-400', 'jpeg': 'text-green-400', 'png': 'text-green-400',
            'gif': 'text-green-400', 'bmp': 'text-green-400', 'webp': 'text-green-400',
            'mp4': 'text-red-400', 'mov': 'text-red-400', 'avi': 'text-red-400',
            'mkv': 'text-red-400', 'webm': 'text-red-400',
            'mp3': 'text-purple-400', 'wav': 'text-purple-400', 'ogg': 'text-purple-400',
            'pdf': 'text-red-400',
            'doc': 'text-blue-400', 'docx': 'text-blue-400',
            'xls': 'text-green-400', 'xlsx': 'text-green-400',
            'zip': 'text-yellow-400', 'rar': 'text-yellow-400'
        };
        
        return colorMap[ext] || 'text-gray-400';
    }

    function handleDragOver(e) {
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.classList.add('dragover');
    }

    function handleDragLeave(e) {
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.classList.remove('dragover');
    }

    function handleDrop(e) {
        e.preventDefault();
        e.stopPropagation();
        e.currentTarget.classList.remove('dragover');
        
        const files = Array.from(e.dataTransfer.files);
        const fileInput = e.currentTarget.querySelector('input[type="file"]');
        const container = e.currentTarget.closest('.mb-6')?.querySelector('#fileList');
        
        if (container) {
            updateFileList(container, files);
        }
        
        // Update file input files
        if (fileInput) {
            const dataTransfer = new DataTransfer();
            files.forEach(file => dataTransfer.items.add(file));
            fileInput.files = dataTransfer.files;
        }
    }

    async function handleMediaUpload(e) {
        e.preventDefault();
        
        const form = e.target;
        const submitButton = form.querySelector('button[type="submit"]');
        const progressContainer = form.querySelector('#uploadProgress');
        const progressBar = form.querySelector('#progressBar');
        const progressText = form.querySelector('#uploadPercentage');
        const statusText = form.querySelector('#uploadStatus');
        
        // Validate form
        if (!validateUploadForm(form)) {
            return;
        }
        
        // Show progress
        if (progressContainer) progressContainer.classList.remove('hidden');
        if (submitButton) submitButton.disabled = true;
        
        try {
            const formData = new FormData(form);
            
            // Show initial progress
            if (progressBar) progressBar.style.width = '5%';
            if (progressText) progressText.textContent = '5%';
            if (statusText) statusText.textContent = 'Preparing upload...';
            
            // Upload with progress tracking
            const response = await uploadWithProgress(form.action, formData, (progress) => {
                if (progressBar) progressBar.style.width = `${progress}%`;
                if (progressText) progressText.textContent = `${Math.round(progress)}%`;
                if (statusText) statusText.textContent = `Uploading... ${Math.round(progress)}%`;
            });
            
            // Complete progress
            if (progressBar) progressBar.style.width = '100%';
            if (progressText) progressText.textContent = '100%';
            if (statusText) statusText.textContent = 'Upload complete!';
            
            // Handle response
            if (response.success) {
                showToast('Media uploaded successfully!', 'success');
                
                // Redirect if specified
                if (response.data?.redirect) {
                    setTimeout(() => {
                        window.location.href = response.data.redirect;
                    }, 1500);
                }
            } else {
                throw new Error(response.message || 'Upload failed');
            }
        } catch (error) {
            console.error('Upload error:', error);
            showToast(`Upload failed: ${error.message}`, 'error');
            
            // Reset UI
            if (progressContainer) progressContainer.classList.add('hidden');
            if (submitButton) submitButton.disabled = false;
        }
    }

    function validateUploadForm(form) {
        // Check for required fields
        const requiredFields = form.querySelectorAll('[required]');
        let isValid = true;
        
        for (const field of requiredFields) {
            if (!field.value.trim()) {
                field.classList.add('border-red-500');
                isValid = false;
            } else {
                field.classList.remove('border-red-500');
            }
        }
        
        // Check file size limits
        const fileInputs = form.querySelectorAll('input[type="file"]');
        for (const input of fileInputs) {
            if (input.files.length > 0) {
                for (const file of input.files) {
                    if (file.size > 100 * 1024 * 1024) { // 100MB limit
                        showToast(`File "${file.name}" exceeds 100MB limit`, 'error');
                        isValid = false;
                        break;
                    }
                }
            }
        }
        
        return isValid;
    }

    async function uploadWithProgress(url, formData, onProgress) {
        return new Promise((resolve, reject) => {
            const xhr = new XMLHttpRequest();
            
            xhr.upload.addEventListener('progress', (event) => {
                if (event.lengthComputable) {
                    const progress = (event.loaded / event.total) * 100;
                    onProgress(progress);
                }
            });
            
            xhr.addEventListener('load', () => {
                if (xhr.status >= 200 && xhr.status < 300) {
                    try {
                        const response = JSON.parse(xhr.responseText);
                        resolve(response);
                    } catch (e) {
                        resolve({ success: true, data: xhr.responseText });
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

    // ==================== MEDIA VIEWER ====================

    function initializeMediaViewers() {
        // Image viewers
        const images = document.querySelectorAll('img[onclick*="openMediaModal"]');
        images.forEach(img => {
            img.addEventListener('click', function() {
                const url = this.src;
                const type = 'image';
                openMediaViewer(url, type);
            });
        });
        
        // Video viewers
        const videos = document.querySelectorAll('video[src]');
        videos.forEach(video => {
            video.addEventListener('click', function() {
                if (this.paused) {
                    this.play();
                } else {
                    this.pause();
                }
            });
        });
        
        // Audio players
        const audios = document.querySelectorAll('audio[src]');
        audios.forEach(audio => {
            // Add custom controls if needed
            if (!audio.controls) {
                audio.controls = true;
            }
        });
    }

    function openMediaViewer(url, type) {
        const modal = document.createElement('div');
        modal.className = 'fixed inset-0 bg-black/90 flex items-center justify-center z-50';
        modal.innerHTML = `
            <div class="relative w-full h-full flex items-center justify-center">
                <button class="absolute top-4 right-4 text-white text-3xl close-viewer z-10">
                    <i class="fas fa-times"></i>
                </button>
                <button class="absolute top-1/2 left-4 text-white text-3xl prev-media z-10 transform -translate-y-1/2">
                    <i class="fas fa-chevron-left"></i>
                </button>
                <button class="absolute top-1/2 right-4 text-white text-3xl next-media z-10 transform -translate-y-1/2">
                    <i class="fas fa-chevron-right"></i>
                </button>
                <div class="media-content max-w-full max-h-full p-4">
                    ${type === 'image' ? 
                        `<img src="${url}" class="max-w-full max-h-full object-contain" alt="Media">` :
                        type === 'video' ?
                        `<video src="${url}" controls autoplay class="max-w-full max-h-full"></video>` :
                        `<div class="text-white">Unsupported media type</div>`
                    }
                </div>
                <div class="absolute bottom-4 left-1/2 transform -translate-x-1/2 text-white text-sm">
                    ${type === 'image' ? 'Image' : type === 'video' ? 'Video' : 'Media'}
                </div>
            </div>
        `;
        
        document.body.appendChild(modal);
        
        // Close button
        modal.querySelector('.close-viewer').addEventListener('click', () => {
            modal.remove();
        });
        
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
        
        // Navigation (if there are multiple media items)
        setupMediaNavigation(modal, url, type);
    }

    function setupMediaNavigation(modal, currentUrl, currentType) {
        // Get all media items on the page
        const mediaItems = Array.from(document.querySelectorAll('img[src], video[src], audio[src]'))
            .filter(item => {
                const url = item.src || item.querySelector('source')?.src;
                return url && url.startsWith('http');
            })
            .map(item => {
                const url = item.src || item.querySelector('source')?.src;
                const type = item.tagName.toLowerCase() === 'img' ? 'image' :
                            item.tagName.toLowerCase() === 'video' ? 'video' : 'audio';
                return { url, type, element: item };
            });
        
        if (mediaItems.length <= 1) {
            // Hide navigation buttons if only one item
            modal.querySelector('.prev-media').style.display = 'none';
            modal.querySelector('.next-media').style.display = 'none';
            return;
        }
        
        const currentIndex = mediaItems.findIndex(item => item.url === currentUrl);
        
        // Previous button
        modal.querySelector('.prev-media').addEventListener('click', () => {
            const prevIndex = (currentIndex - 1 + mediaItems.length) % mediaItems.length;
            const prevItem = mediaItems[prevIndex];
            updateMediaViewer(modal, prevItem.url, prevItem.type);
        });
        
        // Next button
        modal.querySelector('.next-media').addEventListener('click', () => {
            const nextIndex = (currentIndex + 1) % mediaItems.length;
            const nextItem = mediaItems[nextIndex];
            updateMediaViewer(modal, nextItem.url, nextItem.type);
        });
        
        // Keyboard navigation
        const handleKeyNavigation = (e) => {
            if (e.key === 'ArrowLeft') {
                const prevIndex = (currentIndex - 1 + mediaItems.length) % mediaItems.length;
                const prevItem = mediaItems[prevIndex];
                updateMediaViewer(modal, prevItem.url, prevItem.type);
            } else if (e.key === 'ArrowRight') {
                const nextIndex = (currentIndex + 1) % mediaItems.length;
                const nextItem = mediaItems[nextIndex];
                updateMediaViewer(modal, nextItem.url, nextItem.type);
            }
        };
        
        document.addEventListener('keydown', handleKeyNavigation);
        
        // Clean up event listener when modal closes
        modal.addEventListener('remove', () => {
            document.removeEventListener('keydown', handleKeyNavigation);
        });
    }

    function updateMediaViewer(modal, url, type) {
        const content = modal.querySelector('.media-content');
        content.innerHTML = type === 'image' ? 
            `<img src="${url}" class="max-w-full max-h-full object-contain" alt="Media">` :
            type === 'video' ?
            `<video src="${url}" controls autoplay class="max-w-full max-h-full"></video>` :
            `<div class="text-white">Unsupported media type</div>`;
    }

    // ==================== MEDIA DOWNLOAD ====================

    function initializeMediaDownloads() {
        // Download buttons
        const downloadButtons = document.querySelectorAll('button[onclick*="downloadFile"]');
        downloadButtons.forEach(button => {
            button.addEventListener('click', function() {
                const onclick = this.getAttribute('onclick');
                const match = onclick.match(/downloadFile\('([^']+)',\s*'([^']+)'\)/);
                if (match) {
                    const [_, url, filename] = match;
                    downloadMedia(url, filename);
                }
            });
        });
        
        // Context menu for media items
        document.addEventListener('contextmenu', (e) => {
            const mediaElement = e.target.closest('img, video, audio');
            if (mediaElement && mediaElement.src) {
                e.preventDefault();
                showMediaContextMenu(e, mediaElement);
            }
        });
    }

    function downloadMedia(url, filename) {
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        link.target = '_blank';
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
        
        showToast(`Downloading ${filename}...`, 'info');
    }

    function showMediaContextMenu(e, mediaElement) {
        // Remove existing context menu
        const existingMenu = document.querySelector('.media-context-menu');
        if (existingMenu) {
            existingMenu.remove();
        }
        
        const url = mediaElement.src || mediaElement.querySelector('source')?.src;
        const filename = url.split('/').pop() || 'media';
        const type = mediaElement.tagName.toLowerCase();
        
        const menu = document.createElement('div');
        menu.className = 'media-context-menu fixed bg-gray-800 border border-gray-700 rounded-lg shadow-xl z-50 py-2 min-w-48';
        menu.style.left = `${e.pageX}px`;
        menu.style.top = `${e.pageY}px`;
        
        menu.innerHTML = `
            <div class="px-4 py-2 text-sm text-gray-300 border-b border-gray-700">
                Media Actions
            </div>
            <button class="context-action block w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-gray-700" data-action="view">
                <i class="fas fa-eye mr-2"></i>View Fullscreen
            </button>
            <button class="context-action block w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-gray-700" data-action="download">
                <i class="fas fa-download mr-2"></i>Download
            </button>
            <button class="context-action block w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-gray-700" data-action="copy">
                <i class="fas fa-copy mr-2"></i>Copy URL
            </button>
            <button class="context-action block w-full text-left px-4 py-2 text-sm text-blue-300 hover:bg-gray-700" data-action="share">
                <i class="fas fa-share mr-2"></i>Share
            </button>
            <div class="px-4 py-2 text-xs text-gray-500 border-t border-gray-700 mt-2">
                Right-click anywhere to close
            </div>
        `;
        
        document.body.appendChild(menu);
        
        // Handle actions
        menu.querySelectorAll('.context-action').forEach(button => {
            button.addEventListener('click', function() {
                const action = this.dataset.action;
                handleMediaAction(action, url, filename, type, mediaElement);
                menu.remove();
            });
        });
        
        // Close menu when clicking elsewhere
        const closeMenu = function(e) {
            if (!menu.contains(e.target)) {
                menu.remove();
                document.removeEventListener('click', closeMenu);
                document.removeEventListener('contextmenu', closeMenu);
            }
        };
        
        setTimeout(() => {
            document.addEventListener('click', closeMenu);
            document.addEventListener('contextmenu', closeMenu);
        }, 100);
    }

    function handleMediaAction(action, url, filename, type, element) {
        switch(action) {
            case 'view':
                openMediaViewer(url, type === 'img' ? 'image' : type === 'video' ? 'video' : 'audio');
                break;
                
            case 'download':
                downloadMedia(url, filename);
                break;
                
            case 'copy':
                navigator.clipboard.writeText(url)
                    .then(() => showToast('URL copied to clipboard', 'success'))
                    .catch(() => showToast('Failed to copy URL', 'error'));
                break;
                
            case 'share':
                if (navigator.share) {
                    navigator.share({
                        title: filename,
                        text: 'Check out this media',
                        url: url
                    }).catch(() => {
                        // Fallback to copy URL
                        navigator.clipboard.writeText(url)
                            .then(() => showToast('URL copied to clipboard', 'success'))
                            .catch(() => showToast('Failed to share', 'error'));
                    });
                } else {
                    navigator.clipboard.writeText(url)
                        .then(() => showToast('URL copied to clipboard', 'success'))
                        .catch(() => showToast('Failed to copy URL', 'error'));
                }
                break;
        }
    }

    // ==================== MEDIA SHARING ====================

    function initializeMediaSharing() {
        // Share buttons
        const shareButtons = document.querySelectorAll('.share-media');
        shareButtons.forEach(button => {
            button.addEventListener('click', handleMediaShare);
        });
    }

    function handleMediaShare(e) {
        const button = e.currentTarget;
        const mediaContainer = button.closest('.bg-gray-800.rounded-lg');
        
        if (!mediaContainer) return;
        
        const mediaElement = mediaContainer.querySelector('img, video, audio');
        if (!mediaElement) return;
        
        const url = mediaElement.src || mediaElement.querySelector('source')?.src;
        const filename = url.split('/').pop() || 'media';
        
        if (navigator.share) {
            navigator.share({
                title: filename,
                text: 'Check out this evidence media',
                url: url
            }).catch(() => {
                // Fallback to copy URL
                navigator.clipboard.writeText(url)
                    .then(() => showToast('URL copied to clipboard', 'success'))
                    .catch(() => showToast('Failed to share', 'error'));
            });
        } else {
            navigator.clipboard.writeText(url)
                .then(() => showToast('URL copied to clipboard', 'success'))
                .catch(() => showToast('Failed to copy URL', 'error'));
        }
    }

    // ==================== MEDIA DELETION ====================

    function initializeMediaDeletion() {
        // Delete buttons
        const deleteButtons = document.querySelectorAll('.delete-media');
        deleteButtons.forEach(button => {
            button.addEventListener('click', handleMediaDelete);
        });
    }

    function handleMediaDelete(e) {
        const button = e.currentTarget;
        const mediaContainer = button.closest('.bg-gray-800.rounded-lg');
        
        if (!mediaContainer) return;
        
        const mediaId = mediaContainer.dataset.mediaId;
        const evidenceId = mediaContainer.dataset.evidenceId;
        
        if (!mediaId || !evidenceId) {
            console.error('Missing media ID or evidence ID');
            return;
        }
        
        if (!confirm('Are you sure you want to delete this media file? This action cannot be undone.')) {
            return;
        }
        
        // Show loading
        button.disabled = true;
        button.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
        
        // Send delete request
        fetch(`/api/evidence/${evidenceId}/media/${mediaId}`, {
            method: 'DELETE'
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                // Remove media container with animation
                mediaContainer.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
                mediaContainer.style.opacity = '0';
                mediaContainer.style.transform = 'scale(0.8)';
                
                setTimeout(() => {
                    mediaContainer.remove();
                    showToast('Media deleted successfully', 'success');
                }, 300);
            } else {
                throw new Error(data.message || 'Delete failed');
            }
        })
        .catch(error => {
            console.error('Delete error:', error);
            showToast(`Failed to delete media: ${error.message}`, 'error');
            button.disabled = false;
            button.innerHTML = '<i class="fas fa-trash"></i>';
        });
    }

    // ==================== HELPER FUNCTIONS ====================

    function formatFileSize(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    function showToast(message, type = 'info') {
        // Use FLUGUtils if available, otherwise create simple toast
        if (window.FLUGUtils && window.FLUGUtils.showToast) {
            window.FLUGUtils.showToast(message, type);
        } else {
            const toast = document.createElement('div');
            toast.className = `fixed top-4 right-4 z-50 bg-${type === 'success' ? 'green' : type === 'error' ? 'red' : type === 'warning' ? 'yellow' : 'blue'}-900 border border-${type === 'success' ? 'green' : type === 'error' ? 'red' : type === 'warning' ? 'yellow' : 'blue'}-700 rounded-lg p-4 max-w-sm shadow-xl`;
            toast.innerHTML = `
                <div class="flex items-center">
                    <i class="fas ${type === 'success' ? 'fa-check-circle' : type === 'error' ? 'fa-exclamation-circle' : type === 'warning' ? 'fa-exclamation-triangle' : 'fa-info-circle'} text-${type === 'success' ? 'green' : type === 'error' ? 'red' : type === 'warning' ? 'yellow' : 'blue'}-400 mr-2"></i>
                    <span class="text-white">${message}</span>
                </div>
            `;
            document.body.appendChild(toast);
            
            setTimeout(() => {
                if (toast.parentElement) {
                    toast.parentElement.removeChild(toast);
                }
            }, 3000);
        }
    }

    // ==================== EVENT DELEGATION ====================

    // Handle dynamic elements with event delegation
    document.addEventListener('click', function(e) {
        // Remove file buttons
        if (e.target.closest('.remove-file')) {
            const button = e.target.closest('.remove-file');
            const index = button.dataset.index;
            const container = button.closest('#fileList');
            const fileInput = container?.closest('form')?.querySelector('input[type="file"]');
            
            if (container && fileInput) {
                removeFileFromList(container, index, fileInput);
            }
        }
    });

    function removeFileFromList(container, index, fileInput) {
        // Remove from DOM
        const fileElement = container.querySelector(`[data-index="${index}"]`)?.closest('.flex.items-center');
        if (fileElement) {
            fileElement.remove();
        }
        
        // Update file input
        const files = Array.from(fileInput.files);
        files.splice(index, 1);
        
        const dataTransfer = new DataTransfer();
        files.forEach(file => dataTransfer.items.add(file));
        fileInput.files = dataTransfer.files;
        
        // Update indices
        const remainingElements = container.querySelectorAll('.remove-file');
        remainingElements.forEach((element, newIndex) => {
            element.dataset.index = newIndex;
        });
        
        // Show empty state if no files
        if (files.length === 0) {
            container.innerHTML = `
                <div class="text-center py-4 text-gray-500">
                    <i class="fas fa-folder-open text-2xl mb-2"></i>
                    <p>No files selected</p>
                </div>
            `;
        }
    }

    // ==================== INITIALIZATION ====================

    console.log('Media routes initialized successfully');
});