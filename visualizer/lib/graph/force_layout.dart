import 'dart:math';
import '../core/providers.dart';

/// Force-directed layout algorithm.
///
/// Positions nodes using a spring simulation where:
/// - Edges act as springs pulling connected nodes together
/// - All nodes repel each other to prevent overlap
/// - The result is an organic, tree-like arrangement
class ForceLayout {
  // Simulation parameters - tuned for visual appeal
  static const double repulsion = 5000;
  static const double attraction = 0.1;
  static const double damping = 0.9;
  static const double minDistance = 80;

  /// Runs one iteration of the force simulation.
  ///
  /// Call this in an animation loop for smooth results.
  /// Returns true if the simulation is still settling.
  static bool update(List<GraphNode> nodes, List<GraphEdge> edges, double dt) {
    if (nodes.isEmpty) return false;

    // Build a map for quick lookups
    final nodeMap = {for (var n in nodes) n.id: n};

    // Calculate repulsive forces between all pairs
    for (var i = 0; i < nodes.length; i++) {
      for (var j = i + 1; j < nodes.length; j++) {
        final a = nodes[i];
        final b = nodes[j];

        var dx = b.x - a.x;
        var dy = b.y - a.y;
        var dist = sqrt(dx * dx + dy * dy);

        if (dist < 1) {
          // Nudge overlapping nodes apart randomly
          dx = (Random().nextDouble() - 0.5) * 10;
          dy = (Random().nextDouble() - 0.5) * 10;
          dist = sqrt(dx * dx + dy * dy);
        }

        // Coulomb's law: F = k / d^2
        final force = repulsion / (dist * dist);
        final fx = (dx / dist) * force;
        final fy = (dy / dist) * force;

        a.vx -= fx;
        a.vy -= fy;
        b.vx += fx;
        b.vy += fy;
      }
    }

    // Calculate attractive forces along edges
    for (final edge in edges) {
      final a = nodeMap[edge.from];
      final b = nodeMap[edge.to];
      if (a == null || b == null) continue;

      final dx = b.x - a.x;
      final dy = b.y - a.y;
      final dist = sqrt(dx * dx + dy * dy);

      if (dist > minDistance) {
        // Hooke's law: F = k * x
        final force = (dist - minDistance) * attraction;
        final fx = (dx / dist) * force;
        final fy = (dy / dist) * force;

        a.vx += fx;
        a.vy += fy;
        b.vx -= fx;
        b.vy -= fy;
      }
    }

    // Apply velocity with damping
    var totalMovement = 0.0;
    for (final node in nodes) {
      node.vx *= damping;
      node.vy *= damping;

      node.x += node.vx * dt;
      node.y += node.vy * dt;

      totalMovement += node.vx.abs() + node.vy.abs();
    }

    // Return true if still moving significantly
    return totalMovement > 0.5;
  }

  /// Centers the graph in the given bounds.
  static void centerNodes(
    List<GraphNode> nodes,
    double width,
    double height,
  ) {
    if (nodes.isEmpty) return;

    // Find bounding box
    var minX = double.infinity;
    var maxX = double.negativeInfinity;
    var minY = double.infinity;
    var maxY = double.negativeInfinity;

    for (final node in nodes) {
      minX = min(minX, node.x);
      maxX = max(maxX, node.x);
      minY = min(minY, node.y);
      maxY = max(maxY, node.y);
    }

    // Calculate offset to center
    final graphCenterX = (minX + maxX) / 2;
    final graphCenterY = (minY + maxY) / 2;
    final offsetX = width / 2 - graphCenterX;
    final offsetY = height / 2 - graphCenterY;

    for (final node in nodes) {
      node.x += offsetX;
      node.y += offsetY;
    }
  }
}
