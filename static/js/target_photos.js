// Target Photos JavaScript
document.addEventListener('DOMContentLoaded', function() {
    console.log('🎯 Target photos functionality loaded');

    // DOM Elements
    const targetGrid = document.querySelector('.grid.grid-cols-2');
    const targetModal = document.getElementById('targetModal');
    const mediaModal = document.getElementById('mediaModal');
    const evidenceId = document.querySelector('input[name="evidence_id"]')?.value;

    // Initialize
    if (targetGrid) {
        initializeTargetGrid();
    }
    
    if (targetModal) {
        initializeTargetModal();
    }

    // Initialize target grid
    function initializeTargetGrid() {
        const targetItems = targetGrid.querySelectorAll('.bg-gray-800.rounded-lg');
        
        targetItems.forEach((item, index) => {
            // Add hover effects
            item.addEventListener('mouseenter', function() {
                this.style.transform = 'scale(1.05)';
                this.style.transition = 'transform 0.3s ease';
                this.style.zIndex = '10';
            });
            
            item.addEventListener('mouseleave', function() {
                this.style.transform = 'scale(1)';
                this.style.zIndex = '1';
            });
            
            // Click to open modal
            const image = item.querySelector('img');
            if (image) {
                image.addEventListener('click', function() {
                    openTargetModal(index);
                });
            }
            
            // Add context menu
            addTargetContextMenu(item, index);
        });
        
        // Initialize drag and drop for reordering
        if (targetItems.length > 1) {
            initializeDragAndDrop();
        }
    }

    // Initialize target modal
    function initializeTargetModal() {
        // Navigation buttons
        const prevButton = targetModal.querySelector('.fa-chevron-left')?.closest('button');
        const nextButton = targetModal.querySelector('.fa-chevron-right')?.closest('button');
        
        if (prevButton) {
            prevButton.addEventListener('click', function(e) {
                e.stopPropagation();
                navigateTarget('prev');
            });
        }
        
        if (nextButton) {
            nextButton.addEventListener('click', function(e) {
                e.stopPropagation();
                navigateTarget('next');
            });
        }
        
        // Close modal on escape
        document.addEventListener('keydown', function(e) {
            if (e.key === 'Escape' && !targetModal.classList.contains('hidden')) {
                closeTargetModal();
            }
            
            // Arrow key navigation
            if (!targetModal.classList.contains('hidden')) {
                if (e.key === 'ArrowLeft') {
                    navigateTarget('prev');
                } else if (e.key === 'ArrowRight') {
                    navigateTarget('next');
                }
            }
        });
        
        // Close on background click
        targetModal.addEventListener('click', function(e) {
            if (e.target === this) {
                closeTargetModal();
            }
        });
    }

    // Open target modal
    window.openTargetModal = function(index) {
        const targetItems = targetGrid?.querySelectorAll('.bg-gray-800.rounded-lg');
        if (!targetItems || index >= targetItems.length) return;
        
        const item = targetItems[index];
        const image = item.querySelector('img');
        const description = item.querySelector('.font-medium.text-sm')?.textContent;
        const category = item.querySelector('.text-xs.text-gray-400 span')?.textContent;
        const confidenceText = item.querySelectorAll('.text-xs.text-gray-400 span')[1]?.textContent;
        
        if (image && targetModal) {
            // Update modal content
            document.getElementById('targetImage').src = image.src;
            document.getElementById('targetTitle').textContent = `Target #${index + 1}`;
            document.getElementById('targetDescription').textContent = description || 'No description';
            document.getElementById('targetCategory').textContent = category || 'Unknown';
            
            // Parse confidence score
            const confidenceMatch = confidenceText?.match(/(\d+)%/);
            const confidence = confidenceMatch ? parseInt(confidenceMatch[1]) : 0;
            document.getElementById('targetConfidenceBar').style.width = `${confidence}%`;
            document.getElementById('targetConfidence').textContent = `${confidence}%`;
            
            // Set current index
            targetModal.dataset.currentIndex = index;
            
            // Show modal
            targetModal.classList.remove('hidden');
            document.body.style.overflow = 'hidden';
            
            // Update navigation buttons state
            updateNavigationButtons(index);
        }
    };

    // Close target modal
    window.closeTargetModal = function() {
        if (targetModal) {
            targetModal.classList.add('hidden');
            document.body.style.overflow = 'auto';
        }
    };

    // Navigate between targets
    function navigateTarget(direction) {
        const currentIndex = parseInt(targetModal.dataset.currentIndex || '0');
        const targetItems = targetGrid?.querySelectorAll('.bg-gray-800.rounded-lg');
        
        if (!targetItems) return;
        
        let newIndex;
        if (direction === 'next') {
            newIndex = (currentIndex + 1) % targetItems.length;
        } else {
            newIndex = (currentIndex - 1 + targetItems.length) % targetItems.length;
        }
        
        openTargetModal(newIndex);
    }

    // Update navigation buttons state
    function updateNavigationButtons(currentIndex) {
        const targetItems = targetGrid?.querySelectorAll('.bg-gray-800.rounded-lg');
        if (!targetItems) return;
        
        const prevButton = targetModal.querySelector('.fa-chevron-left')?.closest('button');
        const nextButton = targetModal.querySelector('.fa-chevron-right')?.closest('button');
        
        if (prevButton) {
            prevButton.disabled = targetItems.length <= 1;
        }
        
        if (nextButton) {
            nextButton.disabled = targetItems.length <= 1;
        }
        
        // Update counter if exists
        const counter = targetModal.querySelector('.target-counter');
        if (!counter) {
            const counterElement = document.createElement('div');
            counterElement.className = 'target-counter text-sm text-gray-400 absolute top-4 left-1/2 transform -translate-x-1/2';
            counterElement.textContent = `${currentIndex + 1} / ${targetItems.length}`;
            targetModal.querySelector('.modal-content').appendChild(counterElement);
        } else {
            counter.textContent = `${currentIndex + 1} / ${targetItems.length}`;
        }
    }

    // Add context menu to target item
    function addTargetContextMenu(item, index) {
        item.addEventListener('contextmenu', function(e) {
            e.preventDefault();
            
            // Remove existing context menu
            const existingMenu = document.querySelector('.target-context-menu');
            if (existingMenu) {
                existingMenu.remove();
            }
            
            // Create context menu
            const menu = document.createElement('div');
            menu.className = 'target-context-menu fixed bg-gray-800 border border-gray-700 rounded-lg shadow-xl z-50 py-2 min-w-48';
            menu.style.left = `${e.pageX}px`;
            menu.style.top = `${e.pageY}px`;
            
            menu.innerHTML = `
                <div class="px-4 py-2 text-sm text-gray-300 border-b border-gray-700">
                    Target Actions
                </div>
                <button class="context-menu-item block w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-gray-700" data-action="view">
                    <i class="fas fa-eye mr-2"></i>View Details
                </button>
                <button class="context-menu-item block w-full text-left px-4 py-2 text-sm text-gray-300 hover:bg-gray-700" data-action="edit">
                    <i class="fas fa-edit mr-2"></i>Edit Description
                </button>
                <button class="context-menu-item block w-full text-left px-4 py-2 text-sm text-red-300 hover:bg-gray-700" data-action="delete">
                    <i class="fas fa-trash mr-2"></i>Delete Target
                </button>
                <button class="context-menu-item block w-full text-left px-4 py-2 text-sm text-blue-300 hover:bg-gray-700" data-action="download">
                    <i class="fas fa-download mr-2"></i>Download Image
                </button>
                <div class="px-4 py-2 text-xs text-gray-500 border-t border-gray-700 mt-2">
                    Right-click anywhere to close
                </div>
            `;
            
            document.body.appendChild(menu);
            
            // Handle menu actions
            menu.querySelectorAll('.context-menu-item').forEach(button => {
                button.addEventListener('click', function() {
                    const action = this.dataset.action;
                    handleTargetAction(action, index, item);
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
        });
    }

    // Handle target context menu actions
    function handleTargetAction(action, index, item) {
        const image = item.querySelector('img');
        
        switch(action) {
            case 'view':
                openTargetModal(index);
                break;
                
            case 'edit':
                editTargetDescription(index, item);
                break;
                
            case 'delete':
                deleteTarget(index, item);
                break;
                
            case 'download':
                if (image) {
                    downloadImage(image.src, `target-${index + 1}.jpg`);
                }
                break;
        }
    }

    // Edit target description
    function editTargetDescription(index, item) {
        const currentDescription = item.querySelector('.font-medium.text-sm')?.textContent || '';
        
        const modal = document.createElement('div');
        modal.className = 'fixed inset-0 bg-black/70 flex items-center justify-center z-50';
        modal.innerHTML = `
            <div class="bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4">
                <h3 class="text-xl font-bold mb-4">Edit Target Description</h3>
                <div class="mb-4">
                    <label class="block text-sm text-gray-300 mb-2">Description</label>
                    <textarea id="editDescription" class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-3 h-32">${currentDescription}</textarea>
                </div>
                <div class="mb-4">
                    <label class="block text-sm text-gray-300 mb-2">Category</label>
                    <select id="editCategory" class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-3">
                        <option value="person">Person</option>
                        <option value="vehicle">Vehicle</option>
                        <option value="object">Object</option>
                        <option value="location">Location</option>
                        <option value="other">Other</option>
                    </select>
                </div>
                <div class="flex justify-end space-x-3">
                    <button id="cancelEdit" class="px-4 py-2 bg-gray-700 hover:bg-gray-600 rounded-lg">Cancel</button>
                    <button id="saveEdit" class="px-4 py-2 bg-red-600 hover:bg-red-700 rounded-lg">Save Changes</button>
                </div>
            </div>
        `;
        
        document.body.appendChild(modal);
        
        // Set current category
        const currentCategory = item.querySelector('.text-xs.text-gray-400 span')?.textContent?.toLowerCase();
        if (currentCategory) {
            const categorySelect = modal.querySelector('#editCategory');
            categorySelect.value = currentCategory;
        }
        
        // Handle save
        modal.querySelector('#saveEdit').addEventListener('click', function() {
            const newDescription = modal.querySelector('#editDescription').value;
            const newCategory = modal.querySelector('#editCategory').value;
            
            // Update UI
            item.querySelector('.font-medium.text-sm').textContent = newDescription;
            
            const categoryElement = item.querySelector('.text-xs.text-gray-400 span');
            if (categoryElement) {
                categoryElement.textContent = newCategory.charAt(0).toUpperCase() + newCategory.slice(1);
            }
            
            // Send update to server
            updateTargetOnServer(index, newDescription, newCategory);
            
            modal.remove();
        });
        
        // Handle cancel
        modal.querySelector('#cancelEdit').addEventListener('click', function() {
            modal.remove();
        });
        
        // Close on background click
        modal.addEventListener('click', function(e) {
            if (e.target === this) {
                modal.remove();
            }
        });
    }

    // Update target on server
    function updateTargetOnServer(index, description, category) {
        if (!evidenceId) return;
        
        fetch(`/api/evidence/${evidenceId}/targets/${index}`, {
            method: 'PUT',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({
                description: description,
                category: category
            })
        })
        .then(response => response.json())
        .then(data => {
            if (!data.success) {
                console.error('Failed to update target:', data.message);
                alert('Failed to save changes');
            }
        })
        .catch(error => {
            console.error('Update error:', error);
            alert('Error saving changes');
        });
    }

    // Delete target
    function deleteTarget(index, item) {
        if (!confirm('Are you sure you want to delete this target? This action cannot be undone.')) {
            return;
        }
        
        // Animate removal
        item.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
        item.style.opacity = '0';
        item.style.transform = 'scale(0.8)';
        
        setTimeout(() => {
            item.remove();
            
            // Reindex remaining targets
            reindexTargets();
            
            // Update server
            deleteTargetOnServer(index);
        }, 300);
    }

    // Delete target on server
    function deleteTargetOnServer(index) {
        if (!evidenceId) return;
        
        fetch(`/api/evidence/${evidenceId}/targets/${index}`, {
            method: 'DELETE'
        })
        .then(response => response.json())
        .then(data => {
            if (!data.success) {
                console.error('Failed to delete target:', data.message);
                alert('Failed to delete target');
            }
        })
        .catch(error => {
            console.error('Delete error:', error);
            alert('Error deleting target');
        });
    }

    // Reindex targets after deletion
    function reindexTargets() {
        const targetItems = targetGrid?.querySelectorAll('.bg-gray-800.rounded-lg');
        if (!targetItems) return;
        
        targetItems.forEach((item, newIndex) => {
            // Update target number
            const numberElement = item.querySelector('.absolute.top-2.left-2');
            if (numberElement) {
                numberElement.textContent = `#${newIndex + 1}`;
            }
            
            // Update click handler
            const image = item.querySelector('img');
            if (image) {
                image.onclick = () => openTargetModal(newIndex);
            }
            
            // Update context menu
            item.addEventListener('contextmenu', function(e) {
                e.preventDefault();
                addTargetContextMenu(item, newIndex);
            }, { once: true });
        });
    }

    // Download image
    function downloadImage(url, filename) {
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        document.body.appendChild(link);
        link.click();
        document.body.removeChild(link);
    }

    // Initialize drag and drop for reordering
    function initializeDragAndDrop() {
        const targetItems = targetGrid.querySelectorAll('.bg-gray-800.rounded-lg');
        
        targetItems.forEach(item => {
            item.draggable = true;
            
            item.addEventListener('dragstart', function(e) {
                this.classList.add('dragging');
                this.style.opacity = '0.5';
                e.dataTransfer.setData('text/plain', Array.from(targetItems).indexOf(this));
            });
            
            item.addEventListener('dragend', function() {
                this.classList.remove('dragging');
                this.style.opacity = '1';
                
                // Update server with new order
                updateTargetOrder();
            });
        });
        
        targetGrid.addEventListener('dragover', function(e) {
            e.preventDefault();
            const afterElement = getDragAfterElement(targetGrid, e.clientY);
            const draggable = document.querySelector('.dragging');
            
            if (afterElement == null) {
                targetGrid.appendChild(draggable);
            } else {
                targetGrid.insertBefore(draggable, afterElement);
            }
        });
    }

    // Get element after which to insert dragged element
    function getDragAfterElement(container, y) {
        const draggableElements = [...container.querySelectorAll('.bg-gray-800.rounded-lg:not(.dragging)')];
        
        return draggableElements.reduce((closest, child) => {
            const box = child.getBoundingClientRect();
            const offset = y - box.top - box.height / 2;
            
            if (offset < 0 && offset > closest.offset) {
                return { offset: offset, element: child };
            } else {
                return closest;
            }
        }, { offset: Number.NEGATIVE_INFINITY }).element;
    }

    // Update target order on server
    function updateTargetOrder() {
        if (!evidenceId) return;
        
        const targetItems = targetGrid.querySelectorAll('.bg-gray-800.rounded-lg');
        const targetIds = Array.from(targetItems).map(item => {
            // Extract target ID from data attribute or image URL
            const image = item.querySelector('img');
            if (image && image.src) {
                const match = image.src.match(/targets\/(.+?)\./);
                return match ? match[1] : null;
            }
            return null;
        }).filter(id => id !== null);
        
        fetch(`/api/evidence/${evidenceId}/targets/reorder`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
            },
            body: JSON.stringify({ order: targetIds })
        })
        .then(response => response.json())
        .then(data => {
            if (!data.success) {
                console.error('Failed to update order:', data.message);
            }
        })
        .catch(console.error);
    }

    // Bulk target operations
    initializeBulkTargetOperations();

    function initializeBulkTargetOperations() {
        const targetContainer = document.querySelector('.mb-8');
        if (targetContainer && targetGrid) {
            const bulkControls = document.createElement('div');
            bulkControls.className = 'mb-4 flex items-center justify-between';
            bulkControls.innerHTML = `
                <div class="flex items-center space-x-4">
                    <button id="selectAllTargets" class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">
                        <i class="fas fa-check-square mr-1"></i>Select All
                    </button>
                    <div id="targetBulkActions" class="hidden space-x-2">
                        <button id="bulkDownloadTargets" class="px-3 py-1 bg-blue-600 hover:bg-blue-700 rounded text-sm">
                            <i class="fas fa-download mr-1"></i>Download Selected
                        </button>
                        <button id="bulkDeleteTargets" class="px-3 py-1 bg-red-600 hover:bg-red-700 rounded text-sm">
                            <i class="fas fa-trash mr-1"></i>Delete Selected
                        </button>
                    </div>
                </div>
                <span id="selectedTargetCount" class="text-sm text-gray-400 hidden">0 selected</span>
            `;
            
            targetContainer.insertBefore(bulkControls, targetGrid);
            
            // Add checkboxes to targets
            const targetItems = targetGrid.querySelectorAll('.bg-gray-800.rounded-lg');
            targetItems.forEach(item => {
                const checkbox = document.createElement('input');
                checkbox.type = 'checkbox';
                checkbox.className = 'target-checkbox absolute top-2 right-2 z-10';
                checkbox.style.display = 'none';
                
                item.style.position = 'relative';
                item.appendChild(checkbox);
                
                // Show checkbox on hover
                item.addEventListener('mouseenter', function() {
                    if (!this.classList.contains('dragging')) {
                        checkbox.style.display = 'block';
                    }
                });
                
                item.addEventListener('mouseleave', function() {
                    if (!checkbox.checked) {
                        checkbox.style.display = 'none';
                    }
                });
                
                checkbox.addEventListener('change', function() {
                    if (this.checked) {
                        item.classList.add('ring-2', 'ring-yellow-500');
                    } else {
                        item.classList.remove('ring-2', 'ring-yellow-500');
                    }
                    updateTargetBulkActions();
                });
            });
            
            // Select all targets
            document.getElementById('selectAllTargets').addEventListener('click', function() {
                const checkboxes = document.querySelectorAll('.target-checkbox');
                const allChecked = Array.from(checkboxes).every(cb => cb.checked);
                
                checkboxes.forEach(checkbox => {
                    checkbox.checked = !allChecked;
                    checkbox.style.display = !allChecked ? 'block' : 'none';
                    
                    const item = checkbox.closest('.bg-gray-800.rounded-lg');
                    if (item) {
                        if (!allChecked) {
                            item.classList.add('ring-2', 'ring-yellow-500');
                        } else {
                            item.classList.remove('ring-2', 'ring-yellow-500');
                        }
                    }
                });
                
                updateTargetBulkActions();
            });
            
            // Bulk download
            document.getElementById('bulkDownloadTargets').addEventListener('click', bulkDownloadTargets);
            
            // Bulk delete
            document.getElementById('bulkDeleteTargets').addEventListener('click', bulkDeleteTargets);
        }
    }

    function updateTargetBulkActions() {
        const checkboxes = document.querySelectorAll('.target-checkbox');
        const checkedCount = Array.from(checkboxes).filter(cb => cb.checked).length;
        const bulkActions = document.getElementById('targetBulkActions');
        const countDisplay = document.getElementById('selectedTargetCount');
        
        if (checkedCount > 0) {
            bulkActions.classList.remove('hidden');
            countDisplay.classList.remove('hidden');
            countDisplay.textContent = `${checkedCount} target${checkedCount !== 1 ? 's' : ''} selected`;
        } else {
            bulkActions.classList.add('hidden');
            countDisplay.classList.add('hidden');
        }
    }

    function bulkDownloadTargets() {
        const selectedTargets = Array.from(document.querySelectorAll('.target-checkbox:checked'))
            .map(cb => cb.closest('.bg-gray-800.rounded-lg'));
        
        if (selectedTargets.length === 0) {
            alert('Please select at least one target');
            return;
        }
        
        // Download each selected target
        selectedTargets.forEach((item, index) => {
            const image = item.querySelector('img');
            if (image) {
                // Delay downloads to avoid browser blocking
                setTimeout(() => {
                    downloadImage(image.src, `target-bulk-${index + 1}.jpg`);
                }, index * 100);
            }
        });
    }

    function bulkDeleteTargets() {
        const selectedTargets = Array.from(document.querySelectorAll('.target-checkbox:checked'))
            .map(cb => cb.closest('.bg-gray-800.rounded-lg'));
        
        if (selectedTargets.length === 0) {
            alert('Please select at least one target');
            return;
        }
        
        if (!confirm(`Are you sure you want to delete ${selectedTargets.length} selected target(s)? This action cannot be undone.`)) {
            return;
        }
        
        // Delete each selected target
        selectedTargets.forEach(item => {
            const index = Array.from(targetGrid.querySelectorAll('.bg-gray-800.rounded-lg')).indexOf(item);
            deleteTarget(index, item);
        });
    }

    // Auto-refresh targets (if new targets might be added)
    if (evidenceId) {
        setInterval(() => {
            checkForNewTargets();
        }, 30000); // Check every 30 seconds
    }

    function checkForNewTargets() {
        fetch(`/api/evidence/${evidenceId}/targets/count`)
            .then(response => response.json())
            .then(data => {
                if (data.success) {
                    const currentCount = targetGrid?.querySelectorAll('.bg-gray-800.rounded-lg').length || 0;
                    if (data.count > currentCount) {
                        if (confirm('New targets have been identified. Refresh page?')) {
                            window.location.reload();
                        }
                    }
                }
            })
            .catch(console.error);
    }

    // Keyboard shortcuts for target management
    document.addEventListener('keydown', function(e) {
        // Delete selected targets with Delete key
        if (e.key === 'Delete' && !e.ctrlKey && !e.altKey) {
            const selectedTargets = document.querySelectorAll('.target-checkbox:checked');
            if (selectedTargets.length > 0) {
                e.preventDefault();
                bulkDeleteTargets();
            }
        }
        
        // Download selected targets with Ctrl+D
        if (e.ctrlKey && e.key === 'd') {
            e.preventDefault();
            bulkDownloadTargets();
        }
        
        // Select all targets with Ctrl+A in target section
        if (e.ctrlKey && e.key === 'a' && targetGrid && targetGrid.contains(document.activeElement)) {
            e.preventDefault();
            document.getElementById('selectAllTargets')?.click();
        }
    });

    // ── Phase 5: Dismiss auto-generated target ────────────────────────────────
    // Called from the Dismiss button on auto-detected face cards.
    // Removes the card from the DOM optimistically, then asks the server to
    // delete the target record. If the server call fails the card is restored.
    window.dismissAutoTarget = async function(targetId, buttonEl) {
        const card = buttonEl.closest('[data-target-id]');
        if (!card) return;

        if (!confirm('Remove this auto-detected face? This cannot be undone.')) return;

        // Optimistic removal
        card.style.transition = 'opacity 0.3s, transform 0.3s';
        card.style.opacity = '0';
        card.style.transform = 'scale(0.95)';

        try {
            const res = await fetch(`/api/targets/${targetId}`, { method: 'DELETE' });
            if (res.ok) {
                setTimeout(() => card.remove(), 300);
                showToast('Auto-detected target removed', 'success');
            } else {
                // Restore card on failure
                card.style.opacity = '1';
                card.style.transform = '';
                const data = await res.json().catch(() => ({}));
                showToast(data.error || 'Failed to remove target', 'error');
            }
        } catch (err) {
            card.style.opacity = '1';
            card.style.transform = '';
            showToast('Network error — target not removed', 'error');
        }
    };

    // ── Phase 5: Toast notification helper ───────────────────────────────────
    function showToast(message, type = 'info') {
        const existing = document.querySelector('.target-toast');
        if (existing) existing.remove();

        const colors = {
            success: 'bg-green-600 text-white',
            error:   'bg-red-600 text-white',
            info:    'bg-gray-800 text-white',
        };

        const toast = document.createElement('div');
        toast.className = `target-toast fixed bottom-6 right-6 z-50 px-4 py-3 rounded-lg shadow-lg text-sm font-medium flex items-center gap-2 transition-all duration-300 ${colors[type] || colors.info}`;
        toast.innerHTML = `
            <svg class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                ${type === 'success'
                    ? '<path d="M20 6L9 17l-5-5"/>'
                    : type === 'error'
                    ? '<path d="M18 6L6 18M6 6l12 12"/>'
                    : '<circle cx="12" cy="12" r="10"/><path d="M12 8v4m0 4h.01"/>'}
            </svg>
            ${message}
        `;
        document.body.appendChild(toast);

        // Fade in
        requestAnimationFrame(() => {
            toast.style.opacity = '1';
            setTimeout(() => {
                toast.style.opacity = '0';
                setTimeout(() => toast.remove(), 300);
            }, 3000);
        });
    }

    // ── Phase 5: Highlight newly matched targets on page load ─────────────────
    // If the URL contains ?matched=targetId, briefly pulse that card.
    (function highlightMatchedTarget() {
        const params = new URLSearchParams(window.location.search);
        const matchedId = params.get('matched');
        if (!matchedId) return;

        const card = document.querySelector(`[data-target-id="${matchedId}"]`);
        if (!card) return;

        card.scrollIntoView({ behavior: 'smooth', block: 'center' });
        card.classList.add('ring-2', 'ring-orange-400', 'ring-offset-2');

        // Add a "Matched!" banner overlay briefly
        const banner = document.createElement('div');
        banner.className = 'absolute inset-0 flex items-center justify-center bg-orange-500/20 backdrop-blur-sm z-20 rounded-xl pointer-events-none';
        banner.innerHTML = `
            <span class="px-3 py-1.5 bg-orange-500 text-white text-xs font-bold rounded-full shadow-lg animate-pulse">
                🎯 MATCHED
            </span>
        `;
        card.style.position = 'relative';
        card.appendChild(banner);

        setTimeout(() => {
            banner.remove();
            card.classList.remove('ring-2', 'ring-orange-400', 'ring-offset-2');
        }, 4000);
    })();
});