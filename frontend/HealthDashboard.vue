<!-- Advanced Network Health Monitoring Dashboard -->
<!-- Add to Vision Guard UI -->

<div class="health-dashboard">
  <div class="toolbar">
    <a href="/app#/command-center" class="link">← Back to Command Center</a>
  </div>
  <!-- Overall Health Score -->
  <div class="health-score-card">
    <h2>Network Health</h2>
    <div class="score-circle" :class="healthStatusClass">
      <span class="score-value">{{ healthScore.overall }}%</span>
      <span class="score-label">{{ healthStatus }}</span>
    </div>
    <div class="score-breakdown">
      <div class="score-item">
        <span class="label">Connectivity</span>
        <div class="progress-bar">
          <div class="progress" :style="{width: healthScore.connectivity + '%'}"></div>
        </div>
        <span class="value">{{ healthScore.connectivity }}%</span>
      </div>
      <div class="score-item">
        <span class="label">Performance</span>
        <div class="progress-bar">
          <div class="progress" :style="{width: healthScore.performance + '%'}"></div>
        </div>
        <span class="value">{{ healthScore.performance }}%</span>
      </div>
      <div class="score-item">
        <span class="label">Stability</span>
        <div class="progress-bar">
          <div class="progress" :style="{width: healthScore.stability + '%'}"></div>
        </div>
        <span class="value">{{ healthScore.stability }}%</span>
      </div>
      <div class="score-item">
        <span class="label">Reputation</span>
        <div class="progress-bar">
          <div class="progress" :style="{width: healthScore.reputation + '%'}"></div>
        </div>
        <span class="value">{{ healthScore.reputation }}%</span>
      </div>
    </div>
  </div>

  <!-- Active Alerts -->
  <div class="alerts-panel">
    <h3>
      <span class="icon">⚠️</span>
      Active Alerts
      <span class="badge" v-if="alerts.length">{{ alerts.length }}</span>
    </h3>
    <div v-if="alerts.length === 0" class="no-alerts">
      ✅ All systems operational
    </div>
    <div v-else class="alert-list">
      <div v-for="alert in alerts" :key="alert.timestamp" 
           class="alert-item" :class="'severity-' + alert.severity.toLowerCase()">
        <div class="alert-header">
          <span class="severity-badge">{{ alert.severity }}</span>
          <span class="timestamp">{{ formatTime(alert.timestamp) }}</span>
        </div>
        <div class="alert-message">{{ alert.message }}</div>
        <div class="alert-metric">
          <strong>{{ alert.metric }}:</strong> {{ alert.current_value.toFixed(1) }} 
          (threshold: {{ alert.threshold }})
        </div>
        <div class="alert-recommendation">
          💡 {{ alert.recommendation }}
        </div>
      </div>
    </div>
  </div>

  <!-- Peer Distribution Chart -->
  <div class="peer-distribution-card">
    <h3>Peer Distribution</h3>
    <div class="chart-container">
      <canvas ref="peerChart"></canvas>
    </div>
    <div class="legend">
      <div class="legend-item hot">
        <span class="dot"></span>
        <span>Hot: {{ peerStats.hot_peers }}</span>
      </div>
      <div class="legend-item warm">
        <span class="dot"></span>
        <span>Warm: {{ peerStats.warm_peers }}</span>
      </div>
      <div class="legend-item cold">
        <span class="dot"></span>
        <span>Cold: {{ peerStats.cold_peers }}</span>
      </div>
    </div>
  </div>

  <!-- Network Latency Graph -->
  <div class="latency-graph-card">
    <h3>Network Latency (Last Hour)</h3>
    <div class="chart-container">
      <canvas ref="latencyChart"></canvas>
    </div>
    <div class="stats-row">
      <div class="stat">
        <span class="label">Average</span>
        <span class="value">{{ avgLatency }}ms</span>
      </div>
      <div class="stat">
        <span class="label">Maximum</span>
        <span class="value">{{ maxLatency }}ms</span>
      </div>
    </div>
  </div>

  <!-- Connection Stability -->
  <div class="stability-card">
    <h3>Connection Stability</h3>
    <div class="stability-metrics">
      <div class="metric">
        <span class="icon">🔗</span>
        <span class="label">Connected Peers</span>
        <span class="value">{{ peerStats.connected_peers }}</span>
      </div>
      <div class="metric">
        <span class="icon">✅</span>
        <span class="label">Success Rate</span>
        <span class="value">{{ successRate }}%</span>
      </div>
      <div class="metric">
        <span class="icon">🔄</span>
        <span class="label">Reconnections</span>
        <span class="value">{{ reconnectionCount }}</span>
      </div>
    </div>
  </div>

  <!-- Sync Status -->
  <div class="sync-status-card">
    <h3>Blockchain Sync</h3>
    <div class="sync-info">
      <div class="sync-height">
        <span class="label">Local Height</span>
        <span class="value">{{ syncStatus.sync_height }}</span>
      </div>
      <div class="sync-height">
        <span class="label">Network Height</span>
        <span class="value">{{ syncStatus.network_estimated_height }}</span>
      </div>
      <div class="sync-progress">
        <div class="progress-bar">
          <div class="progress" :style="{width: syncProgress + '%'}"></div>
        </div>
        <span class="progress-text">{{ syncProgress }}% synced</span>
      </div>
      <div class="sync-status" :class="{syncing: syncStatus.is_syncing}">
        {{ syncStatus.is_syncing ? '🔄 Syncing...' : '✅ Fully Synced' }}
      </div>
    </div>
  </div>

  <!-- Vault & Treasury -->
  <div class="vault-card">
    <h3>Vault & Treasury</h3>
    <div class="vault-grid">
      <div class="vault-metric">
        <span class="label">Vault Balance</span>
        <span class="value">{{ formatAmount(vaultEpoch.vault_balance) }}</span>
      </div>
      <div class="vault-metric">
        <span class="label">Fund Balance</span>
        <span class="value">{{ formatAmount(vaultEpoch.fund_balance) }}</span>
      </div>
      <div class="vault-metric">
        <span class="label">Treasury Balance</span>
        <span class="value">{{ formatAmount(vaultEpoch.treasury_balance) }}</span>
      </div>
    </div>
    <div class="epoch-row">
      <div class="epoch-item">
        <span class="label">Epoch</span>
        <span class="value">#{{ vaultEpoch.epoch_index }}</span>
      </div>
      <div class="epoch-item">
        <span class="label">Last Payout</span>
        <span class="value">{{ formatMs(vaultEpoch.last_payout_at_ms) }}</span>
      </div>
      <div class="epoch-item">
        <span class="label">Total Stake Weight</span>
        <span class="value">{{ vaultEpoch.total_weight }}</span>
      </div>
      <div class="epoch-item">
        <span class="label">Status</span>
        <span class="badge" :class="vaultEpoch.due ? 'due' : 'ok'">{{ vaultEpoch.due ? '🟡 Payout Due' : '🟢 Up to Date' }}</span>
      </div>
    </div>
  </div>
</div>

<script>
export default {
  name: 'HealthDashboard',
  data() {
    return {
      healthScore: {
        overall: 0,
        connectivity: 0,
        performance: 0,
        stability: 0,
        reputation: 0
      },
      healthStatus: 'Loading...',
      alerts: [],
      peerStats: {
        connected_peers: 0,
        hot_peers: 0,
        warm_peers: 0,
        cold_peers: 0
      },
      syncStatus: {
        sync_height: 0,
        network_estimated_height: 0,
        is_syncing: false
      },
      avgLatency: 0,
      maxLatency: 0,
      successRate: 0,
      reconnectionCount: 0,
      latencyHistory: [],
      peerChart: null,
      latencyChart: null,
      refreshInterval: null,
      vaultEpoch: {
        epoch_index: 0,
        last_payout_height: 0,
        last_payout_at_ms: 0,
        vault_balance: '0',
        fund_balance: '0',
        treasury_balance: '0',
        total_weight: '0',
        due: false,
        height: 0
      }
    }
  },
  computed: {
    healthStatusClass() {
      const score = this.healthScore.overall;
      if (score >= 80) return 'healthy';
      if (score >= 60) return 'degraded';
      if (score >= 40) return 'unhealthy';
      return 'critical';
    },
    syncProgress() {
      if (this.syncStatus.network_estimated_height === 0) return 100;
      return Math.min(100, (this.syncStatus.sync_height / this.syncStatus.network_estimated_height * 100).toFixed(1));
    }
  },
  mounted() {
    this.fetchHealthData();
    this.initCharts();
    
    // Refresh every 30 seconds
    this.refreshInterval = setInterval(() => {
      this.fetchHealthData();
    }, 30000);
  },
  beforeUnmount() {
    if (this.refreshInterval) {
      clearInterval(this.refreshInterval);
    }
  },
  methods: {
    async fetchHealthData() {
      try {
        // Fetch health score and alerts
        const healthRes = await fetch('/p2p/health');
        const healthData = await healthRes.json();
        this.healthScore = healthData.score;
        this.healthStatus = healthData.status;
        this.alerts = healthData.alerts;
        
        // Fetch peer statistics
        const peersRes = await fetch('/p2p/peers');
        const peersData = await peersRes.json();
        this.peerStats = {
          connected_peers: peersData.connected_peers,
          hot_peers: peersData.hot_peers,
          warm_peers: peersData.warm_peers,
          cold_peers: peersData.cold_peers
        };
        
        // Fetch constellation status
        const statusRes = await fetch('/constellation/status');
        const statusData = await statusRes.json();
        this.syncStatus = {
          sync_height: statusData.sync_height,
          network_estimated_height: statusData.network_estimated_height,
          is_syncing: statusData.is_syncing
        };
        this.avgLatency = statusData.avg_peer_latency_ms || 0;
        this.maxLatency = statusData.max_peer_latency_ms || 0;

        // Fetch vault epoch status
        const epochRes = await fetch('/vault/epoch');
        if (epochRes.ok) {
          const epochData = await epochRes.json();
          this.vaultEpoch = {
            epoch_index: epochData.epoch_index || 0,
            last_payout_height: epochData.last_payout_height || 0,
            last_payout_at_ms: epochData.last_payout_at_ms || 0,
            vault_balance: epochData.vault_balance || '0',
            fund_balance: epochData.fund_balance || '0',
            treasury_balance: epochData.treasury_balance || '0',
            total_weight: epochData.total_weight || '0',
            due: !!epochData.due,
            height: epochData.height || 0
          };
        }
        
        // Update charts
        this.updateCharts();
        this.updateLatencyHistory();
        
      } catch (error) {
        console.error('Failed to fetch health data:', error);
      }
    },
    
    initCharts() {
      // Initialize peer distribution pie chart
      const peerCtx = this.$refs.peerChart.getContext('2d');
      this.peerChart = new Chart(peerCtx, {
        type: 'doughnut',
        data: {
          labels: ['Hot', 'Warm', 'Cold'],
          datasets: [{
            data: [0, 0, 0],
            backgroundColor: ['#ff4444', '#ffaa44', '#4444ff']
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false
        }
      });
      
      // Initialize latency line chart
      const latencyCtx = this.$refs.latencyChart.getContext('2d');
      this.latencyChart = new Chart(latencyCtx, {
        type: 'line',
        data: {
          labels: [],
          datasets: [{
            label: 'Average Latency (ms)',
            data: [],
            borderColor: '#4444ff',
            backgroundColor: 'rgba(68, 68, 255, 0.1)',
            tension: 0.4
          }]
        },
        options: {
          responsive: true,
          maintainAspectRatio: false,
          scales: {
            y: {
              beginAtZero: true
            }
          }
        }
      });
    },
    
    updateCharts() {
      // Update peer distribution chart
      if (this.peerChart) {
        this.peerChart.data.datasets[0].data = [
          this.peerStats.hot_peers,
          this.peerStats.warm_peers,
          this.peerStats.cold_peers
        ];
        this.peerChart.update();
      }
    },
    
    updateLatencyHistory() {
      // Add current latency to history
      const now = new Date();
      this.latencyHistory.push({
        time: now.toLocaleTimeString(),
        latency: this.avgLatency
      });
      
      // Keep only last 60 data points (30 minutes at 30s intervals)
      if (this.latencyHistory.length > 60) {
        this.latencyHistory.shift();
      }
      
      // Update latency chart
      if (this.latencyChart) {
        this.latencyChart.data.labels = this.latencyHistory.map(d => d.time);
        this.latencyChart.data.datasets[0].data = this.latencyHistory.map(d => d.latency);
        this.latencyChart.update();
      }
    },
    
    formatTime(timestamp) {
      const date = new Date(timestamp * 1000);
      const now = new Date();
      const diff = Math.floor((now - date) / 1000);
      
      if (diff < 60) return `${diff}s ago`;
      if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
      if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
      return date.toLocaleDateString();
    },
    formatMs(ms) {
      if (!ms || ms <= 0) return '—';
      const date = new Date(ms);
      return date.toLocaleString();
    },
    formatAmount(s) {
      const n = typeof s === 'string' ? Number(s) : s;
      if (!Number.isFinite(n)) return s;
      return n.toLocaleString('en-US');
    }
  }
}
</script>

<style scoped>
.health-dashboard {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 20px;
  padding: 20px;
}

.health-score-card,
.alerts-panel,
.peer-distribution-card,
.latency-graph-card,
.stability-card,
.sync-status-card {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}

.vault-card {
  background: white;
  border-radius: 8px;
  padding: 20px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}

.vault-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: 12px;
  margin-bottom: 12px;
}

.vault-metric {
  display: flex;
  align-items: center;
  gap: 10px;
}

.vault-metric .label {
  font-size: 13px;
  color: #666;
}

.vault-metric .value {
  margin-left: auto;
  font-weight: bold;
}

.epoch-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  align-items: center;
}

.epoch-item .label {
  font-size: 13px;
  color: #666;
}

.epoch-item .value {
  font-weight: bold;
}

.epoch-item .badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 14px;
  font-size: 12px;
  font-weight: 600;
}

.epoch-item .badge.ok {
  background: #e8f5e9;
  color: #2e7d32;
}

.epoch-item .badge.due {
  background: #fff8e1;
  color: #b26a00;
}

.score-circle {
  width: 150px;
  height: 150px;
  border-radius: 50%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  margin: 20px auto;
  border: 8px solid;
}

.score-circle.healthy { border-color: #4caf50; }
.score-circle.degraded { border-color: #ff9800; }
.score-circle.unhealthy { border-color: #ff5722; }
.score-circle.critical { border-color: #f44336; }

.score-value {
  font-size: 32px;
  font-weight: bold;
}

.score-label {
  font-size: 14px;
  text-transform: uppercase;
  margin-top: 5px;
}

.score-breakdown {
  margin-top: 20px;
}

.score-item {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.progress-bar {
  flex: 1;
  height: 8px;
  background: #e0e0e0;
  border-radius: 4px;
  overflow: hidden;
}

.progress {
  height: 100%;
  background: linear-gradient(90deg, #4444ff, #44ff44);
  transition: width 0.3s ease;
}

.alert-item {
  padding: 12px;
  margin-bottom: 10px;
  border-radius: 4px;
  border-left: 4px solid;
}

.alert-item.severity-critical {
  background: #ffebee;
  border-left-color: #f44336;
}

.alert-item.severity-warning {
  background: #fff3e0;
  border-left-color: #ff9800;
}

.alert-item.severity-info {
  background: #e3f2fd;
  border-left-color: #2196f3;
}

.severity-badge {
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 11px;
  font-weight: bold;
  text-transform: uppercase;
}

.chart-container {
  height: 200px;
  margin: 20px 0;
}

.legend {
  display: flex;
  gap: 20px;
  justify-content: center;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 8px;
}

.legend-item .dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.legend-item.hot .dot { background: #ff4444; }
.legend-item.warm .dot { background: #ffaa44; }
.legend-item.cold .dot { background: #4444ff; }

.stability-metrics,
.sync-info {
  display: flex;
  flex-direction: column;
  gap: 15px;
}

.metric {
  display: flex;
  align-items: center;
  gap: 10px;
}

.metric .icon {
  font-size: 24px;
}

.metric .value {
  margin-left: auto;
  font-weight: bold;
  font-size: 18px;
}
</style>
<style scoped>
.toolbar {
  display: flex;
  justify-content: flex-end;
}
.toolbar .link {
  display: inline-block;
  padding: 8px 12px;
  border-radius: 6px;
  background: #f5f5f5;
  color: #333;
  text-decoration: none;
}
.toolbar .link:hover {
  background: #eee;
}
</style>
