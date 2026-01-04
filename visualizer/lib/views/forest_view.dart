import 'dart:math';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../core/providers.dart';
import '../core/theme.dart';
import '../graph/force_layout.dart';
import '../graph/graph_painter.dart';

/// The main visualization view - the Logic Forest.
///
/// This is where the magic happens. Nodes float in space, connected
/// by glowing edges. Users can pan, zoom, and select nodes to explore
/// the code structure.
class ForestView extends ConsumerStatefulWidget {
  const ForestView({super.key});

  @override
  ConsumerState<ForestView> createState() => _ForestViewState();
}

class _ForestViewState extends ConsumerState<ForestView>
    with TickerProviderStateMixin {
  // Transform state
  Offset _offset = Offset.zero;
  double _scale = 1.0;

  // Interaction state
  String? _hoveredNodeId;
  Offset? _lastPanPosition;
  GraphNode? _draggedNode;

  // Physics simulation
  late AnimationController _simulationController;
  bool _isSimulating = false;

  // Search
  final _searchController = TextEditingController();
  bool _showSearch = false;

  @override
  void initState() {
    super.initState();
    _simulationController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 60),
    )..addListener(_runSimulation);

    // Connect to server on start
    Future.microtask(() {
      ref.read(graphProvider.notifier).connect('ws://127.0.0.1:7432');
    });
  }

  @override
  void dispose() {
    _simulationController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  void _runSimulation() {
    final state = ref.read(graphProvider);
    if (state.nodes.isEmpty) return;

    final stillMoving = ForceLayout.update(
      state.nodes,
      state.edges,
      0.016, // ~60fps
    );

    if (!stillMoving && _isSimulating) {
      _simulationController.stop();
      _isSimulating = false;
    }

    setState(() {});
  }

  void _startSimulation() {
    if (!_isSimulating) {
      _isSimulating = true;
      _simulationController.repeat();
    }
  }

  void _handleSearch(String query) {
    if (query.isEmpty) return;
    ref.read(graphProvider.notifier).search(query);
    _startSimulation();
  }

  GraphNode? _hitTest(Offset position) {
    final state = ref.read(graphProvider);
    for (final node in state.nodes.reversed) {
      final nodePos = Offset(
        node.x * _scale + _offset.dx,
        node.y * _scale + _offset.dy,
      );
      final radius = 8 + node.centrality * 16;
      if ((position - nodePos).distance < radius * _scale) {
        return node;
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(graphProvider);

    return Scaffold(
      backgroundColor: ArborTheme.background,
      body: Stack(
        children: [
          // Background gradient for depth
          _buildBackground(),

          // Graph canvas
          Listener(
            onPointerSignal: (event) {
              if (event is PointerScrollEvent) {
                setState(() {
                  final delta = event.scrollDelta.dy > 0 ? 0.9 : 1.1;
                  _scale = (_scale * delta).clamp(0.2, 3.0);
                });
              }
            },
            child: GestureDetector(
              onPanStart: (details) {
                final node = _hitTest(details.localPosition);
                if (node != null) {
                  _draggedNode = node;
                } else {
                  _lastPanPosition = details.localPosition;
                }
              },
              onPanUpdate: (details) {
                if (_draggedNode != null) {
                  setState(() {
                    _draggedNode!.x += details.delta.dx / _scale;
                    _draggedNode!.y += details.delta.dy / _scale;
                  });
                } else if (_lastPanPosition != null) {
                  setState(() {
                    _offset += details.delta;
                  });
                }
              },
              onPanEnd: (_) {
                _draggedNode = null;
                _lastPanPosition = null;
                _startSimulation();
              },
              onTapUp: (details) {
                final node = _hitTest(details.localPosition);
                ref.read(graphProvider.notifier).selectNode(node?.id);
              },
              child: MouseRegion(
                onHover: (event) {
                  final node = _hitTest(event.localPosition);
                  setState(() => _hoveredNodeId = node?.id);
                },
                onExit: (_) => setState(() => _hoveredNodeId = null),
                child: CustomPaint(
                  painter: GraphPainter(
                    nodes: state.nodes,
                    edges: state.edges,
                    selectedNodeId: state.selectedNodeId,
                    hoveredNodeId: _hoveredNodeId,
                    offset: _offset,
                    scale: _scale,
                  ),
                  size: Size.infinite,
                ),
              ),
            ),
          ),

          // Top bar
          _buildTopBar(state),

          // Node inspector (right panel)
          if (state.selectedNodeId != null) _buildInspector(state),

          // Status bar
          _buildStatusBar(state),

          // Loading overlay
          if (state.isLoading)
            Container(
              color: ArborTheme.background.withOpacity(0.7),
              child: const Center(
                child: CircularProgressIndicator(
                  color: ArborTheme.function,
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildBackground() {
    return Container(
      decoration: BoxDecoration(
        gradient: RadialGradient(
          center: Alignment.center,
          radius: 1.5,
          colors: [
            ArborTheme.surface,
            ArborTheme.background,
          ],
        ),
      ),
    );
  }

  Widget _buildTopBar(GraphState state) {
    return Positioned(
      top: 0,
      left: 0,
      right: 0,
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.topCenter,
            end: Alignment.bottomCenter,
            colors: [
              ArborTheme.background,
              ArborTheme.background.withOpacity(0),
            ],
          ),
        ),
        child: Row(
          children: [
            // Logo
            Row(
              children: [
                Container(
                  width: 32,
                  height: 32,
                  decoration: BoxDecoration(
                    color: ArborTheme.function.withOpacity(0.2),
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: const Icon(
                    Icons.account_tree,
                    color: ArborTheme.function,
                    size: 20,
                  ),
                ),
                const SizedBox(width: 12),
                Text(
                  'Arbor',
                  style: Theme.of(context).textTheme.titleLarge,
                ),
              ],
            ),
            const SizedBox(width: 32),

            // Search
            Expanded(
              child: SizedBox(
                height: 40,
                child: TextField(
                  controller: _searchController,
                  onSubmitted: _handleSearch,
                  decoration: InputDecoration(
                    hintText: 'Search for functions, classes...',
                    prefixIcon: const Icon(
                      Icons.search,
                      color: ArborTheme.textMuted,
                      size: 20,
                    ),
                    suffixIcon: IconButton(
                      icon: const Icon(
                        Icons.arrow_forward,
                        color: ArborTheme.function,
                        size: 20,
                      ),
                      onPressed: () => _handleSearch(_searchController.text),
                    ),
                    contentPadding: const EdgeInsets.symmetric(horizontal: 16),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 32),

            // Connection indicator
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              decoration: BoxDecoration(
                color: state.isConnected
                    ? ArborTheme.method.withOpacity(0.2)
                    : ArborTheme.importType.withOpacity(0.2),
                borderRadius: BorderRadius.circular(16),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 8,
                    height: 8,
                    decoration: BoxDecoration(
                      color: state.isConnected
                          ? ArborTheme.method
                          : ArborTheme.importType,
                      shape: BoxShape.circle,
                    ),
                  ),
                  const SizedBox(width: 8),
                  Text(
                    state.isConnected ? 'Connected' : 'Disconnected',
                    style: TextStyle(
                      color: state.isConnected
                          ? ArborTheme.method
                          : ArborTheme.importType,
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildInspector(GraphState state) {
    final node = state.nodes.firstWhere(
      (n) => n.id == state.selectedNodeId,
      orElse: () => state.nodes.first,
    );

    return Positioned(
      top: 80,
      right: 16,
      width: 320,
      child: Container(
        padding: const EdgeInsets.all(16),
        decoration: BoxDecoration(
          color: ArborTheme.surface,
          borderRadius: BorderRadius.circular(12),
          border: Border.all(color: ArborTheme.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 4,
                  ),
                  decoration: BoxDecoration(
                    color: ArborTheme.colorForKind(node.kind).withOpacity(0.2),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Text(
                    node.kind.toUpperCase(),
                    style: TextStyle(
                      color: ArborTheme.colorForKind(node.kind),
                      fontSize: 10,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
                const Spacer(),
                IconButton(
                  icon: const Icon(Icons.close, size: 18),
                  color: ArborTheme.textMuted,
                  onPressed: () {
                    ref.read(graphProvider.notifier).selectNode(null);
                  },
                ),
              ],
            ),
            const SizedBox(height: 12),

            // Name
            Text(
              node.name,
              style: Theme.of(context).textTheme.titleLarge,
            ),
            if (node.qualifiedName.isNotEmpty && node.qualifiedName != node.name)
              Text(
                node.qualifiedName,
                style: Theme.of(context).textTheme.bodyMedium,
              ),
            const SizedBox(height: 16),

            // File location
            Row(
              children: [
                const Icon(
                  Icons.insert_drive_file_outlined,
                  size: 14,
                  color: ArborTheme.textMuted,
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${node.file}:${node.lineStart}',
                    style: Theme.of(context).textTheme.bodyMedium,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),

            // Signature
            if (node.signature != null) ...[
              Container(
                width: double.infinity,
                padding: const EdgeInsets.all(12),
                decoration: BoxDecoration(
                  color: ArborTheme.background,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Text(
                  node.signature!,
                  style: Theme.of(context).textTheme.bodyLarge,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  Widget _buildStatusBar(GraphState state) {
    return Positioned(
      bottom: 0,
      left: 0,
      right: 0,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          gradient: LinearGradient(
            begin: Alignment.bottomCenter,
            end: Alignment.topCenter,
            colors: [
              ArborTheme.background,
              ArborTheme.background.withOpacity(0),
            ],
          ),
        ),
        child: Row(
          children: [
            Text(
              '${state.nodes.length} nodes',
              style: Theme.of(context).textTheme.labelSmall,
            ),
            const SizedBox(width: 16),
            Text(
              '${state.edges.length} edges',
              style: Theme.of(context).textTheme.labelSmall,
            ),
            const Spacer(),
            Text(
              'Zoom: ${(_scale * 100).round()}%',
              style: Theme.of(context).textTheme.labelSmall,
            ),
          ],
        ),
      ),
    );
  }
}
