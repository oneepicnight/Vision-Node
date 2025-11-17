Vision Consensus: Reorg Protection and Static Checkpoints

Overview

Vision uses PoW with a heaviest-chain fork-choice rule. To reduce the risk of long-range and deep offline forks rewriting history, Vision implements two protections:

- Max reorg depth: a node will refuse to adopt a chain that requires rolling back more than a configured number of blocks.
- Static checkpoints: a small, static list of (height, hash) pairs hard-coded into the node; forks that disagree with any checkpoint are rejected.

Configuration

- MAX_REORG_DEPTH (code constant): default 100. This is the compile-time default.
- VISION_MAX_REORG_DEPTH (env): override the max reorg depth at runtime (u64).

Static checkpoints

Static checkpoints are simple recorded (height, hash) pairs embedded in the node. They are intended as a basic long-range protection and may later be extended to signed checkpoints or a JSON configuration.

Behavior

Startup validation:
- On startup the node verifies that any checkpoint height that it already has a local block for matches the checkpoint hash. If a mismatch is detected, the node exits to avoid running on an invalid history.

Reorg handling:
- When evaluating a candidate chain that would cause a reorg the node:
  1. Finds the fork point (ancestor) between current tip and candidate tip.
  2. Computes depth = current_tip_height - fork_point_height.
  3. If depth > max_reorg (VISION_MAX_REORG_DEPTH) the candidate is rejected (logged and metric incremented).
  4. For each static checkpoint whose height lies between the ancestor and the candidate tip, the candidate must contain the same hash at that height; otherwise the fork is rejected.

Why this design

- Normal PoW short reorgs (a few blocks) are still allowed so miners and relays operate normally.
- Long-range or deep offline forks are rejected to prevent an attacker or an offline miner from rewinding finalized history.

Testing

- Test A (small reorg): verify reorg allowed when depth <= max.
- Test B (deep offline fork): verify node rejects reorg when depth > max.
- Test C (checkpoint conflict): add a checkpoint at a target height and verify that a candidate chain with a differing block hash at the checkpoint height is rejected.

Notes & future work

- Currently checkpoints are in-code. Future work: allow JSON/signed checkpoints and provide an admin endpoint to add non-signed checkpoints with operator approval.
- Metrics: reorgs, rejected reorgs, reorg length, duration are exported to Prometheus.

"Wet concrete" analogy

A public node can go down; miners keep mining. A sneaky miner who goes offline and mines privately shouldn't be able to rewrite deep history when reconnecting. At best they can affect a short window of recent blocks (normal PoW behavior). The chain becomes like wet concrete: soft on top, solid underneath.
