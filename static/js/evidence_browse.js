// Evidence Browse Page JavaScript
document.addEventListener('DOMContentLoaded', function() {
    console.log('🔍 Evidence browse page loaded');

    // DOM Elements
    const searchInput = document.querySelector('input[name="q"]');
    const filterForm = document.querySelector('form[method="GET"]');
    const filterSelects = document.querySelectorAll('select[name]');
    const filterCheckboxes = document.querySelectorAll('input[type="checkbox"]');
    const tableRows = document.querySelectorAll('tbody tr');
    const sortSelect = document.getElementById('sortSelect');
    const paginationLinks = document.querySelectorAll('a[href*="page="]');
    const quickFilterButtons = document.querySelectorAll('.flex-1.bg-gray-700');
    const clearFiltersButton = document.querySelector('a[href="/evidence/browse"]');

    // Initialize
    initializeFilters();
    initializeTableInteractions();
    initializeQuickFilters();
    initializeSearch();

    // Initialize filters
    function initializeFilters() {
        // Auto-submit select changes
        filterSelects.forEach(select => {
            select.addEventListener('change', function() {
                // Add loading indicator
                const originalText = filterForm.querySelector('button[type="submit"]')?.textContent;
                const submitButton = filterForm.querySelector('button[type="submit"]');
                if (submitButton) {
                    submitButton.innerHTML = '<i class="fas fa-spinner fa-spin mr-2"></i>Applying...';
                    submitButton.disabled = true;
                }
                
                // Submit form
                setTimeout(() => {
                    filterForm.submit();
                }, 300);
            });
        });

        // Handle checkbox changes
        filterCheckboxes.forEach(checkbox => {
            checkbox.addEventListener('change', function() {
                // Submit form with delay to allow multiple selections
                clearTimeout(window.filterTimeout);
                window.filterTimeout = setTimeout(() => {
                    filterForm.submit();
                }, 500);
            });
        });

        // Date range validation
        const dateFrom = document.querySelector('input[name="date_from"]');
        const dateTo = document.querySelector('input[name="date_to"]');
        
        if (dateFrom && dateTo) {
            dateFrom.addEventListener('change', function() {
                if (dateTo.value && this.value > dateTo.value) {
                    dateTo.value = this.value;
                }
            });
            
            dateTo.addEventListener('change', function() {
                if (dateFrom.value && this.value < dateFrom.value) {
                    dateFrom.value = this.value;
                }
            });
        }
    }

    // Initialize table interactions
    function initializeTableInteractions() {
        // Row hover effects
        tableRows.forEach(row => {
            if (!row.classList.contains('text-center')) { // Skip empty state row
                row.addEventListener('mouseenter', function() {
                    this.classList.add('bg-gray-800/70');
                });
                
                row.addEventListener('mouseleave', function() {
                    this.classList.remove('bg-gray-800/70');
                });
                
                // Click to view (if not already a link)
                row.addEventListener('click', function(e) {
                    // Don't trigger if clicking on links or buttons
                    if (e.target.tagName === 'A' || e.target.tagName === 'BUTTON' || e.target.closest('a') || e.target.closest('button')) {
                        return;
                    }
                    
                    const viewLink = this.querySelector('a[href*="/evidence/view/"]');
                    if (viewLink) {
                        window.location.href = viewLink.href;
                    }
                });
                
                // Add pointer cursor
                row.style.cursor = 'pointer';
            }
        });

        // Initialize tooltips
        initializeTableTooltips();
    }

    // Initialize table tooltips
    function initializeTableTooltips() {
        // Add tooltips for truncated text
        const truncatedCells = document.querySelectorAll('.truncate');
        truncatedCells.forEach(cell => {
            if (cell.scrollWidth > cell.clientWidth) {
                cell.title = cell.textContent;
            }
        });

        // Add tooltips for status badges
        const statusBadges = document.querySelectorAll('.inline-flex.items-center.px-2.py-1');
        statusBadges.forEach(badge => {
            const statusText = badge.textContent.trim();
            badge.title = `Status: ${statusText}`;
        });
    }

    // Initialize quick filters
    function initializeQuickFilters() {
        quickFilterButtons.forEach(button => {
            button.addEventListener('click', function(e) {
                // Show loading state
                const originalHTML = this.innerHTML;
                this.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
                this.disabled = true;
                
                // Restore after delay
                setTimeout(() => {
                    this.innerHTML = originalHTML;
                    this.disabled = false;
                }, 1000);
            });
        });

        // Clear filters button
        if (clearFiltersButton) {
            clearFiltersButton.addEventListener('click', function(e) {
                // Show confirmation if filters are active
                const activeFilters = getActiveFilterCount();
                if (activeFilters > 0) {
                    if (!confirm(`Clear ${activeFilters} active filter(s)?`)) {
                        e.preventDefault();
                    }
                }
            });
        }
    }

    // Count active filters
    function getActiveFilterCount() {
        let count = 0;
        
        // Check search
        if (searchInput && searchInput.value.trim()) count++;
        
        // Check selects
        filterSelects.forEach(select => {
            if (select.value) count++;
        });
        
        // Check checkboxes
        filterCheckboxes.forEach(checkbox => {
            if (checkbox.checked) count++;
        });
        
        // Check dates
        const dateFrom = document.querySelector('input[name="date_from"]');
        const dateTo = document.querySelector('input[name="date_to"]');
        if (dateFrom && dateFrom.value) count++;
        if (dateTo && dateTo.value) count++;
        
        return count;
    }

    // Initialize search
    function initializeSearch() {
        if (!searchInput) return;
        
        // Debounced search
        let searchTimeout;
        searchInput.addEventListener('input', function() {
            clearTimeout(searchTimeout);
            searchTimeout = setTimeout(() => {
                // Submit form if search term is long enough
                if (this.value.length >= 2 || this.value.length === 0) {
                    filterForm.submit();
                }
            }, 800);
        });
        
        // Search on Enter
        searchInput.addEventListener('keydown', function(e) {
            if (e.key === 'Enter') {
                e.preventDefault();
                filterForm.submit();
            }
        });
        
        // Clear search button
        if (searchInput.value) {
            const clearButton = document.createElement('button');
            clearButton.type = 'button';
            clearButton.className = 'absolute right-3 top-3 text-gray-500 hover:text-white';
            clearButton.innerHTML = '<i class="fas fa-times"></i>';
            clearButton.addEventListener('click', function() {
                searchInput.value = '';
                filterForm.submit();
            });
            
            const searchContainer = searchInput.parentElement;
            searchContainer.classList.add('relative');
            searchContainer.appendChild(clearButton);
        }
    }

    // Sort functionality
    if (sortSelect) {
        const urlParams = new URLSearchParams(window.location.search);
        const currentSort = urlParams.get('sort_by') || 'newest';
        sortSelect.value = currentSort;
        
        sortSelect.addEventListener('change', function() {
            urlParams.set('sort_by', this.value);
            window.location.search = urlParams.toString();
        });
    }

    // Pagination loading states
    paginationLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            // Don't intercept if it's the current page
            if (this.classList.contains('bg-red-600')) {
                e.preventDefault();
                return;
            }
            
            // Show loading state
            const originalHTML = this.innerHTML;
            this.innerHTML = '<i class="fas fa-spinner fa-spin"></i>';
            
            // Add loading overlay
            const loadingOverlay = document.createElement('div');
            loadingOverlay.className = 'fixed inset-0 bg-black/50 flex items-center justify-center z-50';
            loadingOverlay.innerHTML = `
                <div class="bg-gray-800 p-6 rounded-lg">
                    <div class="w-12 h-12 border-4 border-red-600 border-t-transparent rounded-full animate-spin mx-auto mb-4"></div>
                    <p class="text-white">Loading page...</p>
                </div>
            `;
            document.body.appendChild(loadingOverlay);
            
            // Remove after navigation (won't work if page changes)
            setTimeout(() => {
                if (loadingOverlay.parentElement) {
                    loadingOverlay.remove();
                }
                this.innerHTML = originalHTML;
            }, 3000);
        });
    });

    // Export functionality
    initializeExport();

    function initializeExport() {
        // Create export button if not exists
        const tableHeader = document.querySelector('.px-6.py-4.border-b');
        if (tableHeader && !document.getElementById('exportButton')) {
            const exportButton = document.createElement('button');
            exportButton.id = 'exportButton';
            exportButton.className = 'ml-2 px-4 py-2 bg-green-600 hover:bg-green-700 rounded text-sm';
            exportButton.innerHTML = '<i class="fas fa-download mr-2"></i>Export';
            exportButton.addEventListener('click', exportTableData);
            
            const headerActions = tableHeader.querySelector('.flex.items-center');
            if (headerActions) {
                headerActions.appendChild(exportButton);
            }
        }
    }

    // Export table data to CSV
    function exportTableData() {
        const rows = [];
        const headers = [];
        
        // Get headers
        document.querySelectorAll('thead th').forEach(th => {
            const text = th.textContent.trim();
            if (text && !text.includes('Actions')) {
                headers.push(text);
            }
        });
        
        // Get data rows
        document.querySelectorAll('tbody tr').forEach(row => {
            if (!row.classList.contains('text-center')) { // Skip empty state
                const rowData = [];
                const cells = row.querySelectorAll('td');
                
                cells.forEach((cell, index) => {
                    if (index !== cells.length - 1) { // Skip actions column
                        // Get clean text (remove HTML)
                        const text = cell.textContent.trim().replace(/\s+/g, ' ');
                        rowData.push(`"${text}"`);
                    }
                });
                
                if (rowData.length > 0) {
                    rows.push(rowData.join(','));
                }
            }
        });
        
        if (rows.length === 0) {
            alert('No data to export');
            return;
        }
        
        // Create CSV content
        const csvContent = [
            headers.join(','),
            ...rows
        ].join('\n');
        
        // Create download link
        const blob = new Blob([csvContent], { type: 'text/csv' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = `evidence-export-${new Date().toISOString().slice(0, 10)}.csv`;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
    }

    // Bulk actions
    initializeBulkActions();

    function initializeBulkActions() {
        // Add select all checkbox
        const thead = document.querySelector('thead tr');
        if (thead && !thead.querySelector('input[type="checkbox"]')) {
            const th = document.createElement('th');
            th.className = 'px-6 py-3 text-left text-xs font-medium text-gray-400 uppercase tracking-wider';
            th.innerHTML = '<input type="checkbox" id="selectAll" class="rounded">';
            thead.insertBefore(th, thead.firstChild);
            
            // Add checkbox to each row
            tableRows.forEach(row => {
                if (!row.classList.contains('text-center')) {
                    const td = document.createElement('td');
                    td.className = 'px-6 py-4';
                    td.innerHTML = '<input type="checkbox" class="row-checkbox rounded">';
                    row.insertBefore(td, row.firstChild);
                }
            });
            
            // Select all functionality
            const selectAll = document.getElementById('selectAll');
            selectAll.addEventListener('change', function() {
                const checkboxes = document.querySelectorAll('.row-checkbox');
                checkboxes.forEach(checkbox => {
                    checkbox.checked = this.checked;
                });
                updateBulkActions();
            });
            
            // Individual checkbox changes
            document.addEventListener('change', function(e) {
                if (e.target.classList.contains('row-checkbox')) {
                    updateBulkActions();
                    updateSelectAll();
                }
            });
            
            // Add bulk actions container
            addBulkActionsContainer();
        }
    }

    function addBulkActionsContainer() {
        const container = document.createElement('div');
        container.id = 'bulkActions';
        container.className = 'hidden fixed bottom-4 left-1/2 transform -translate-x-1/2 bg-gray-800 border border-gray-700 rounded-lg shadow-xl p-4 z-50';
        container.innerHTML = `
            <div class="flex items-center space-x-4">
                <span id="selectedCount" class="text-sm text-gray-300">0 selected</span>
                <button id="bulkExport" class="px-3 py-1 bg-blue-600 hover:bg-blue-700 rounded text-sm">
                    <i class="fas fa-download mr-1"></i>Export Selected
                </button>
                <button id="bulkDelete" class="px-3 py-1 bg-red-600 hover:bg-red-700 rounded text-sm">
                    <i class="fas fa-trash mr-1"></i>Delete Selected
                </button>
                <button id="bulkClose" class="px-3 py-1 bg-gray-700 hover:bg-gray-600 rounded text-sm">
                    <i class="fas fa-times mr-1"></i>Close
                </button>
            </div>
        `;
        document.body.appendChild(container);
        
        // Add event listeners
        document.getElementById('bulkExport').addEventListener('click', bulkExport);
        document.getElementById('bulkDelete').addEventListener('click', bulkDelete);
        document.getElementById('bulkClose').addEventListener('click', () => {
            container.classList.add('hidden');
            document.querySelectorAll('.row-checkbox').forEach(cb => cb.checked = false);
            document.getElementById('selectAll').checked = false;
        });
    }

    function updateSelectAll() {
        const checkboxes = document.querySelectorAll('.row-checkbox');
        const checkedCount = Array.from(checkboxes).filter(cb => cb.checked).length;
        const selectAll = document.getElementById('selectAll');
        
        if (checkedCount === 0) {
            selectAll.checked = false;
            selectAll.indeterminate = false;
        } else if (checkedCount === checkboxes.length) {
            selectAll.checked = true;
            selectAll.indeterminate = false;
        } else {
            selectAll.checked = false;
            selectAll.indeterminate = true;
        }
    }

    function updateBulkActions() {
        const checkboxes = document.querySelectorAll('.row-checkbox');
        const checkedCount = Array.from(checkboxes).filter(cb => cb.checked).length;
        const bulkActions = document.getElementById('bulkActions');
        const selectedCount = document.getElementById('selectedCount');
        
        if (checkedCount > 0) {
            selectedCount.textContent = `${checkedCount} selected`;
            bulkActions.classList.remove('hidden');
        } else {
            bulkActions.classList.add('hidden');
        }
    }

    function bulkExport() {
        const selectedRows = Array.from(document.querySelectorAll('.row-checkbox:checked'))
            .map(cb => cb.closest('tr'));
        
        if (selectedRows.length === 0) {
            alert('Please select at least one row to export');
            return;
        }
        
        // Get evidence IDs
        const evidenceIds = selectedRows.map(row => {
            const viewLink = row.querySelector('a[href*="/evidence/view/"]');
            if (viewLink) {
                const match = viewLink.href.match(/\/evidence\/view\/(.+)/);
                return match ? match[1] : null;
            }
            return null;
        }).filter(id => id !== null);
        
        // Export selected
        alert(`Would export ${evidenceIds.length} selected items. In a real app, this would generate a report.`);
    }

    function bulkDelete() {
        const selectedRows = Array.from(document.querySelectorAll('.row-checkbox:checked'))
            .map(cb => cb.closest('tr'));
        
        if (selectedRows.length === 0) {
            alert('Please select at least one row to delete');
            return;
        }
        
        if (!confirm(`Are you sure you want to delete ${selectedRows.length} selected evidence item(s)? This action cannot be undone.`)) {
            return;
        }
        
        // Get evidence IDs
        const evidenceIds = selectedRows.map(row => {
            const viewLink = row.querySelector('a[href*="/evidence/view/"]');
            if (viewLink) {
                const match = viewLink.href.match(/\/evidence\/view\/(.+)/);
                return match ? match[1] : null;
            }
            return null;
        }).filter(id => id !== null);
        
        // Delete selected (simulated)
        console.log('Would delete:', evidenceIds);
        alert(`${evidenceIds.length} items marked for deletion. In a real app, this would send delete requests.`);
    }

    // Keyboard navigation
    document.addEventListener('keydown', function(e) {
        // Navigate table with arrow keys when focused
        if (document.activeElement.closest('tbody')) {
            const currentRow = document.activeElement.closest('tr');
            if (currentRow) {
                let nextRow;
                
                if (e.key === 'ArrowDown') {
                    nextRow = currentRow.nextElementSibling;
                } else if (e.key === 'ArrowUp') {
                    nextRow = currentRow.previousElementSibling;
                }
                
                if (nextRow && !nextRow.classList.contains('text-center')) {
                    e.preventDefault();
                    nextRow.focus();
                    nextRow.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
                }
            }
        }
        
        // Select all with Ctrl+A
        if (e.ctrlKey && e.key === 'a') {
            e.preventDefault();
            const selectAll = document.getElementById('selectAll');
            if (selectAll) {
                selectAll.checked = !selectAll.checked;
                selectAll.dispatchEvent(new Event('change'));
            }
        }
        
        // Export with Ctrl+E
        if (e.ctrlKey && e.key === 'e') {
            e.preventDefault();
            exportTableData();
        }
    });

    // Infinite scroll (if pagination exists)
    initializeInfiniteScroll();

    function initializeInfiniteScroll() {
        if (paginationLinks.length > 0) {
            const observer = new IntersectionObserver((entries) => {
                entries.forEach(entry => {
                    if (entry.isIntersecting) {
                        const nextPageLink = document.querySelector('a[href*="page="]:not(.bg-red-600)');
                        if (nextPageLink) {
                            nextPageLink.click();
                        }
                    }
                });
            }, { threshold: 0.1 });
            
            // Observe the last row
            const lastRow = tableRows[tableRows.length - 1];
            if (lastRow) {
                observer.observe(lastRow);
            }
        }
    }

    // Refresh data periodically
    setInterval(() => {
        const activeFilters = getActiveFilterCount();
        if (activeFilters === 0) { // Only auto-refresh if no filters applied
            fetch('/api/evidence/count')
                .then(response => response.json())
                .then(data => {
                    if (data.success && data.count !== tableRows.length) {
                        if (confirm('New evidence available. Refresh page?')) {
                            window.location.reload();
                        }
                    }
                })
                .catch(console.error);
        }
    }, 60000); // Check every minute
});