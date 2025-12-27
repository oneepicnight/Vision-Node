/**
 * Routing Intelligence Dashboard - Live Swarm Topology & Adversarial Defense Monitor
 * 
 * Displays real-time metrics from:
 * - Phase 3.5: Latency-based routing intelligence (ring classification, cluster stats)
 * - Phase 4: Adversarial resilience & reputation system (trust levels, misbehavior tracking)
 * 
 * Features:
 * - Cluster health visualization (Inner/Middle/Outer ring distribution)
 * - Top performing peers by routing score
 * - Bad actors & watchlist (reputation-based)
 * - Network evolution timeline (recent events)
 * - Auto-refresh every 20 seconds
 */

import React, { useEffect, useState } from "react";

export type PeerRing = "Inner" | "Middle" | "Outer";

export type TrustLevel = "Trusted" | "Normal" | "Probation" | "Graylisted" | "Banned";

export interface ClusterStats {
  inner_count: number;
  middle_count: number;
  outer_count: number;
  avg_inner_latency_ms: number | null;
  avg_middle_latency_ms: number | null;
  avg_outer_latency_ms: number | null;
  guardians: number;
  anchors: number;
  total_peers: number;
  routing_health_score: number; // 0–100 (computed from reliability, latency, trust distribution)
}

export interface RoutingPeer {
  node_tag: string;
  node_id: string;
  region: string | null;
  ring: PeerRing;
  routing_score: number;
  avg_rtt_ms: number | null;
  success_rate: number; // 0–1 (route_successes / route_uses)
  trust_level: TrustLevel;
  reputation: number; // 0–100
  misbehavior_score: number; // Accumulated violations
}

export interface RoutingEvent {
  id: string;
  timestamp: string; // ISO 8601
  level: "info" | "warn" | "bad";
  message: string;
}

interface RoutingIntelligenceDashboardProps {
  initialClusterStats?: ClusterStats;
  className?: string;
}

const RoutingIntelligenceDashboard: React.FC<RoutingIntelligenceDashboardProps> = ({
  initialClusterStats,
  className = "",
}) => {
  const [clusterStats, setClusterStats] = useState<ClusterStats | null>(
    initialClusterStats ?? null
  );
  const [topPeers, setTopPeers] = useState<RoutingPeer[]>([]);
  const [badActors, setBadActors] = useState<RoutingPeer[]>([]);
  const [events, setEvents] = useState<RoutingEvent[]>([]);
  const [loading, setLoading] = useState(true);
  const [lastUpdated, setLastUpdated] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const fetchData = async () => {
      try {
        setLoading(true);

        // API endpoints (adjust paths to match your backend routing)
        const [clusterRes, peersRes, eventsRes] = await Promise.all([
          fetch("/api/p2p/routing/cluster_stats"),
          fetch("/api/p2p/routing/top_peers?limit=20"),
          fetch("/api/p2p/routing/events?limit=50"),
        ]);

        if (cancelled) return;

        const clusterJson = await clusterRes.json();
        const peersJson: RoutingPeer[] = await peersRes.json();
        const eventsJson: RoutingEvent[] = await eventsRes.json();

        setClusterStats(clusterJson);
        
        // Sort top peers by routing score descending
        const sortedPeers = [...peersJson].sort(
          (a, b) => b.routing_score - a.routing_score
        );
        setTopPeers(sortedPeers.slice(0, 20));

        // Bad actors: low reputation OR non-Normal trust level OR high misbehavior
        const bad = sortedPeers.filter(
          (p) =>
            p.trust_level !== "Normal" && p.trust_level !== "Trusted" ||
            p.reputation < 50 ||
            p.misbehavior_score > 10
        );
        setBadActors(bad.slice(0, 20));

        setEvents(eventsJson);
        setLastUpdated(new Date().toLocaleTimeString());
      } catch (err) {
        console.error("[RoutingDashboard] Failed to fetch routing stats", err);
        
        // Fallback to mock data for development
        if (!cancelled) {
          setClusterStats({
            inner_count: 12,
            middle_count: 6,
            outer_count: 4,
            avg_inner_latency_ms: 45,
            avg_middle_latency_ms: 120,
            avg_outer_latency_ms: 250,
            guardians: 3,
            anchors: 2,
            total_peers: 22,
            routing_health_score: 85,
          });
          setTopPeers([]);
          setBadActors([]);
          setEvents([
            {
              id: "1",
              timestamp: new Date().toISOString(),
              level: "info",
              message: "Cluster balance maintained: 12 inner, 6 middle, 4 outer",
            },
          ]);
          setLastUpdated(new Date().toLocaleTimeString());
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    fetchData();

    // Refresh every 20 seconds
    const id = setInterval(fetchData, 20_000);

    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const renderHealthBadge = (score: number | null | undefined) => {
    if (score == null) {
      return <span className="ri-badge ri-badge--neutral">Unknown</span>;
    }
    if (score >= 80) {
      return <span className="ri-badge ri-badge--good">Excellent</span>;
    }
    if (score >= 60) {
      return <span className="ri-badge ri-badge--ok">Healthy</span>;
    }
    if (score >= 40) {
      return <span className="ri-badge ri-badge--warn">Degraded</span>;
    }
    return <span className="ri-badge ri-badge--bad">Critical</span>;
  };

  const renderTrustPill = (trust: TrustLevel) => {
    const map: Record<TrustLevel, string> = {
      Trusted: "ri-pill ri-pill--trusted",
      Normal: "ri-pill ri-pill--normal",
      Probation: "ri-pill ri-pill--probation",
      Graylisted: "ri-pill ri-pill--gray",
      Banned: "ri-pill ri-pill--banned",
    };
    return <span className={map[trust]}>{trust}</span>;
  };

  const renderRingChip = (ring: PeerRing) => {
    const cls =
      ring === "Inner"
        ? "ri-chip ri-chip--inner"
        : ring === "Middle"
        ? "ri-chip ri-chip--middle"
        : "ri-chip ri-chip--outer";
    return <span className={cls}>{ring}</span>;
  };

  const cluster = clusterStats;

  return (
    <div className={`cc-panel routing-intel-panel ${className}`}>
      <div className="cc-panel__header">
        <div>
          <h2 className="cc-panel__title">Routing Intelligence Dashboard</h2>
          <p className="cc-panel__subtitle">
            Live view of swarm topology, peer quality, and adversarial defense.
          </p>
        </div>
        <div className="cc-panel__meta">
          {loading && <span className="ri-meta ri-meta--loading">Syncing…</span>}
          {!loading && lastUpdated && (
            <span className="ri-meta">Updated {lastUpdated}</span>
          )}
        </div>
      </div>

      {/* TOP: Cluster Health */}
      <div className="ri-grid ri-grid--top">
        <div className="ri-card ri-card--health">
          <div className="ri-card__header">
            <h3>Cluster Health</h3>
          </div>
          {cluster ? (
            <div className="ri-health-grid">
              <div className="ri-health-block">
                <span className="ri-label">Inner Ring</span>
                <div className="ri-health-metric">
                  <span className="ri-value">
                    {cluster.inner_count}/{cluster.total_peers}
                  </span>
                  <span className="ri-sub">
                    {cluster.avg_inner_latency_ms != null
                      ? `${cluster.avg_inner_latency_ms} ms`
                      : "—"}
                  </span>
                </div>
              </div>
              <div className="ri-health-block">
                <span className="ri-label">Middle Ring</span>
                <div className="ri-health-metric">
                  <span className="ri-value">
                    {cluster.middle_count}/{cluster.total_peers}
                  </span>
                  <span className="ri-sub">
                    {cluster.avg_middle_latency_ms != null
                      ? `${cluster.avg_middle_latency_ms} ms`
                      : "—"}
                  </span>
                </div>
              </div>
              <div className="ri-health-block">
                <span className="ri-label">Outer Ring</span>
                <div className="ri-health-metric">
                  <span className="ri-value">
                    {cluster.outer_count}/{cluster.total_peers}
                  </span>
                  <span className="ri-sub">
                    {cluster.avg_outer_latency_ms != null
                      ? `${cluster.avg_outer_latency_ms} ms`
                      : "—"}
                  </span>
                </div>
              </div>
              <div className="ri-health-block ri-health-block--wide">
                <span className="ri-label">Guardians & Anchors</span>
                <div className="ri-health-inline">
                  <span className="ri-chip ri-chip--guardian">
                    Guardians: {cluster.guardians}
                  </span>
                  <span className="ri-chip ri-chip--anchor">
                    Anchors: {cluster.anchors}
                  </span>
                </div>
                <div className="ri-health-status">
                  <span className="ri-label">Routing Health</span>
                  <div className="ri-health-status__row">
                    <span className="ri-value">
                      {Math.round(cluster.routing_health_score)}%
                    </span>
                    {renderHealthBadge(cluster.routing_health_score)}
                  </div>
                </div>
              </div>
            </div>
          ) : (
            <div className="ri-empty">No cluster stats available yet.</div>
          )}
        </div>
      </div>

      {/* MIDDLE: Top Peers + Bad Actors */}
      <div className="ri-grid ri-grid--middle">
        <div className="ri-card ri-card--top-peers">
          <div className="ri-card__header">
            <h3>Top Peers</h3>
            <span className="ri-card__hint">
              Ranked by routing score, latency, and reliability.
            </span>
          </div>
          <div className="ri-table-wrapper">
            <table className="ri-table">
              <thead>
                <tr>
                  <th>Node</th>
                  <th>Ring</th>
                  <th>Region</th>
                  <th>Latency</th>
                  <th>Success</th>
                  <th>Score</th>
                  <th>Trust</th>
                </tr>
              </thead>
              <tbody>
                {topPeers.length === 0 && (
                  <tr>
                    <td colSpan={7} className="ri-empty">
                      No peers ranked yet.
                    </td>
                  </tr>
                )}
                {topPeers.map((peer) => (
                  <tr key={peer.node_id}>
                    <td>
                      <div className="ri-node">
                        <span className="ri-node__tag">
                          {peer.node_tag || "Unknown"}
                        </span>
                        <span className="ri-node__id">
                          {peer.node_id.slice(0, 10)}…
                        </span>
                      </div>
                    </td>
                    <td>{renderRingChip(peer.ring)}</td>
                    <td>{peer.region ?? "—"}</td>
                    <td>
                      {peer.avg_rtt_ms != null
                        ? `${peer.avg_rtt_ms} ms`
                        : "—"}
                    </td>
                    <td>{Math.round(peer.success_rate * 100)}%</td>
                    <td>{peer.routing_score.toFixed(1)}</td>
                    <td>{renderTrustPill(peer.trust_level)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        <div className="ri-card ri-card--bad-actors">
          <div className="ri-card__header">
            <h3>Bad Actors & Watchlist</h3>
            <span className="ri-card__hint">
              Peers with low reputation or non-normal trust state.
            </span>
          </div>
          <div className="ri-table-wrapper">
            <table className="ri-table ri-table--compact">
              <thead>
                <tr>
                  <th>Node</th>
                  <th>Trust</th>
                  <th>Reputation</th>
                  <th>Latency</th>
                  <th>Score</th>
                </tr>
              </thead>
              <tbody>
                {badActors.length === 0 && (
                  <tr>
                    <td colSpan={5} className="ri-empty">
                      No bad actors detected. The swarm is calm… for now.
                    </td>
                  </tr>
                )}
                {badActors.map((peer) => (
                  <tr
                    key={peer.node_id}
                    className={
                      peer.trust_level === "Banned"
                        ? "ri-row--banned"
                        : peer.trust_level === "Graylisted"
                        ? "ri-row--gray"
                        : peer.trust_level === "Probation"
                        ? "ri-row--probation"
                        : ""
                    }
                  >
                    <td>
                      <div className="ri-node">
                        <span className="ri-node__tag">
                          {peer.node_tag || "Unknown"}
                        </span>
                        <span className="ri-node__id">
                          {peer.node_id.slice(0, 10)}…
                        </span>
                      </div>
                    </td>
                    <td>{renderTrustPill(peer.trust_level)}</td>
                    <td>{Math.round(peer.reputation)} / 100</td>
                    <td>
                      {peer.avg_rtt_ms != null
                        ? `${peer.avg_rtt_ms} ms`
                        : "—"}
                    </td>
                    <td>{peer.routing_score.toFixed(1)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="ri-card__footer">
            <span className="ri-footnote">
              Graylisted/Banned peers still decay back toward Normal over time if
              they behave.
            </span>
          </div>
        </div>
      </div>

      {/* BOTTOM: Evolution Over Time */}
      <div className="ri-grid ri-grid--bottom">
        <div className="ri-card ri-card--evolution">
          <div className="ri-card__header">
            <h3>Network Evolution Timeline</h3>
            <span className="ri-card__hint">
              Most recent routing-related events and adjustments.
            </span>
          </div>
          <div className="ri-events">
            {events.length === 0 && (
              <div className="ri-empty">
                No routing events yet. The constellation is warming up.
              </div>
            )}
            {events.map((ev) => (
              <div
                key={ev.id}
                className={`ri-event ri-event--${ev.level}`}
              >
                <div className="ri-event__meta">
                  <span className="ri-event__time">
                    {new Date(ev.timestamp).toLocaleTimeString()}
                  </span>
                  <span className="ri-event__level">{ev.level}</span>
                </div>
                <div className="ri-event__msg">{ev.message}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Optional: little trend summary */}
        <div className="ri-card ri-card--trend">
          <div className="ri-card__header">
            <h3>Trend Snapshot</h3>
          </div>
          {cluster ? (
            <div className="ri-trend-grid">
              <div className="ri-trend-item">
                <span className="ri-label">Total Peers</span>
                <span className="ri-value">{cluster.total_peers}</span>
              </div>
              <div className="ri-trend-item">
                <span className="ri-label">Inner Ring Share</span>
                <span className="ri-value">
                  {cluster.total_peers > 0
                    ? Math.round(
                        (cluster.inner_count / cluster.total_peers) * 100
                      )
                    : 0}
                  %
                </span>
              </div>
              <div className="ri-trend-item">
                <span className="ri-label">Routing Health</span>
                <span className="ri-value">
                  {Math.round(cluster.routing_health_score)}%
                </span>
              </div>
              <div className="ri-trend-footnote">
                Routing intelligence adapts as peers join, leave, and misbehave.
              </div>
            </div>
          ) : (
            <div className="ri-empty">
              Waiting for initial swarm stats…
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default RoutingIntelligenceDashboard;
