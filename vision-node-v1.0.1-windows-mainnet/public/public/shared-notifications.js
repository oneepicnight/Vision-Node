// Shared Notification System for Vision Node
// Include this script on any page that needs notifications

(function() {
    const API_BASE = window.location.origin;
    let currentWallet = '';
    let notifications = [];
    let notificationPanelOpen = false;
    let notificationCheckInterval = null;

    // Get wallet from localStorage or URL params
    function initializeWallet() {
        // Try URL params first
        const urlParams = new URLSearchParams(window.location.search);
        const walletParam = urlParams.get('wallet');
        
        if (walletParam) {
            currentWallet = walletParam;
            localStorage.setItem('visionWalletAddress', walletParam);
        } else {
            currentWallet = localStorage.getItem('visionWalletAddress') || '';
        }
        
        return currentWallet;
    }

    // Load notifications from API
    async function loadNotifications() {
        if (!currentWallet) return;
        
        try {
            const response = await fetch(`${API_BASE}/wallet/notifications?wallet=${currentWallet}`);
            if (!response.ok) return;
            
            notifications = await response.json();
            updateNotificationBadge();
            
            if (notificationPanelOpen) {
                renderNotifications();
            }
        } catch (error) {
            console.error('Error loading notifications:', error);
        }
    }

    // Update notification badge
    function updateNotificationBadge() {
        const badge = document.getElementById('visionNotificationBadge');
        if (!badge) return;
        
        const unreadCount = notifications.filter(n => !n.read).length;
        
        if (unreadCount > 0) {
            badge.textContent = unreadCount;
            badge.style.display = 'flex';
        } else {
            badge.style.display = 'none';
        }
    }

    // Toggle notification panel
    window.toggleVisionNotificationPanel = function() {
        notificationPanelOpen = !notificationPanelOpen;
        const panel = document.getElementById('visionNotificationPanel');
        
        if (!panel) return;
        
        if (notificationPanelOpen) {
            panel.classList.add('show');
            renderNotifications();
        } else {
            panel.classList.remove('show');
        }
    }

    // Render notifications
    function renderNotifications() {
        const container = document.getElementById('visionNotificationPanelBody');
        if (!container) return;
        
        if (notifications.length === 0) {
            container.innerHTML = '<div style="padding: 2rem; text-align: center; color: #9aa3c7;">No notifications</div>';
            return;
        }
        
        container.innerHTML = notifications.map(n => `
            <div class="vision-notification-item ${n.read ? '' : 'unread'}" onclick="markVisionNotificationRead('${n.notification_id}', '${n.related_proposal_id || ''}')">
                <div class="vision-notification-item-title">${n.kind === 'proposal_created' ? '🔔 DING DONG BITCH!' : '📢 Notification'}</div>
                <div class="vision-notification-item-message">${escapeHtml(n.message)}</div>
                <div class="vision-notification-item-time">${getTimeAgo(n.created_at)}</div>
            </div>
        `).join('');
    }

    // Mark notification as read
    window.markVisionNotificationRead = async function(notificationId, proposalId) {
        try {
            await fetch(`${API_BASE}/wallet/notifications/${notificationId}/read`, {
                method: 'POST'
            });
            
            await loadNotifications();
            
            if (proposalId) {
                window.location.href = `/governance.html?proposal=${proposalId}`;
            }
        } catch (error) {
            console.error('Error marking notification read:', error);
        }
    }

    // Mark all notifications as read
    window.markAllVisionNotificationsRead = async function() {
        for (const notification of notifications.filter(n => !n.read)) {
            try {
                await fetch(`${API_BASE}/wallet/notifications/${notification.notification_id}/read`, {
                    method: 'POST'
                });
            } catch (error) {
                console.error('Error marking notification read:', error);
            }
        }
        await loadNotifications();
    }

    // Utility functions
    function getTimeAgo(timestamp) {
        const now = new Date();
        const time = new Date(timestamp);
        const diff = now - time;
        
        const minutes = Math.floor(diff / (1000 * 60));
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);
        
        if (days > 0) return `${days}d ago`;
        if (hours > 0) return `${hours}h ago`;
        if (minutes > 0) return `${minutes}m ago`;
        return 'Just now';
    }

    function escapeHtml(text) {
        const div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    // Initialize on page load
    function initNotifications() {
        currentWallet = initializeWallet();
        
        if (!currentWallet) {
            console.log('No wallet address found for notifications');
            return;
        }
        
        // Initial load
        loadNotifications();
        
        // Poll every 30 seconds
        notificationCheckInterval = setInterval(loadNotifications, 30000);
        
        // Close panel on outside click
        document.addEventListener('click', (e) => {
            const panel = document.getElementById('visionNotificationPanel');
            const bell = document.querySelector('.vision-notification-bell');
            
            if (notificationPanelOpen && panel && bell && 
                !panel.contains(e.target) && !bell.contains(e.target)) {
                toggleVisionNotificationPanel();
            }
        });
    }

    // Auto-initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', initNotifications);
    } else {
        initNotifications();
    }

    // Cleanup on page unload
    window.addEventListener('beforeunload', () => {
        if (notificationCheckInterval) {
            clearInterval(notificationCheckInterval);
        }
    });
})();
