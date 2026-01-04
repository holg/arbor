import 'dart:convert';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:web_socket_channel/web_socket_channel.dart';

/// Represents a node in the code graph.
class GraphNode {
  final String id;
  final String name;
  final String qualifiedName;
  final String kind;
  final String file;
  final int lineStart;
  final int lineEnd;
  final String? signature;
  final double centrality;
  
  // Position in the graph (set by layout algorithm)
  double x;
  double y;
  
  // Velocity for physics simulation
  double vx = 0;
  double vy = 0;
  
  // UI state
  bool isHovered = false;
  bool isSelected = false;

  GraphNode({
    required this.id,
    required this.name,
    required this.qualifiedName,
    required this.kind,
    required this.file,
    required this.lineStart,
    required this.lineEnd,
    this.signature,
    this.centrality = 0,
    this.x = 0,
    this.y = 0,
  });

  factory GraphNode.fromJson(Map<String, dynamic> json) {
    return GraphNode(
      id: json['id'] ?? '',
      name: json['name'] ?? '',
      qualifiedName: json['qualifiedName'] ?? json['qualified_name'] ?? '',
      kind: json['kind'] ?? '',
      file: json['file'] ?? '',
      lineStart: json['lineStart'] ?? json['line_start'] ?? 0,
      lineEnd: json['lineEnd'] ?? json['line_end'] ?? 0,
      signature: json['signature'],
      centrality: (json['centrality'] ?? 0).toDouble(),
    );
  }
}

/// Represents an edge between nodes.
class GraphEdge {
  final String from;
  final String to;
  final String kind;

  GraphEdge({
    required this.from,
    required this.to,
    required this.kind,
  });
}

/// State of the graph visualization.
class GraphState {
  final List<GraphNode> nodes;
  final List<GraphEdge> edges;
  final bool isConnected;
  final bool isLoading;
  final String? error;
  final String? selectedNodeId;

  const GraphState({
    this.nodes = const [],
    this.edges = const [],
    this.isConnected = false,
    this.isLoading = false,
    this.error,
    this.selectedNodeId,
  });

  GraphState copyWith({
    List<GraphNode>? nodes,
    List<GraphEdge>? edges,
    bool? isConnected,
    bool? isLoading,
    String? error,
    String? selectedNodeId,
  }) {
    return GraphState(
      nodes: nodes ?? this.nodes,
      edges: edges ?? this.edges,
      isConnected: isConnected ?? this.isConnected,
      isLoading: isLoading ?? this.isLoading,
      error: error,
      selectedNodeId: selectedNodeId ?? this.selectedNodeId,
    );
  }
}

/// Provider for graph state management.
class GraphNotifier extends StateNotifier<GraphState> {
  WebSocketChannel? _channel;
  int _requestId = 0;

  GraphNotifier() : super(const GraphState());

  /// Connects to the Arbor server.
  Future<void> connect(String url) async {
    state = state.copyWith(isLoading: true, error: null);

    try {
      _channel = WebSocketChannel.connect(Uri.parse(url));
      
      _channel!.stream.listen(
        (message) => _handleMessage(message),
        onError: (error) {
          state = state.copyWith(
            isConnected: false,
            error: 'Connection error: $error',
          );
        },
        onDone: () {
          state = state.copyWith(isConnected: false);
        },
      );

      state = state.copyWith(isConnected: true, isLoading: false);
      
      // Fetch initial graph info
      fetchGraphInfo();
    } catch (e) {
      state = state.copyWith(
        isLoading: false,
        error: 'Failed to connect: $e',
      );
    }
  }

  /// Disconnects from the server.
  void disconnect() {
    _channel?.sink.close();
    _channel = null;
    state = state.copyWith(isConnected: false);
  }

  /// Fetches graph info.
  void fetchGraphInfo() {
    _send('graph.info', {});
  }

  /// Searches for nodes.
  void search(String query) {
    state = state.copyWith(isLoading: true);
    _send('discover', {'query': query, 'limit': 50});
  }

  /// Selects a node.
  void selectNode(String? id) {
    state = state.copyWith(selectedNodeId: id);
  }

  void _send(String method, Map<String, dynamic> params) {
    if (_channel == null) return;

    final message = jsonEncode({
      'jsonrpc': '2.0',
      'id': ++_requestId,
      'method': method,
      'params': params,
    });

    _channel!.sink.add(message);
  }

  void _handleMessage(dynamic message) {
    try {
      final data = jsonDecode(message);
      
      if (data['result'] != null) {
        final result = data['result'];
        
        // Handle discover/search results
        if (result['nodes'] != null) {
          final nodes = (result['nodes'] as List)
              .map((n) => GraphNode.fromJson(n))
              .toList();
          
          // Give nodes initial random positions
          for (var i = 0; i < nodes.length; i++) {
            nodes[i].x = 400 + (i % 10) * 80;
            nodes[i].y = 300 + (i ~/ 10) * 80;
          }
          
          state = state.copyWith(nodes: nodes, isLoading: false);
        }
      }
      
      if (data['error'] != null) {
        state = state.copyWith(
          error: data['error']['message'],
          isLoading: false,
        );
      }
    } catch (e) {
      // Ignore parse errors
    }
  }

  @override
  void dispose() {
    disconnect();
    super.dispose();
  }
}

/// Provider for graph state.
final graphProvider = StateNotifierProvider<GraphNotifier, GraphState>((ref) {
  return GraphNotifier();
});
