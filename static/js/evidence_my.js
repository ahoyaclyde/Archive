// My Evidence Page JavaScript
document.addEventListener('DOMContentLoaded', function() {
    console.log('👤 My Evidence page loaded');

    // DOM Elements
    const tabs = document.querySelectorAll('.border-b-2');
    const evidenceCards = document.querySelectorAll('.bg-gray-800.rounded-lg');
    const paginationLinks = document.querySelectorAll('a[href*="page="]');
    const statsCards = document.querySelectorAll('.bg-gray-800.rounded-lg.p-6');
    const emptyState = document.querySelector('.text-center.py-16');
    const deleteButtons = document.querySelectorAll('.delete-btn');

    // Initialize
    initializeTabs();
    initializeEvidenceCards();
    initializePagination();
    initializeStats();
    initializeDeleteButtons();
    initializeSearch();

    // Initialize tabs
    function initializeTabs() {
        tabs.forEach(tab => {
            tab.addEventListener('click', function(e) {
                // Show loading state
                const originalHTML = this.innerHTML;
                this.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
                
                // Add active class to clicked tab, remove from others
                tabs.forEach(t => {
                    t.classList.remove('border-red-500', 'text-red-500');
                    t.classList.add('border-transparent', 'text-gray-500');
                });
                
                this.classList.add('border-red-500', 'text-red-500');
                this.classList.remove('border-transparent', 'text-gray-500');
                
                // Restore after navigation
                setTimeout(() => {
                    this.innerHTML = originalHTML;
                }, 1000);
            });
        });
    }

    // Initialize evidence cards
    function initializeEvidenceCards() {
        evidenceCards.forEach(card => {
            // Hover effects
            card.addEventListener('mouseenter', function() {
                this.style.transform = 'translateY(-4px) scale(1.02)';
                this.style.transition = 'transform 0.3s ease, box-shadow 0.3s ease';
                this.style.boxShadow = '0 10px 25px -5px rgba(239, 68, 68, 0.1)';
            });
            
            card.addEventListener('mouseleave', function() {
                this.style.transform = 'translateY(0) scale(1)';
                this.style.boxShadow = 'none';
            });
            
            // Status badge tooltips
            const statusBadge = card.querySelector('.px-2.py-1.text-xs');
            if (statusBadge) {
                const statusText = statusBadge.textContent.trim();
                statusBadge.title = `Status: ${statusText}`;
            }
            
            // Emergency level tooltips
            const emergencyBadge = card.querySelectorAll('.px-2.py-1.text-xs')[1];
            if (emergencyBadge) {
                const emergencyText = emergencyBadge.textContent.trim();
                emergencyBadge.title = `Emergency Level: ${emergencyText}`;
            }
            
            // Quick actions menu
            addQuickActionsMenu(card);
        });
    }

    // Add quick actions menu to each card
    function addQuickActionsMenu(card) {
        const viewLink = card.querySelector('a[href*="/evidence/view/"]');
        const completeLink = card.querySelector('a[href*="/evidence/complete/"]');
        
        if (viewLink || completeLink) {
            const actionsContainer = card.querySelector('.flex.space-x-2');
            if (actionsContainer) {
                // Create dropdown button
                const dropdownButton = document.createElement('button');
                dropdownButton.className = 'px-2 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm transition-colors';
                dropdownButton.innerHTML = '<i class="fas fa-ellipsis-h"></i>';
                
                // Create dropdown menu
                const dropdownMenu = document.createElement('div');
                dropdownMenu.className = 'hidden absolute right-0 mt-2 w-48 bg-gray-800 border border-gray-700 rounded-lg shadow-xl z-10';
                dropdownMenu.innerHTML = `
                    <div class="py-1">
                        ${viewLink ? `<a href="${viewLink.href}" class="block px-4 py-2 text-sm text-gray-300 hover:bg-gray-700">
                            <i class="fas fa-eye mr-2"></i>View Details
                        </a>` : ''}
                        ${completeLink ? `<a href="${completeLink.href}" class="block px-4 py-2 text-sm text-yellow-300 hover:bg-gray-700">
                            <i class="fas fa-edit mr-2"></i>Complete
                        </a>` : ''}
                        <a href="#" class="block px-4 py-2 text-sm text-blue-300 hover:bg-gray-700">
                            <i class="fas fa-share mr-2"></i>Share
                        </a>
                        <a href="#" class="block px-4 py-2 text-sm text-red-300 hover:bg-gray-700 delete-card-btn">
                            <i class="fas fa-trash mr-2"></i>Delete
                        </a>
                    </div>
                `;
                
                // Position container
                const positionContainer = document.createElement('div');
                positionContainer.className = 'relative';
                positionContainer.appendChild(dropdownButton);
                positionContainer.appendChild(dropdownMenu);
                actionsContainer.appendChild(positionContainer);
                
                // Toggle dropdown
                dropdownButton.addEventListener('click', function(e) {
                    e.stopPropagation();
                    dropdownMenu.classList.toggle('hidden');
                });
                
                // Close dropdown when clicking elsewhere
                document.addEventListener('click', function() {
                    dropdownMenu.classList.add('hidden');
                });
                
                // Handle delete action
                const deleteBtn = dropdownMenu.querySelector('.delete-card-btn');
                if (deleteBtn) {
                    deleteBtn.addEventListener('click', function(e) {
                        e.preventDefault();
                        const evidenceId = getEvidenceIdFromCard(card);
                        if (evidenceId) {
                            deleteEvidence(evidenceId, card);
                        }
                    });
                }
            }
        }
    }

    // Get evidence ID from card
    function getEvidenceIdFromCard(card) {
        const viewLink = card.querySelector('a[href*="/evidence/view/"]');
        if (viewLink) {
            const match = viewLink.href.match(/\/evidence\/view\/(.+)/);
            return match ? match[1] : null;
        }
        return null;
    }

    // Delete evidence
    function deleteEvidence(evidenceId, card) {
        if (!confirm('Are you sure you want to delete this evidence? This action cannot be undone.')) {
            return;
        }
        
        // Show loading
        card.style.opacity = '0.5';
        card.style.pointerEvents = 'none';
        
        // Send delete request
        fetch(`/api/evidence/${evidenceId}/delete`, {
            method: 'POST'
        })
        .then(response => response.json())
        .then(data => {
            if (data.success) {
                // Remove card with animation
                card.style.transition = 'opacity 0.3s ease, transform 0.3s ease';
                card.style.opacity = '0';
                card.style.transform = 'scale(0.8)';
                
                setTimeout(() => {
                    card.remove();
                    updateEmptyState();
                    updateStatsAfterDelete();
                }, 300);
            } else {
                alert('Error: ' + data.message);
                card.style.opacity = '1';
                card.style.pointerEvents = 'auto';
            }
        })
        .catch(error => {
            console.error('Delete error:', error);
            alert('Failed to delete evidence');
            card.style.opacity = '1';
            card.style.pointerEvents = 'auto';
        });
    }

    // Update empty state visibility
    function updateEmptyState() {
        const remainingCards = document.querySelectorAll('.bg-gray-800.rounded-lg.border');
        if (remainingCards.length === 0 && emptyState) {
            emptyState.classList.remove('hidden');
        }
    }

    // Update stats after delete
    function updateStatsAfterDelete() {
        // Update total count
        const totalCountElement = statsCards[0]?.querySelector('.text-2xl');
        if (totalCountElement) {
            const currentCount = parseInt(totalCountElement.textContent);
            if (!isNaN(currentCount)) {
                animateCountChange(totalCountElement, currentCount, currentCount - 1);
            }
        }
    }

    // Animate count change
    function animateCountChange(element, from, to) {
        const duration = 500;
        const steps = 30;
        const increment = (to - from) / steps;
        let current = from;
        let step = 0;

        const timer = setInterval(() => {
            current += increment;
            step++;
            
            if (step >= steps) {
                element.textContent = to;
                clearInterval(timer);
            } else {
                element.textContent = Math.floor(current);
            }
        }, duration / steps);
    }

    // Initialize pagination
    function initializePagination() {
        paginationLinks.forEach(link => {
            link.addEventListener('click', function(e) {
                // Don't intercept if it's the current page
                if (this.classList.contains('bg-red-600')) {
                    e.preventDefault();
                    return;
                }
                
                // Show loading overlay
                showLoadingOverlay('Loading your evidence...');
            });
        });
    }

    // Show loading overlay
    function showLoadingOverlay(message) {
        const overlay = document.createElement('div');
        overlay.className = 'fixed inset-0 bg-black/70 flex items-center justify-center z-50';
        overlay.innerHTML = `
            <div class="bg-gray-800 rounded-lg p-8 text-center">
                <div class="w-16 h-16 border-4 border-red-600 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
                <h3 class="text-xl font-bold mb-2">Loading</h3>
                <p class="text-gray-400">${message}</p>
            </div>
        `;
        document.body.appendChild(overlay);
        
        // Remove after 3 seconds (should be removed by page navigation)
        setTimeout(() => {
            if (overlay.parentElement) {
                overlay.remove();
            }
        }, 3000);
    }

    // Initialize stats
    function initializeStats() {
        statsCards.forEach(card => {
            // Add hover effect
            card.addEventListener('mouseenter', function() {
                this.style.transform = 'translateY(-2px)';
                this.style.transition = 'transform 0.2s ease';
            });
            
            card.addEventListener('mouseleave', function() {
                this.style.transform = 'translateY(0)';
            });
            
            // Add click to filter
            card.addEventListener('click', function() {
                const label = this.querySelector('.text-sm.text-gray-400')?.textContent;
                if (label) {
                    filterByStat(label.toLowerCase());
                }
            });
            
            // Add tooltip
            card.title = 'Click to filter by this status';
        });
    }

    // Filter by stat
    function filterByStat(stat) {
        let status = 'all';
        
        switch(stat) {
            case 'drafts':
                status = 'draft';
                break;
            case 'submitted':
                status = 'submitted';
                break;
            case 'needs attention':
                status = 'needs_attention';
                break;
        }
        
        window.location.href = `/evidence/my?status=${status}`;
    }

    // Initialize delete buttons
    function initializeDeleteButtons() {
        deleteButtons.forEach(button => {
            button.addEventListener('click', function(e) {
                e.preventDefault();
                
                const card = this.closest('.bg-gray-800.rounded-lg');
                const evidenceId = getEvidenceIdFromCard(card);
                
                if (evidenceId) {
                    deleteEvidence(evidenceId, card);
                }
            });
        });
    }

    // Initialize search
    function initializeSearch() {
        const searchInput = document.querySelector('input[type="search"]');
        if (searchInput) {
            // Debounced search
            let searchTimeout;
            searchInput.addEventListener('input', function() {
                clearTimeout(searchTimeout);
                searchTimeout = setTimeout(() => {
                    searchMyEvidence(this.value);
                }, 500);
            });
            
            // Clear search button
            if (searchInput.value) {
                addClearSearchButton(searchInput);
            }
        }
    }

    // Add clear search button
    function addClearSearchButton(searchInput) {
        const clearButton = document.createElement('button');
        clearButton.type = 'button';
        clearButton.className = 'absolute right-3 top-1/2 transform -translate-y-1/2 text-gray-500 hover:text-white';
        clearButton.innerHTML = '<i class="fas fa-times"></i>';
        clearButton.addEventListener('click', function() {
            searchInput.value = '';
            searchInput.focus();
            searchMyEvidence('');
        });
        
        const container = searchInput.parentElement;
        container.classList.add('relative');
        container.appendChild(clearButton);
    }

    // Search my evidence
    function searchMyEvidence(query) {
        if (query.trim().length >= 2 || query.length === 0) {
            // Show loading
            const cardsContainer = document.querySelector('.grid.grid-cols-1');
            if (cardsContainer) {
                cardsContainer.style.opacity = '0.5';
                cardsContainer.style.pointerEvents = 'none';
            }
            
            // Simulate search (in real app, this would be an API call)
            setTimeout(() => {
                if (cardsContainer) {
                    cardsContainer.style.opacity = '1';
                    cardsContainer.style.pointerEvents = 'auto';
                }
                
                // Filter cards locally for demo
                filterCardsByQuery(query);
            }, 300);
        }
    }

    // Filter cards by query
    function filterCardsByQuery(query) {
        if (!query.trim()) {
            // Show all cards
            evidenceCards.forEach(card => {
                card.style.display = 'block';
            });
            return;
        }
        
        const searchTerm = query.toLowerCase();
        let matchCount = 0;
        
        evidenceCards.forEach(card => {
            const title = card.querySelector('.font-bold.text-lg')?.textContent.toLowerCase() || '';
            const description = card.querySelector('.text-sm.text-gray-300')?.textContent.toLowerCase() || '';
            const county = card.querySelector('.flex.items-center span')?.textContent.toLowerCase() || '';
            const evidenceNumber = card.querySelector('.text-xs.text-gray-500')?.textContent.toLowerCase() || '';
            
            const matches = title.includes(searchTerm) ||
                           description.includes(searchTerm) ||
                           county.includes(searchTerm) ||
                           evidenceNumber.includes(searchTerm);
            
            if (matches) {
                card.style.display = 'block';
                matchCount++;
                
                // Highlight search term
                highlightText(card, searchTerm);
            } else {
                card.style.display = 'none';
            }
        });
        
        // Show no results message
        showSearchResultsCount(matchCount, searchTerm);
    }

    // Highlight search term in card
    function highlightText(card, searchTerm) {
        const elements = card.querySelectorAll('.font-bold.text-lg, .text-sm.text-gray-300, .flex.items-center span, .text-xs.text-gray-500');
        
        elements.forEach(element => {
            const originalHTML = element.innerHTML;
            const regex = new RegExp(`(${searchTerm})`, 'gi');
            const highlighted = originalHTML.replace(regex, '<span class="bg-yellow-600 text-white px-1 rounded">$1</span>');
            element.innerHTML = highlighted;
        });
    }

    // Show search results count
    function showSearchResultsCount(count, query) {
        // Remove existing message
        const existingMessage = document.querySelector('.search-results-message');
        if (existingMessage) {
            existingMessage.remove();
        }
        
        if (query.trim()) {
            const message = document.createElement('div');
            message.className = 'search-results-message mb-4 p-4 bg-gray-800 rounded-lg';
            message.innerHTML = `
                <div class="flex items-center justify-between">
                    <div>
                        <span class="font-medium">${count} result${count !== 1 ? 's' : ''} found for "${query}"</span>
                        <span class="text-sm text-gray-400 ml-2">in your evidence</span>
                    </div>
                    <button class="text-sm text-gray-400 hover:text-white clear-search-btn">
                        Clear search
                    </button>
                </div>
            `;
            
            const container = document.querySelector('.mb-8');
            if (container) {
                container.appendChild(message);
                
                // Clear search button
                message.querySelector('.clear-search-btn').addEventListener('click', function() {
                    const searchInput = document.querySelector('input[type="search"]');
                    if (searchInput) {
                        searchInput.value = '';
                        searchMyEvidence('');
                    }
                    message.remove();
                });
            }
        }
    }

    // Bulk actions
    initializeBulkActions();

    function initializeBulkActions() {
        // Add select all checkbox to header
        const header = document.querySelector('.mb-8');
        if (header && evidenceCards.length > 0) {
            const bulkContainer = document.createElement('div');
            bulkContainer.className = 'mb-4 flex items-center space-x-4';
            bulkContainer.innerHTML = `
                <label class="flex items-center">
                    <input type="checkbox" id="selectAllCards" class="mr-2 rounded">
                    <span class="text-sm text-gray-300">Select all</span>
                </label>
                <div id="bulkActionsContainer" class="hidden space-x-2">
                    <button id="bulkDeleteCards" class="px-3 py-1 bg-red-600 hover:bg-red-700 rounded text-sm">
                        <i class="fas fa-trash mr-1"></i>Delete Selected
                    </button>
                    <button id="bulkExportCards" class="px-3 py-1 bg-blue-600 hover:bg-blue-700 rounded text-sm">
                        <i class="fas fa-download mr-1"></i>Export Selected
                    </button>
                </div>
            `;
            
            header.appendChild(bulkContainer);
            
            // Add checkboxes to each card
            evidenceCards.forEach(card => {
                const checkbox = document.createElement('input');
                checkbox.type = 'checkbox';
                checkbox.className = 'card-checkbox absolute top-3 left-3 z-10 rounded';
                checkbox.style.display = 'none'; // Hidden by default
                
                card.style.position = 'relative';
                card.appendChild(checkbox);
                
                // Show checkbox on card hover
                card.addEventListener('mouseenter', function() {
                    checkbox.style.display = 'block';
                });
                
                card.addEventListener('mouseleave', function() {
                    if (!checkbox.checked) {
                        checkbox.style.display = 'none';
                    }
                });
                
                // Toggle card selection style
                checkbox.addEventListener('change', function() {
                    if (this.checked) {
                        card.classList.add('ring-2', 'ring-red-500');
                    } else {
                        card.classList.remove('ring-2', 'ring-red-500');
                    }
                    updateBulkActionsState();
                });
            });
            
            // Select all functionality
            const selectAll = document.getElementById('selectAllCards');
            selectAll.addEventListener('change', function() {
                const checkboxes = document.querySelectorAll('.card-checkbox');
                checkboxes.forEach(checkbox => {
                    checkbox.checked = this.checked;
                    checkbox.style.display = this.checked ? 'block' : 'none';
                    
                    const card = checkbox.closest('.bg-gray-800.rounded-lg');
                    if (card) {
                        if (this.checked) {
                            card.classList.add('ring-2', 'ring-red-500');
                        } else {
                            card.classList.remove('ring-2', 'ring-red-500');
                        }
                    }
                });
                updateBulkActionsState();
            });
            
            // Bulk delete
            document.getElementById('bulkDeleteCards').addEventListener('click', bulkDeleteCards);
            
            // Bulk export
            document.getElementById('bulkExportCards').addEventListener('click', bulkExportCards);
        }
    }

    function updateBulkActionsState() {
        const checkboxes = document.querySelectorAll('.card-checkbox');
        const checkedCount = Array.from(checkboxes).filter(cb => cb.checked).length;
        const bulkContainer = document.getElementById('bulkActionsContainer');
        const selectAll = document.getElementById('selectAllCards');
        
        if (checkedCount > 0) {
            bulkContainer.classList.remove('hidden');
            selectAll.indeterminate = checkedCount < checkboxes.length;
        } else {
            bulkContainer.classList.add('hidden');
            selectAll.indeterminate = false;
        }
    }

    function bulkDeleteCards() {
        const selectedCards = Array.from(document.querySelectorAll('.card-checkbox:checked'))
            .map(cb => cb.closest('.bg-gray-800.rounded-lg'));
        
        if (selectedCards.length === 0) {
            alert('Please select at least one evidence item');
            return;
        }
        
        if (!confirm(`Are you sure you want to delete ${selectedCards.length} selected evidence item(s)? This action cannot be undone.`)) {
            return;
        }
        
        // Delete each selected card
        selectedCards.forEach(card => {
            const evidenceId = getEvidenceIdFromCard(card);
            if (evidenceId) {
                deleteEvidence(evidenceId, card);
            }
        });
    }

    function bulkExportCards() {
        const selectedCards = Array.from(document.querySelectorAll('.card-checkbox:checked'))
            .map(cb => cb.closest('.bg-gray-800.rounded-lg'));
        
        if (selectedCards.length === 0) {
            alert('Please select at least one evidence item');
            return;
        }
        
        // Get evidence IDs
        const evidenceIds = selectedCards.map(card => getEvidenceIdFromCard(card)).filter(id => id !== null);
        
        // Create export data
        const exportData = selectedCards.map(card => {
            return {
                title: card.querySelector('.font-bold.text-lg')?.textContent || '',
                evidenceNumber: card.querySelector('.text-xs.text-gray-500')?.textContent || '',
                county: card.querySelector('.flex.items-center span')?.textContent || '',
                date: card.querySelectorAll('.flex.items-center span')[1]?.textContent || '',
                status: card.querySelector('.px-2.py-1.text-xs')?.textContent || ''
            };
        });
        
        // Convert to CSV
        const csvContent = [
            ['Title', 'Case #', 'County', 'Date', 'Status'].join(','),
            ...exportData.map(row => Object.values(row).map(v => `"${v}"`).join(','))
        ].join('\n');
        
        // Download
        const blob = new Blob([csvContent], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `my-evidence-export-${new Date().toISOString().slice(0, 10)}.csv`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // Refresh data periodically
    setInterval(() => {
        fetch('/api/evidence/my/count')
            .then(response => response.json())
            .then(data => {
                if (data.success && data.count !== evidenceCards.length) {
                    if (confirm('Your evidence list has changed. Refresh page?')) {
                        window.location.reload();
                    }
                }
            })
            .catch(console.error);
    }, 30000); // Check every 30 seconds

    // Keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        // Refresh with Ctrl+R
        if (e.ctrlKey && e.key === 'r') {
            e.preventDefault();
            window.location.reload();
        }
        
        // Search with /
        if (e.key === '/' && !e.ctrlKey && !e.altKey) {
            const searchInput = document.querySelector('input[type="search"]');
            if (searchInput) {
                e.preventDefault();
                searchInput.focus();
            }
        }
        
        // Select all with Ctrl+A
        if (e.ctrlKey && e.key === 'a') {
            e.preventDefault();
            const selectAll = document.getElementById('selectAllCards');
            if (selectAll) {
                selectAll.checked = !selectAll.checked;
                selectAll.dispatchEvent(new Event('change'));
            }
        }
    });
});