// Evidence Dashboard JavaScript
document.addEventListener('DOMContentLoaded', function() {
    console.log('🎬 Dashboard loaded');

    // DOM Elements
    const statsCards = document.querySelectorAll('.bg-gradient-to-r');
    const recentEvidenceGrid = document.querySelector('.grid.grid-cols-1');
    const countyStats = document.querySelector('.space-y-2');
    const typeStats = document.querySelectorAll('.space-y-2')[1];
    const walletButton = document.querySelector('.bg-yellow-600');
    const walletStatus = document.querySelector('.bg-green-900\\/20');

    // Initialize animations
    animateStatsCards();
    
    // Update real-time stats if needed
    updateRealTimeStats();

    // Animate stats cards on scroll
    function animateStatsCards() {
        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    entry.target.classList.add('fade-in');
                    // Add subtle animation to numbers
                    const numberElement = entry.target.querySelector('.text-3xl');
                    if (numberElement) {
                        animateNumber(numberElement);
                    }
                }
            });
        }, { threshold: 0.1 });

        statsCards.forEach(card => observer.observe(card));
    }

    // Animate number counting
    function animateNumber(element) {
        const targetNumber = parseInt(element.textContent);
        const duration = 1500; // ms
        const steps = 60;
        const increment = targetNumber / steps;
        let current = 0;
        let step = 0;

        const timer = setInterval(() => {
            current += increment;
            step++;
            
            if (step >= steps) {
                element.textContent = targetNumber.toLocaleString();
                clearInterval(timer);
            } else {
                element.textContent = Math.floor(current).toLocaleString();
            }
        }, duration / steps);
    }

    // Update real-time stats (could fetch from API)
    function updateRealTimeStats() {
        // This would make API calls to get updated stats
        // For now, just log
        console.log('Updating real-time stats...');
        
        // Example: Fetch updated stats every 30 seconds
        setInterval(() => {
            fetch('/api/dashboard/stats')
                .then(response => response.json())
                .then(data => {
                    if (data.success) {
                        updateStatsDisplay(data.data);
                    }
                })
                .catch(error => console.error('Error fetching stats:', error));
        }, 30000);
    }

    // Update stats display
    function updateStatsDisplay(stats) {
        const elements = {
            'total_evidence': document.querySelector('.text-3xl.font-bold'),
            'urgent_count': document.querySelector('.text-3xl.font-bold.text-red-400'),
            'reported_count': document.querySelector('.text-3xl.font-bold.text-green-400'),
            'needs_attention_count': document.querySelector('.text-3xl.font-bold.text-yellow-400')
        };

        for (const [key, element] of Object.entries(elements)) {
            if (element && stats[key] !== undefined) {
                const current = parseInt(element.textContent.replace(/,/g, ''));
                const target = stats[key];
                
                if (current !== target) {
                    animateNumberChange(element, current, target);
                }
            }
        }
    }

    // Animate number change
    function animateNumberChange(element, from, to) {
        const duration = 1000;
        const steps = 30;
        const increment = (to - from) / steps;
        let current = from;
        let step = 0;

        const timer = setInterval(() => {
            current += increment;
            step++;
            
            if (step >= steps) {
                element.textContent = to.toLocaleString();
                clearInterval(timer);
            } else {
                element.textContent = Math.floor(current).toLocaleString();
            }
        }, duration / steps);
    }

    // Handle wallet connection
    if (walletButton) {
        walletButton.addEventListener('click', async function(e) {
            e.preventDefault();
            
            try {
                // Check if wallet is available
                if (typeof window.ethereum === 'undefined') {
                    alert('Please install MetaMask or another Ethereum wallet to connect');
                    return;
                }
                
                // Request account access
                const accounts = await window.ethereum.request({ 
                    method: 'eth_requestAccounts' 
                });
                
                if (accounts.length > 0) {
                    // Get chain ID
                    const chainId = await window.ethereum.request({ 
                        method: 'eth_chainId' 
                    });
                    
                    // Map chain ID to chain name
                    const chainMap = {
                        '0x1': 'ethereum',
                        '0x89': 'polygon',
                        '0xa4b1': 'arbitrum',
                        '0xa86a': 'avalanche',
                        '0x38': 'binance',
                        '0x2105': 'base'
                    };
                    
                    const chainName = chainMap[chainId] || 'ethereum';
                    
                    // Send to server
                    const response = await fetch('/api/connect-wallet', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                            address: accounts[0],
                            chain: chainName
                        })
                    });
                    
                    const result = await response.json();
                    
                    if (result.success) {
                        // Update UI
                        updateWalletUI(accounts[0], chainName);
                        alert('Wallet connected successfully!');
                    } else {
                        throw new Error(result.message);
                    }
                }
            } catch (error) {
                console.error('Wallet connection error:', error);
                alert('Failed to connect wallet: ' + error.message);
            }
        });
    }

    // Update wallet UI
    function updateWalletUI(address, chain) {
        // Update or create wallet status display
        if (walletStatus) {
            walletStatus.innerHTML = `
                <div class="flex items-center justify-between">
                    <div class="flex items-center space-x-3">
                        <div class="w-10 h-10 rounded-full bg-green-600 flex items-center justify-center">
                            <i class="fas fa-wallet"></i>
                        </div>
                        <div>
                            <div class="font-semibold flex items-center">
                                Wallet Connected
                                <span class="ml-2 px-2 py-1 text-xs rounded bg-purple-600">${chain}</span>
                            </div>
                            <div class="text-sm text-gray-300 font-mono">
                                ${address.substring(0, 6)}...${address.substring(address.length - 4)}
                            </div>
                        </div>
                    </div>
                    <form method="POST" action="/disconnect-wallet" class="inline">
                        <button type="submit" 
                                class="text-sm bg-red-600 px-3 py-1 rounded hover:bg-red-700">
                            <i class="fas fa-unlink mr-1"></i>Disconnect
                        </button>
                    </form>
                </div>
                <div class="mt-3 text-sm text-green-300">
                    <i class="fas fa-check-circle mr-1"></i>
                    Your evidence will be cryptographically signed and verified on-chain.
                </div>
            `;
        }
        
        // Remove connect wallet button
        if (walletButton) {
            walletButton.remove();
        }
    }

    // Handle wallet disconnection
    document.addEventListener('submit', function(e) {
        if (e.target.action && e.target.action.includes('disconnect-wallet')) {
            e.preventDefault();
            
            if (confirm('Are you sure you want to disconnect your wallet?')) {
                fetch(e.target.action, {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/x-www-form-urlencoded',
                    },
                    body: new URLSearchParams({})
                })
                .then(response => response.json())
                .then(data => {
                    if (data.success) {
                        window.location.reload();
                    } else {
                        alert('Failed to disconnect wallet: ' + data.message);
                    }
                })
                .catch(error => {
                    console.error('Disconnect error:', error);
                    alert('An error occurred while disconnecting wallet');
                });
            }
        }
    });

    // Refresh dashboard data
    const refreshButton = document.createElement('button');
    refreshButton.className = 'bg-gray-700 hover:bg-gray-600 px-3 py-2 rounded text-sm ml-2';
    refreshButton.innerHTML = '<i class="fas fa-sync-alt mr-1"></i>Refresh';
    refreshButton.addEventListener('click', refreshDashboard);
    
    // Add refresh button to header if not already present
    const headerActions = document.querySelector('.flex.items-center.space-x-4');
    if (headerActions && !headerActions.querySelector('.bg-gray-700.hover\\:bg-gray-600')) {
        headerActions.appendChild(refreshButton);
    }

    function refreshDashboard() {
        refreshButton.disabled = true;
        refreshButton.innerHTML = '<i class="fas fa-spinner fa-spin mr-1"></i>Refreshing...';
        
        // Simulate refresh (in real app, this would fetch new data)
        setTimeout(() => {
            // Reload the page for now
            window.location.reload();
        }, 1000);
    }

    // Chart initialization (if charts are used)
    initializeCharts();

    function initializeCharts() {
        // Check if any chart containers exist
        const chartContainers = document.querySelectorAll('.chart-container');
        
        if (chartContainers.length > 0) {
            // Load Chart.js if not already loaded
            if (typeof Chart === 'undefined') {
                const script = document.createElement('script');
                script.src = 'https://cdn.jsdelivr.net/npm/chart.js';
                script.onload = createCharts;
                document.head.appendChild(script);
            } else {
                createCharts();
            }
        }
    }

    function createCharts() {
        // Create charts based on data
        const countyData = parseCountyStats();
        const typeData = parseTypeStats();
        
        if (countyData && countyData.labels.length > 0) {
            createCountyChart(countyData);
        }
        
        if (typeData && typeData.labels.length > 0) {
            createTypeChart(typeData);
        }
    }

    function parseCountyStats() {
        const countyItems = countyStats?.querySelectorAll('.flex.items-center.justify-between');
        if (!countyItems || countyItems.length === 0) return null;
        
        const labels = [];
        const data = [];
        const colors = ['#EF4444', '#F97316', '#EAB308', '#22C55E', '#3B82F6'];
        
        countyItems.forEach((item, index) => {
            const label = item.querySelector('.text-sm')?.textContent;
            const countText = item.querySelector('.text-xs.text-gray-400')?.textContent;
            
            if (label && countText) {
                const count = parseInt(countText.replace(' cases', ''));
                if (!isNaN(count)) {
                    labels.push(label);
                    data.push(count);
                }
            }
        });
        
        return { labels, data, colors };
    }

    function parseTypeStats() {
        const typeItems = typeStats?.querySelectorAll('.flex.items-center.justify-between');
        if (!typeItems || typeItems.length === 0) return null;
        
        const labels = [];
        const data = [];
        const colors = ['#8B5CF6', '#EC4899', '#14B8A6', '#F59E0B', '#6366F1'];
        
        typeItems.forEach((item, index) => {
            const label = item.querySelector('.text-sm')?.textContent;
            const countText = item.querySelector('.text-xs.text-gray-400')?.textContent;
            
            if (label && countText) {
                const count = parseInt(countText.replace(' cases', ''));
                if (!isNaN(count)) {
                    labels.push(label);
                    data.push(count);
                }
            }
        });
        
        return { labels, data, colors };
    }

    function createCountyChart(data) {
        const canvas = document.createElement('canvas');
        canvas.id = 'countyChart';
        canvas.className = 'mt-4';
        
        const container = countyStats?.parentElement;
        if (container) {
            container.appendChild(canvas);
            
            new Chart(canvas, {
                type: 'doughnut',
                data: {
                    labels: data.labels,
                    datasets: [{
                        data: data.data,
                        backgroundColor: data.colors,
                        borderWidth: 2,
                        borderColor: '#1F2937'
                    }]
                },
                options: {
                    responsive: true,
                    plugins: {
                        legend: {
                            position: 'bottom',
                            labels: {
                                color: '#9CA3AF',
                                padding: 20,
                                font: {
                                    size: 11
                                }
                            }
                        },
                        tooltip: {
                            backgroundColor: 'rgba(31, 41, 55, 0.9)',
                            titleColor: '#F3F4F6',
                            bodyColor: '#F3F4F6'
                        }
                    }
                }
            });
        }
    }

    function createTypeChart(data) {
        const canvas = document.createElement('canvas');
        canvas.id = 'typeChart';
        canvas.className = 'mt-4';
        
        const container = typeStats?.parentElement;
        if (container) {
            container.appendChild(canvas);
            
            new Chart(canvas, {
                type: 'bar',
                data: {
                    labels: data.labels,
                    datasets: [{
                        label: 'Cases',
                        data: data.data,
                        backgroundColor: data.colors,
                        borderRadius: 6
                    }]
                },
                options: {
                    responsive: true,
                    plugins: {
                        legend: {
                            display: false
                        },
                        tooltip: {
                            backgroundColor: 'rgba(31, 41, 55, 0.9)',
                            titleColor: '#F3F4F6',
                            bodyColor: '#F3F4F6'
                        }
                    },
                    scales: {
                        y: {
                            beginAtZero: true,
                            grid: {
                                color: 'rgba(75, 85, 99, 0.2)'
                            },
                            ticks: {
                                color: '#9CA3AF'
                            }
                        },
                        x: {
                            grid: {
                                display: false
                            },
                            ticks: {
                                color: '#9CA3AF',
                                maxRotation: 45
                            }
                        }
                    }
                }
            });
        }
    }

    // Recent evidence hover effects
    if (recentEvidenceGrid) {
        const evidenceCards = recentEvidenceGrid.querySelectorAll('a.group');
        evidenceCards.forEach(card => {
            card.addEventListener('mouseenter', function() {
                this.style.transform = 'translateY(-4px)';
                this.style.transition = 'transform 0.3s ease';
            });
            
            card.addEventListener('mouseleave', function() {
                this.style.transform = 'translateY(0)';
            });
        });
    }

    // Initialize tooltips
    initializeTooltips();

    function initializeTooltips() {
        // Initialize tooltips for stats cards
        statsCards.forEach(card => {
            const tooltipText = card.querySelector('.text-sm.text-gray-400')?.textContent;
            if (tooltipText) {
                card.title = tooltipText;
            }
        });
    }

    // Keyboard shortcuts
    document.addEventListener('keydown', function(e) {
        // Refresh with Ctrl+R (but prevent browser refresh)
        if (e.ctrlKey && e.key === 'r') {
            e.preventDefault();
            refreshDashboard();
        }
        
        // Focus search with /
        if (e.key === '/' && !e.ctrlKey && !e.altKey) {
            const searchInput = document.querySelector('input[type="text"]');
            if (searchInput) {
                e.preventDefault();
                searchInput.focus();
            }
        }
    });
});