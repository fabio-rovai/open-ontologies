# Open Ontologies Studio — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a cross-platform visual ontology editor (Flutter + Dart) that connects to the Open Ontologies Rust engine over HTTP+SSE, with a graph-first canvas, AI activity feed, and human-in-the-loop controls.

**Architecture:** Flutter app as a thin client, Open Ontologies engine as the backend via `serve-http`. Start with HTTP+JSON (the MCP-over-HTTP endpoint already exists). Add SSE for real-time push. Graph layout in pure Dart initially, Rust FFI later. Separate repo `open-ontologies-studio`.

**Tech Stack:** Flutter 3.x, Dart, Riverpod (state), CustomPainter (graph canvas), HTTP+SSE (bridge to engine), GitHub Actions (CI/CD)

**Design doc:** `docs/plans/2026-03-14-open-ontologies-studio-design.md`

---

## Phase 1: Scaffold + Engine Connection (get data flowing)

### Task 1: Create Flutter project and repo structure

**Files:**
- Create: `open-ontologies-studio/` (new repo at `/Users/fabio/projects/open-ontologies-studio/`)

**Step 1: Create the Flutter project**

```bash
cd /Users/fabio/projects
flutter create --org com.openontologies --project-name open_ontologies_studio open-ontologies-studio
cd open-ontologies-studio
```

**Step 2: Clean up default boilerplate**

Delete the default counter app code from `lib/main.dart`. Replace with a minimal app shell:

```dart
import 'package:flutter/material.dart';

void main() {
  runApp(const OpenOntologiesStudio());
}

class OpenOntologiesStudio extends StatelessWidget {
  const OpenOntologiesStudio({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Ontologies Studio',
      theme: ThemeData.dark(useMaterial3: true),
      home: const Scaffold(
        body: Center(child: Text('Open Ontologies Studio')),
      ),
    );
  }
}
```

**Step 3: Set up directory structure**

```bash
mkdir -p lib/features/graph
mkdir -p lib/features/ai_activity
mkdir -p lib/features/validation
mkdir -p lib/features/lineage
mkdir -p lib/features/sparql
mkdir -p lib/features/inspector
mkdir -p lib/features/command_palette
mkdir -p lib/services
mkdir -p lib/models
```

**Step 4: Add core dependencies to `pubspec.yaml`**

```yaml
dependencies:
  flutter:
    sdk: flutter
  flutter_riverpod: ^2.6.1
  http: ^1.3.0
  json_annotation: ^4.9.0

dev_dependencies:
  flutter_test:
    sdk: flutter
  build_runner: ^2.4.0
  json_serializable: ^6.9.0
```

**Step 5: Verify it runs**

```bash
flutter run -d chrome
```

Expected: dark-themed app with "Open Ontologies Studio" text centered.

**Step 6: Initialize git and commit**

```bash
cd /Users/fabio/projects/open-ontologies-studio
git init
git add .
git commit -m "feat: scaffold Flutter project with directory structure"
```

---

### Task 2: Data models for graph nodes and edges

**Files:**
- Create: `lib/models/graph_node.dart`
- Create: `lib/models/graph_edge.dart`
- Create: `lib/models/ontology_stats.dart`
- Test: `test/models/graph_node_test.dart`

**Step 1: Write the failing test**

```dart
// test/models/graph_node_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:open_ontologies_studio/models/graph_node.dart';

void main() {
  test('GraphNode.fromSparqlBinding parses a SPARQL result row', () {
    final binding = {
      'class': {'type': 'uri', 'value': 'http://example.org/Vehicle'},
      'label': {'type': 'literal', 'value': 'Vehicle'},
      'comment': {'type': 'literal', 'value': 'A mode of transport'},
    };
    final node = GraphNode.fromSparqlBinding(binding);
    expect(node.iri, 'http://example.org/Vehicle');
    expect(node.label, 'Vehicle');
    expect(node.comment, 'A mode of transport');
    expect(node.nodeType, NodeType.owlClass);
  });

  test('GraphNode.fromSparqlBinding handles missing optional fields', () {
    final binding = {
      'class': {'type': 'uri', 'value': 'http://example.org/Car'},
    };
    final node = GraphNode.fromSparqlBinding(binding);
    expect(node.iri, 'http://example.org/Car');
    expect(node.label, 'Car');
    expect(node.comment, isNull);
  });
}
```

**Step 2: Run test to verify it fails**

```bash
flutter test test/models/graph_node_test.dart
```

Expected: FAIL — `graph_node.dart` doesn't exist.

**Step 3: Write the models**

```dart
// lib/models/graph_node.dart
enum NodeType { owlClass, individual, datatypeProperty, objectProperty }

class GraphNode {
  final String iri;
  final String label;
  final String? comment;
  final NodeType nodeType;
  double x;
  double y;

  GraphNode({
    required this.iri,
    required this.label,
    this.comment,
    this.nodeType = NodeType.owlClass,
    this.x = 0,
    this.y = 0,
  });

  /// Parse from a SPARQL SELECT result binding.
  /// Expects keys: class (uri), label (optional), comment (optional).
  factory GraphNode.fromSparqlBinding(Map<String, dynamic> binding) {
    final iri = binding['class']['value'] as String;
    final label = binding['label']?['value'] as String? ?? _localName(iri);
    final comment = binding['comment']?['value'] as String?;
    return GraphNode(iri: iri, label: label, comment: comment);
  }

  static String _localName(String iri) {
    final hash = iri.lastIndexOf('#');
    if (hash >= 0) return iri.substring(hash + 1);
    final slash = iri.lastIndexOf('/');
    if (slash >= 0) return iri.substring(slash + 1);
    return iri;
  }
}
```

```dart
// lib/models/graph_edge.dart
class GraphEdge {
  final String sourceIri;
  final String targetIri;
  final String predicate;

  const GraphEdge({
    required this.sourceIri,
    required this.targetIri,
    required this.predicate,
  });

  factory GraphEdge.fromSparqlBinding(Map<String, dynamic> binding) {
    return GraphEdge(
      sourceIri: binding['subject']['value'] as String,
      targetIri: binding['object']['value'] as String,
      predicate: binding['predicate']['value'] as String,
    );
  }
}
```

```dart
// lib/models/ontology_stats.dart
class OntologyStats {
  final int classes;
  final int properties;
  final int individuals;
  final int triples;

  const OntologyStats({
    required this.classes,
    required this.properties,
    required this.individuals,
    required this.triples,
  });

  factory OntologyStats.fromJson(Map<String, dynamic> json) {
    return OntologyStats(
      classes: json['classes'] as int? ?? 0,
      properties: json['properties'] as int? ?? 0,
      individuals: json['individuals'] as int? ?? 0,
      triples: json['triples'] as int? ?? 0,
    );
  }
}
```

**Step 4: Run test to verify it passes**

```bash
flutter test test/models/graph_node_test.dart
```

Expected: PASS

**Step 5: Commit**

```bash
git add lib/models/ test/models/
git commit -m "feat: add GraphNode, GraphEdge, OntologyStats models"
```

---

### Task 3: Engine client service (HTTP bridge to Open Ontologies)

**Files:**
- Create: `lib/services/engine_client.dart`
- Test: `test/services/engine_client_test.dart`

The engine exposes MCP-over-HTTP at `/mcp`. However, MCP's JSON-RPC protocol is complex for a UI client. The simpler approach: add a thin REST+JSON API alongside the MCP endpoint in the engine. But to avoid modifying the engine initially, we'll call the MCP endpoint directly using JSON-RPC.

**Step 1: Write the failing test**

```dart
// test/services/engine_client_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:open_ontologies_studio/services/engine_client.dart';

void main() {
  test('EngineClient builds correct MCP tool call JSON-RPC request', () {
    final client = EngineClient(baseUrl: 'http://localhost:3000');
    final body = client.buildToolCallRequest('onto_stats', {});
    expect(body['jsonrpc'], '2.0');
    expect(body['method'], 'tools/call');
    expect(body['params']['name'], 'onto_stats');
  });
}
```

**Step 2: Run test to verify it fails**

```bash
flutter test test/services/engine_client_test.dart
```

Expected: FAIL

**Step 3: Write the engine client**

```dart
// lib/services/engine_client.dart
import 'dart:convert';
import 'package:http/http.dart' as http;

class EngineClient {
  final String baseUrl;
  final http.Client _http;
  int _requestId = 0;
  String? _sessionId;

  EngineClient({required this.baseUrl, http.Client? httpClient})
      : _http = httpClient ?? http.Client();

  /// Build a JSON-RPC request for an MCP tool call.
  Map<String, dynamic> buildToolCallRequest(String toolName, Map<String, dynamic> arguments) {
    _requestId++;
    return {
      'jsonrpc': '2.0',
      'id': _requestId,
      'method': 'tools/call',
      'params': {
        'name': toolName,
        'arguments': arguments,
      },
    };
  }

  /// Initialize the MCP session.
  Future<void> initialize() async {
    _requestId++;
    final body = {
      'jsonrpc': '2.0',
      'id': _requestId,
      'method': 'initialize',
      'params': {
        'protocolVersion': '2024-11-05',
        'capabilities': {},
        'clientInfo': {'name': 'open-ontologies-studio', 'version': '0.1.0'},
      },
    };
    final response = await _post(body);
    // Extract session ID from response headers if present
    _sessionId = response.headers['mcp-session-id'];
  }

  /// Call an MCP tool and return the result content.
  Future<Map<String, dynamic>> callTool(String toolName, Map<String, dynamic> arguments) async {
    final body = buildToolCallRequest(toolName, arguments);
    final response = await _post(body);
    final json = jsonDecode(response.body) as Map<String, dynamic>;
    if (json.containsKey('error')) {
      throw EngineException(json['error']['message'] as String);
    }
    // MCP tool results are in result.content[0].text (JSON string)
    final content = json['result']['content'] as List;
    final text = content.first['text'] as String;
    return jsonDecode(text) as Map<String, dynamic>;
  }

  Future<http.Response> _post(Map<String, dynamic> body) async {
    final headers = <String, String>{
      'Content-Type': 'application/json',
    };
    if (_sessionId != null) {
      headers['Mcp-Session-Id'] = _sessionId!;
    }
    return _http.post(
      Uri.parse('$baseUrl/mcp'),
      headers: headers,
      body: jsonEncode(body),
    );
  }

  void dispose() {
    _http.close();
  }
}

class EngineException implements Exception {
  final String message;
  EngineException(this.message);
  @override
  String toString() => 'EngineException: $message';
}
```

**Step 4: Run test to verify it passes**

```bash
flutter test test/services/engine_client_test.dart
```

Expected: PASS

**Step 5: Commit**

```bash
git add lib/services/engine_client.dart test/services/
git commit -m "feat: add EngineClient for MCP-over-HTTP communication"
```

---

### Task 4: Engine connection provider (Riverpod)

**Files:**
- Create: `lib/services/providers.dart`
- Modify: `lib/main.dart`

**Step 1: Create providers**

```dart
// lib/services/providers.dart
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'engine_client.dart';
import '../models/ontology_stats.dart';
import '../models/graph_node.dart';
import '../models/graph_edge.dart';

/// The engine client singleton. Configure the URL here.
final engineClientProvider = Provider<EngineClient>((ref) {
  final client = EngineClient(baseUrl: 'http://localhost:3000');
  ref.onDispose(() => client.dispose());
  return client;
});

/// Connection status.
enum ConnectionStatus { disconnected, connecting, connected, error }

final connectionStatusProvider = StateProvider<ConnectionStatus>(
  (ref) => ConnectionStatus.disconnected,
);

/// Stats from the engine (refreshed on demand).
final statsProvider = FutureProvider<OntologyStats?>((ref) async {
  final client = ref.watch(engineClientProvider);
  try {
    final result = await client.callTool('onto_stats', {});
    ref.read(connectionStatusProvider.notifier).state = ConnectionStatus.connected;
    return OntologyStats.fromJson(result);
  } catch (_) {
    ref.read(connectionStatusProvider.notifier).state = ConnectionStatus.error;
    return null;
  }
});

/// Graph data — nodes and edges from a SPARQL query.
final graphNodesProvider = FutureProvider<List<GraphNode>>((ref) async {
  final client = ref.watch(engineClientProvider);
  final result = await client.callTool('onto_query', {
    'sparql': '''
      SELECT ?class ?label ?comment WHERE {
        ?class a owl:Class .
        OPTIONAL { ?class rdfs:label ?label }
        OPTIONAL { ?class rdfs:comment ?comment }
      }
    ''',
  });
  final bindings = (result['results']?['bindings'] as List?) ?? [];
  return bindings.map((b) => GraphNode.fromSparqlBinding(b as Map<String, dynamic>)).toList();
});

final graphEdgesProvider = FutureProvider<List<GraphEdge>>((ref) async {
  final client = ref.watch(engineClientProvider);
  final result = await client.callTool('onto_query', {
    'sparql': '''
      SELECT ?subject ?predicate ?object WHERE {
        ?subject rdfs:subClassOf ?object .
        FILTER(isIRI(?object))
        BIND(rdfs:subClassOf AS ?predicate)
      }
    ''',
  });
  final bindings = (result['results']?['bindings'] as List?) ?? [];
  return bindings.map((b) => GraphEdge.fromSparqlBinding(b as Map<String, dynamic>)).toList();
});
```

**Step 2: Wrap the app with ProviderScope**

Update `lib/main.dart`:

```dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

void main() {
  runApp(const ProviderScope(child: OpenOntologiesStudio()));
}

class OpenOntologiesStudio extends StatelessWidget {
  const OpenOntologiesStudio({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Ontologies Studio',
      theme: ThemeData.dark(useMaterial3: true),
      home: const Scaffold(
        body: Center(child: Text('Open Ontologies Studio')),
      ),
    );
  }
}
```

**Step 3: Run the app to verify no errors**

```bash
flutter run -d chrome
```

Expected: app launches with no errors.

**Step 4: Commit**

```bash
git add lib/services/providers.dart lib/main.dart
git commit -m "feat: add Riverpod providers for engine connection and graph data"
```

---

## Phase 2: Graph Canvas (see the ontology)

### Task 5: Basic graph canvas with CustomPainter

**Files:**
- Create: `lib/features/graph/graph_canvas.dart`
- Create: `lib/features/graph/graph_painter.dart`
- Create: `lib/features/graph/graph_state.dart`

**Step 1: Create the graph state**

```dart
// lib/features/graph/graph_state.dart
import 'dart:ui';
import '../../models/graph_node.dart';
import '../../models/graph_edge.dart';

class GraphState {
  final List<GraphNode> nodes;
  final List<GraphEdge> edges;
  Offset panOffset;
  double zoom;
  int? selectedNodeIndex;
  int? hoveredNodeIndex;

  GraphState({
    required this.nodes,
    required this.edges,
    this.panOffset = Offset.zero,
    this.zoom = 1.0,
    this.selectedNodeIndex,
    this.hoveredNodeIndex,
  });
}
```

**Step 2: Create the painter**

```dart
// lib/features/graph/graph_painter.dart
import 'dart:math';
import 'package:flutter/material.dart';
import '../../models/graph_node.dart';
import '../../models/graph_edge.dart';
import 'graph_state.dart';

class GraphPainter extends CustomPainter {
  final GraphState state;

  static const nodeRadius = 24.0;
  static const _classColor = Color(0xFF4A90D9);
  static const _individualColor = Color(0xFF50C878);
  static const _selectedColor = Color(0xFFFFD700);
  static const _edgeColor = Color(0xFF666666);

  GraphPainter(this.state);

  @override
  void paint(Canvas canvas, Size size) {
    canvas.save();
    canvas.translate(
      size.width / 2 + state.panOffset.dx,
      size.height / 2 + state.panOffset.dy,
    );
    canvas.scale(state.zoom);

    _drawEdges(canvas);
    _drawNodes(canvas);

    canvas.restore();
  }

  void _drawEdges(Canvas canvas) {
    final paint = Paint()
      ..color = _edgeColor
      ..strokeWidth = 1.5
      ..style = PaintingStyle.stroke;

    final nodeMap = <String, GraphNode>{};
    for (final node in state.nodes) {
      nodeMap[node.iri] = node;
    }

    for (final edge in state.edges) {
      final source = nodeMap[edge.sourceIri];
      final target = nodeMap[edge.targetIri];
      if (source == null || target == null) continue;

      canvas.drawLine(
        Offset(source.x, source.y),
        Offset(target.x, target.y),
        paint,
      );

      // Arrowhead
      _drawArrowhead(canvas, source, target, paint);
    }
  }

  void _drawArrowhead(Canvas canvas, GraphNode from, GraphNode to, Paint paint) {
    final dx = to.x - from.x;
    final dy = to.y - from.y;
    final dist = sqrt(dx * dx + dy * dy);
    if (dist < 0.001) return;

    final ux = dx / dist;
    final uy = dy / dist;

    // Arrow tip stops at the node edge
    final tipX = to.x - ux * nodeRadius;
    final tipY = to.y - uy * nodeRadius;

    const arrowSize = 10.0;
    const arrowAngle = 0.4;

    final path = Path()
      ..moveTo(tipX, tipY)
      ..lineTo(
        tipX - arrowSize * (ux * cos(arrowAngle) - uy * sin(arrowAngle)),
        tipY - arrowSize * (uy * cos(arrowAngle) + ux * sin(arrowAngle)),
      )
      ..moveTo(tipX, tipY)
      ..lineTo(
        tipX - arrowSize * (ux * cos(arrowAngle) + uy * sin(arrowAngle)),
        tipY - arrowSize * (uy * cos(arrowAngle) - ux * sin(arrowAngle)),
      );

    canvas.drawPath(path, paint);
  }

  void _drawNodes(Canvas canvas) {
    for (var i = 0; i < state.nodes.length; i++) {
      final node = state.nodes[i];
      final isSelected = i == state.selectedNodeIndex;
      final isHovered = i == state.hoveredNodeIndex;

      // Node circle
      final fillPaint = Paint()
        ..color = node.nodeType == NodeType.owlClass ? _classColor : _individualColor
        ..style = PaintingStyle.fill;

      canvas.drawCircle(Offset(node.x, node.y), nodeRadius, fillPaint);

      // Selection/hover ring
      if (isSelected || isHovered) {
        final ringPaint = Paint()
          ..color = isSelected ? _selectedColor : Colors.white54
          ..style = PaintingStyle.stroke
          ..strokeWidth = isSelected ? 3.0 : 2.0;
        canvas.drawCircle(Offset(node.x, node.y), nodeRadius + 3, ringPaint);
      }

      // Label
      final textPainter = TextPainter(
        text: TextSpan(
          text: node.label,
          style: const TextStyle(color: Colors.white, fontSize: 11),
        ),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: 120);

      textPainter.paint(
        canvas,
        Offset(node.x - textPainter.width / 2, node.y + nodeRadius + 4),
      );
    }
  }

  @override
  bool shouldRepaint(GraphPainter oldDelegate) => true;
}
```

**Step 3: Create the interactive canvas widget**

```dart
// lib/features/graph/graph_canvas.dart
import 'dart:math';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import '../../models/graph_node.dart';
import '../../models/graph_edge.dart';
import 'graph_painter.dart';
import 'graph_state.dart';

class GraphCanvas extends StatefulWidget {
  final List<GraphNode> nodes;
  final List<GraphEdge> edges;
  final void Function(GraphNode)? onNodeSelected;

  const GraphCanvas({
    super.key,
    required this.nodes,
    required this.edges,
    this.onNodeSelected,
  });

  @override
  State<GraphCanvas> createState() => _GraphCanvasState();
}

class _GraphCanvasState extends State<GraphCanvas> {
  late GraphState _state;
  int? _draggedNodeIndex;
  Offset? _lastPanPosition;

  @override
  void initState() {
    super.initState();
    _state = GraphState(nodes: widget.nodes, edges: widget.edges);
    _applyInitialLayout();
  }

  @override
  void didUpdateWidget(GraphCanvas oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.nodes != oldWidget.nodes || widget.edges != oldWidget.edges) {
      _state = GraphState(
        nodes: widget.nodes,
        edges: widget.edges,
        panOffset: _state.panOffset,
        zoom: _state.zoom,
      );
      _applyInitialLayout();
    }
  }

  /// Simple circular layout as initial placement.
  void _applyInitialLayout() {
    final n = _state.nodes.length;
    if (n == 0) return;
    final radius = max(100.0, n * 20.0);
    for (var i = 0; i < n; i++) {
      final angle = (2 * pi * i) / n;
      _state.nodes[i].x = radius * cos(angle);
      _state.nodes[i].y = radius * sin(angle);
    }
  }

  int? _hitTest(Offset localPosition, Size size) {
    final canvasCenter = Offset(size.width / 2, size.height / 2);
    for (var i = _state.nodes.length - 1; i >= 0; i--) {
      final node = _state.nodes[i];
      final nodeScreen = Offset(
        canvasCenter.dx + _state.panOffset.dx + node.x * _state.zoom,
        canvasCenter.dy + _state.panOffset.dy + node.y * _state.zoom,
      );
      final dist = (localPosition - nodeScreen).distance;
      if (dist <= GraphPainter.nodeRadius * _state.zoom) return i;
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      onPointerSignal: (event) {
        if (event is PointerScrollEvent) {
          setState(() {
            final delta = event.scrollDelta.dy > 0 ? 0.9 : 1.1;
            _state.zoom = (_state.zoom * delta).clamp(0.1, 5.0);
          });
        }
      },
      child: GestureDetector(
        onPanStart: (details) {
          final hit = _hitTest(details.localPosition, context.size!);
          _draggedNodeIndex = hit;
          _lastPanPosition = details.localPosition;
          if (hit != null) {
            setState(() {
              _state.selectedNodeIndex = hit;
            });
            widget.onNodeSelected?.call(_state.nodes[hit]);
          }
        },
        onPanUpdate: (details) {
          setState(() {
            if (_draggedNodeIndex != null) {
              // Drag node
              _state.nodes[_draggedNodeIndex!].x += details.delta.dx / _state.zoom;
              _state.nodes[_draggedNodeIndex!].y += details.delta.dy / _state.zoom;
            } else {
              // Pan canvas
              _state.panOffset += details.delta;
            }
          });
          _lastPanPosition = details.localPosition;
        },
        onPanEnd: (_) {
          _draggedNodeIndex = null;
          _lastPanPosition = null;
        },
        child: CustomPaint(
          painter: GraphPainter(_state),
          size: Size.infinite,
        ),
      ),
    );
  }
}
```

**Step 4: Verify it compiles**

```bash
flutter analyze
```

Expected: no errors.

**Step 5: Commit**

```bash
git add lib/features/graph/
git commit -m "feat: add graph canvas with CustomPainter, pan, zoom, node drag"
```

---

### Task 6: Force-directed layout (pure Dart)

**Files:**
- Create: `lib/features/graph/force_layout.dart`
- Test: `test/features/graph/force_layout_test.dart`

**Step 1: Write the failing test**

```dart
// test/features/graph/force_layout_test.dart
import 'package:flutter_test/flutter_test.dart';
import 'package:open_ontologies_studio/models/graph_node.dart';
import 'package:open_ontologies_studio/models/graph_edge.dart';
import 'package:open_ontologies_studio/features/graph/force_layout.dart';

void main() {
  test('ForceLayout separates overlapping nodes', () {
    final nodes = [
      GraphNode(iri: 'a', label: 'A', x: 0, y: 0),
      GraphNode(iri: 'b', label: 'B', x: 0, y: 0),
    ];
    final edges = <GraphEdge>[];

    final layout = ForceLayout(nodes: nodes, edges: edges);
    for (var i = 0; i < 50; i++) {
      layout.tick();
    }

    final dx = nodes[0].x - nodes[1].x;
    final dy = nodes[0].y - nodes[1].y;
    final distance = (dx * dx + dy * dy);
    expect(distance, greaterThan(100)); // nodes should have separated
  });

  test('ForceLayout pulls connected nodes closer', () {
    final nodes = [
      GraphNode(iri: 'a', label: 'A', x: -500, y: 0),
      GraphNode(iri: 'b', label: 'B', x: 500, y: 0),
    ];
    final edges = [
      GraphEdge(sourceIri: 'a', targetIri: 'b', predicate: 'rdfs:subClassOf'),
    ];

    final layout = ForceLayout(nodes: nodes, edges: edges);
    final initialDistance = 1000.0;
    for (var i = 0; i < 100; i++) {
      layout.tick();
    }

    final dx = nodes[0].x - nodes[1].x;
    final dy = nodes[0].y - nodes[1].y;
    final finalDistance = (dx * dx + dy * dy);
    expect(finalDistance, lessThan(initialDistance * initialDistance));
  });
}
```

**Step 2: Run test to verify it fails**

```bash
flutter test test/features/graph/force_layout_test.dart
```

Expected: FAIL

**Step 3: Write the force layout**

```dart
// lib/features/graph/force_layout.dart
import 'dart:math';
import '../../models/graph_node.dart';
import '../../models/graph_edge.dart';

class ForceLayout {
  final List<GraphNode> nodes;
  final List<GraphEdge> edges;

  double repulsionStrength;
  double attractionStrength;
  double damping;
  double idealEdgeLength;

  final List<double> _vx;
  final List<double> _vy;

  ForceLayout({
    required this.nodes,
    required this.edges,
    this.repulsionStrength = 5000.0,
    this.attractionStrength = 0.01,
    this.damping = 0.9,
    this.idealEdgeLength = 150.0,
  })  : _vx = List.filled(nodes.length, 0.0),
        _vy = List.filled(nodes.length, 0.0);

  /// Run one simulation tick. Returns total kinetic energy.
  double tick() {
    // Repulsion (all pairs)
    for (var i = 0; i < nodes.length; i++) {
      for (var j = i + 1; j < nodes.length; j++) {
        var dx = nodes[j].x - nodes[i].x;
        var dy = nodes[j].y - nodes[i].y;
        var dist = sqrt(dx * dx + dy * dy);
        if (dist < 1.0) {
          dx = (Random().nextDouble() - 0.5) * 2;
          dy = (Random().nextDouble() - 0.5) * 2;
          dist = 1.0;
        }
        final force = repulsionStrength / (dist * dist);
        final fx = force * dx / dist;
        final fy = force * dy / dist;
        _vx[i] -= fx;
        _vy[i] -= fy;
        _vx[j] += fx;
        _vy[j] += fy;
      }
    }

    // Attraction (edges)
    final nodeIndex = <String, int>{};
    for (var i = 0; i < nodes.length; i++) {
      nodeIndex[nodes[i].iri] = i;
    }

    for (final edge in edges) {
      final si = nodeIndex[edge.sourceIri];
      final ti = nodeIndex[edge.targetIri];
      if (si == null || ti == null) continue;

      final dx = nodes[ti].x - nodes[si].x;
      final dy = nodes[ti].y - nodes[si].y;
      final dist = sqrt(dx * dx + dy * dy);
      if (dist < 0.001) continue;

      final force = attractionStrength * (dist - idealEdgeLength);
      final fx = force * dx / dist;
      final fy = force * dy / dist;
      _vx[si] += fx;
      _vy[si] += fy;
      _vx[ti] -= fx;
      _vy[ti] -= fy;
    }

    // Apply velocities with damping
    var energy = 0.0;
    for (var i = 0; i < nodes.length; i++) {
      _vx[i] *= damping;
      _vy[i] *= damping;
      nodes[i].x += _vx[i];
      nodes[i].y += _vy[i];
      energy += _vx[i] * _vx[i] + _vy[i] * _vy[i];
    }

    return energy;
  }

  /// Run until converged or max iterations.
  void run({int maxIterations = 300, double threshold = 0.5}) {
    for (var i = 0; i < maxIterations; i++) {
      final energy = tick();
      if (energy < threshold) break;
    }
  }
}
```

**Step 4: Run test to verify it passes**

```bash
flutter test test/features/graph/force_layout_test.dart
```

Expected: PASS

**Step 5: Commit**

```bash
git add lib/features/graph/force_layout.dart test/features/graph/
git commit -m "feat: add force-directed graph layout in pure Dart"
```

---

## Phase 3: Main UI Shell (wire it together)

### Task 7: Main screen with graph + status bar

**Files:**
- Create: `lib/features/home_screen.dart`
- Modify: `lib/main.dart`

**Step 1: Create the home screen**

```dart
// lib/features/home_screen.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../services/providers.dart';
import 'graph/graph_canvas.dart';

class HomeScreen extends ConsumerStatefulWidget {
  const HomeScreen({super.key});

  @override
  ConsumerState<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends ConsumerState<HomeScreen> {
  @override
  void initState() {
    super.initState();
    _connect();
  }

  Future<void> _connect() async {
    ref.read(connectionStatusProvider.notifier).state = ConnectionStatus.connecting;
    try {
      await ref.read(engineClientProvider).initialize();
      ref.read(connectionStatusProvider.notifier).state = ConnectionStatus.connected;
      ref.invalidate(statsProvider);
      ref.invalidate(graphNodesProvider);
      ref.invalidate(graphEdgesProvider);
    } catch (e) {
      ref.read(connectionStatusProvider.notifier).state = ConnectionStatus.error;
    }
  }

  @override
  Widget build(BuildContext context) {
    final status = ref.watch(connectionStatusProvider);
    final nodesAsync = ref.watch(graphNodesProvider);
    final edgesAsync = ref.watch(graphEdgesProvider);
    final statsAsync = ref.watch(statsProvider);

    return Scaffold(
      body: Column(
        children: [
          // Graph canvas (fills available space)
          Expanded(
            child: nodesAsync.when(
              data: (nodes) => edgesAsync.when(
                data: (edges) => nodes.isEmpty
                    ? const Center(child: Text('No ontology loaded. Use Claude or CLI to load one.'))
                    : GraphCanvas(nodes: nodes, edges: edges),
                loading: () => const Center(child: CircularProgressIndicator()),
                error: (e, _) => Center(child: Text('Error: $e')),
              ),
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Center(child: Text('Error: $e')),
            ),
          ),

          // Status bar
          Container(
            height: 28,
            padding: const EdgeInsets.symmetric(horizontal: 12),
            color: Colors.grey[900],
            child: Row(
              children: [
                // Connection indicator
                Icon(
                  Icons.circle,
                  size: 8,
                  color: switch (status) {
                    ConnectionStatus.connected => Colors.green,
                    ConnectionStatus.connecting => Colors.orange,
                    ConnectionStatus.error => Colors.red,
                    ConnectionStatus.disconnected => Colors.grey,
                  },
                ),
                const SizedBox(width: 6),
                Text(
                  switch (status) {
                    ConnectionStatus.connected => 'Connected',
                    ConnectionStatus.connecting => 'Connecting...',
                    ConnectionStatus.error => 'Disconnected',
                    ConnectionStatus.disconnected => 'Not connected',
                  },
                  style: const TextStyle(fontSize: 11),
                ),
                const Spacer(),
                // Stats
                statsAsync.when(
                  data: (stats) => stats == null
                      ? const SizedBox()
                      : Text(
                          '${stats.classes} classes  ${stats.properties} props  ${stats.triples} triples',
                          style: const TextStyle(fontSize: 11, color: Colors.grey),
                        ),
                  loading: () => const SizedBox(),
                  error: (_, __) => const SizedBox(),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
```

**Step 2: Update main.dart to use HomeScreen**

```dart
// lib/main.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'features/home_screen.dart';

void main() {
  runApp(const ProviderScope(child: OpenOntologiesStudio()));
}

class OpenOntologiesStudio extends StatelessWidget {
  const OpenOntologiesStudio({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Open Ontologies Studio',
      debugShowCheckedModeBanner: false,
      theme: ThemeData.dark(useMaterial3: true).copyWith(
        scaffoldBackgroundColor: const Color(0xFF1E1E2E),
      ),
      home: const HomeScreen(),
    );
  }
}
```

**Step 3: Run the app**

```bash
# Terminal 1: start the engine with a loaded ontology
open-ontologies serve-http

# Terminal 2: run the Studio
cd /Users/fabio/projects/open-ontologies-studio
flutter run -d chrome
```

Expected: dark app with graph canvas (empty if no ontology loaded), status bar showing connection status.

**Step 4: Commit**

```bash
git add lib/features/home_screen.dart lib/main.dart
git commit -m "feat: add main screen with graph canvas and status bar"
```

---

### Task 8: Property inspector panel

**Files:**
- Create: `lib/features/inspector/property_inspector.dart`
- Modify: `lib/features/home_screen.dart`

**Step 1: Create the inspector**

```dart
// lib/features/inspector/property_inspector.dart
import 'package:flutter/material.dart';
import '../../models/graph_node.dart';

class PropertyInspector extends StatelessWidget {
  final GraphNode node;
  final VoidCallback onClose;

  const PropertyInspector({
    super.key,
    required this.node,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 300,
      color: const Color(0xFF252535),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Container(
            padding: const EdgeInsets.all(12),
            color: const Color(0xFF2D2D3F),
            child: Row(
              children: [
                const Icon(Icons.info_outline, size: 16),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    node.label,
                    style: const TextStyle(fontWeight: FontWeight.bold),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                IconButton(
                  icon: const Icon(Icons.close, size: 16),
                  onPressed: onClose,
                  padding: EdgeInsets.zero,
                  constraints: const BoxConstraints(),
                ),
              ],
            ),
          ),

          // Properties
          Expanded(
            child: ListView(
              padding: const EdgeInsets.all(12),
              children: [
                _propertyRow('IRI', node.iri),
                _propertyRow('Type', node.nodeType.name),
                if (node.comment != null) _propertyRow('Comment', node.comment!),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _propertyRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: const TextStyle(fontSize: 10, color: Colors.grey)),
          const SizedBox(height: 2),
          SelectableText(value, style: const TextStyle(fontSize: 13)),
        ],
      ),
    );
  }
}
```

**Step 2: Wire into HomeScreen**

Add a `selectedNode` state and show the inspector as a side panel when a node is selected. In `home_screen.dart`, wrap the graph in a `Row` with a conditional `PropertyInspector` on the right.

Add state:
```dart
GraphNode? _selectedNode;
```

Update the body to:
```dart
Expanded(
  child: Row(
    children: [
      Expanded(child: /* existing graph canvas with onNodeSelected */),
      if (_selectedNode != null)
        PropertyInspector(
          node: _selectedNode!,
          onClose: () => setState(() => _selectedNode = null),
        ),
    ],
  ),
),
```

Pass `onNodeSelected: (node) => setState(() => _selectedNode = node)` to `GraphCanvas`.

**Step 3: Verify it works**

```bash
flutter run -d chrome
```

Expected: clicking a node opens the property panel on the right. Clicking X closes it.

**Step 4: Commit**

```bash
git add lib/features/inspector/ lib/features/home_screen.dart
git commit -m "feat: add property inspector panel for selected nodes"
```

---

## Phase 4: Load + Validate (make it useful)

### Task 9: File picker to load ontology via engine

**Files:**
- Create: `lib/features/toolbar.dart`
- Modify: `lib/features/home_screen.dart`
- Add dependency: `file_picker` to `pubspec.yaml`

**Step 1: Add file_picker dependency**

```yaml
# Add to pubspec.yaml dependencies
file_picker: ^8.0.0
```

```bash
flutter pub get
```

**Step 2: Create toolbar**

```dart
// lib/features/toolbar.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:file_picker/file_picker.dart';
import 'dart:io';
import '../services/providers.dart';

class Toolbar extends ConsumerWidget {
  const Toolbar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      height: 40,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      color: const Color(0xFF2D2D3F),
      child: Row(
        children: [
          const Text(
            'Open Ontologies Studio',
            style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13),
          ),
          const SizedBox(width: 16),
          _toolbarButton(
            icon: Icons.folder_open,
            label: 'Load',
            onPressed: () => _loadOntology(context, ref),
          ),
          _toolbarButton(
            icon: Icons.save,
            label: 'Save',
            onPressed: () => _saveOntology(context, ref),
          ),
          _toolbarButton(
            icon: Icons.delete_outline,
            label: 'Clear',
            onPressed: () => _clearStore(context, ref),
          ),
          const Spacer(),
          _toolbarButton(
            icon: Icons.refresh,
            label: 'Refresh',
            onPressed: () {
              ref.invalidate(graphNodesProvider);
              ref.invalidate(graphEdgesProvider);
              ref.invalidate(statsProvider);
            },
          ),
        ],
      ),
    );
  }

  Widget _toolbarButton({
    required IconData icon,
    required String label,
    required VoidCallback onPressed,
  }) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 2),
      child: TextButton.icon(
        icon: Icon(icon, size: 14),
        label: Text(label, style: const TextStyle(fontSize: 12)),
        onPressed: onPressed,
        style: TextButton.styleFrom(
          padding: const EdgeInsets.symmetric(horizontal: 8),
          minimumSize: const Size(0, 30),
        ),
      ),
    );
  }

  Future<void> _loadOntology(BuildContext context, WidgetRef ref) async {
    final result = await FilePicker.platform.pickFiles(
      type: FileType.custom,
      allowedExtensions: ['ttl', 'owl', 'rdf', 'nt', 'nq', 'trig'],
    );
    if (result == null) return;

    final file = File(result.files.single.path!);
    final content = await file.readAsString();

    final client = ref.read(engineClientProvider);
    try {
      // Validate first
      await client.callTool('onto_validate', {'turtle': content});
      // Clear and load
      await client.callTool('onto_clear', {});
      await client.callTool('onto_load', {'turtle': content});
      // Refresh
      ref.invalidate(graphNodesProvider);
      ref.invalidate(graphEdgesProvider);
      ref.invalidate(statsProvider);
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  Future<void> _saveOntology(BuildContext context, WidgetRef ref) async {
    final client = ref.read(engineClientProvider);
    try {
      final result = await client.callTool('onto_save', {'format': 'turtle'});
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Saved: ${result['path'] ?? 'ok'}')),
        );
      }
    } catch (e) {
      if (context.mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Error: $e'), backgroundColor: Colors.red),
        );
      }
    }
  }

  Future<void> _clearStore(BuildContext context, WidgetRef ref) async {
    final client = ref.read(engineClientProvider);
    await client.callTool('onto_clear', {});
    ref.invalidate(graphNodesProvider);
    ref.invalidate(graphEdgesProvider);
    ref.invalidate(statsProvider);
  }
}
```

**Step 3: Add Toolbar to HomeScreen**

In `home_screen.dart`, add `const Toolbar()` as the first child in the `Column`, above the `Expanded` graph area.

**Step 4: Verify**

```bash
flutter run -d chrome
```

Expected: toolbar with Load, Save, Clear, Refresh buttons. Load opens file picker, loads a .ttl file, graph renders.

**Step 5: Commit**

```bash
git add lib/features/toolbar.dart lib/features/home_screen.dart pubspec.yaml
git commit -m "feat: add toolbar with load, save, clear, refresh actions"
```

---

### Task 10: Validation panel

**Files:**
- Create: `lib/features/validation/validation_panel.dart`
- Create: `lib/services/providers.dart` (add lint provider)

**Step 1: Add lint provider to providers.dart**

```dart
/// Lint results from the engine.
final lintProvider = FutureProvider<List<Map<String, dynamic>>>((ref) async {
  final client = ref.watch(engineClientProvider);
  try {
    final result = await client.callTool('onto_lint', {});
    return (result['issues'] as List?)?.cast<Map<String, dynamic>>() ?? [];
  } catch (_) {
    return [];
  }
});
```

**Step 2: Create validation panel**

```dart
// lib/features/validation/validation_panel.dart
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import '../../services/providers.dart';

class ValidationPanel extends ConsumerWidget {
  const ValidationPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final lintAsync = ref.watch(lintProvider);

    return Container(
      height: 200,
      color: const Color(0xFF252535),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
            color: const Color(0xFF2D2D3F),
            child: Row(
              children: [
                const Icon(Icons.checklist, size: 14),
                const SizedBox(width: 6),
                const Text('Validation', style: TextStyle(fontSize: 12, fontWeight: FontWeight.bold)),
                const Spacer(),
                lintAsync.when(
                  data: (issues) => Text(
                    '${issues.length} issue${issues.length == 1 ? '' : 's'}',
                    style: TextStyle(
                      fontSize: 11,
                      color: issues.isEmpty ? Colors.green : Colors.orange,
                    ),
                  ),
                  loading: () => const SizedBox(width: 12, height: 12, child: CircularProgressIndicator(strokeWidth: 1)),
                  error: (_, __) => const Text('error', style: TextStyle(fontSize: 11, color: Colors.red)),
                ),
              ],
            ),
          ),
          Expanded(
            child: lintAsync.when(
              data: (issues) => issues.isEmpty
                  ? const Center(child: Text('No issues', style: TextStyle(color: Colors.grey, fontSize: 12)))
                  : ListView.builder(
                      itemCount: issues.length,
                      itemBuilder: (context, index) {
                        final issue = issues[index];
                        final severity = issue['severity'] as String? ?? 'warning';
                        return ListTile(
                          dense: true,
                          leading: Icon(
                            severity == 'error' ? Icons.error : Icons.warning,
                            color: severity == 'error' ? Colors.red : Colors.orange,
                            size: 16,
                          ),
                          title: Text(
                            issue['message'] as String? ?? '',
                            style: const TextStyle(fontSize: 12),
                          ),
                          subtitle: Text(
                            issue['iri'] as String? ?? '',
                            style: const TextStyle(fontSize: 10, color: Colors.grey),
                          ),
                        );
                      },
                    ),
              loading: () => const Center(child: CircularProgressIndicator()),
              error: (e, _) => Center(child: Text('$e')),
            ),
          ),
        ],
      ),
    );
  }
}
```

**Step 3: Add to HomeScreen**

Add `const ValidationPanel()` below the graph `Expanded` and above the status bar, wrapped in a visibility toggle.

**Step 4: Commit**

```bash
git add lib/features/validation/ lib/services/providers.dart lib/features/home_screen.dart
git commit -m "feat: add validation panel showing lint issues"
```

---

## Phase 5: Editing (make it interactive)

### Task 11: Add class via right-click context menu on canvas

**Files:**
- Modify: `lib/features/graph/graph_canvas.dart`
- Modify: `lib/services/engine_client.dart` (add insert triple method)

This task adds a right-click context menu on the canvas background that lets users create a new class. The new class is added to the triple store via `onto_load` (loading a small Turtle snippet), then the graph refreshes.

**Step 1:** Add a `secondaryTapUp` handler to `GraphCanvas` that shows a `PopupMenuButton`-style overlay.

**Step 2:** Menu options: "Add Class", "Add Individual". Selecting "Add Class" opens a dialog asking for class IRI and label.

**Step 3:** On submit, build Turtle: `<iri> a owl:Class ; rdfs:label "label" .` and call `onto_load`.

**Step 4:** Refresh providers to update the graph.

**Step 5: Commit**

```bash
git add lib/features/graph/ lib/services/
git commit -m "feat: add right-click context menu to create classes on canvas"
```

---

### Task 12: Drag-to-connect (create subClassOf relationships)

**Files:**
- Modify: `lib/features/graph/graph_canvas.dart`

Add a mode where dragging from one node to another creates a `rdfs:subClassOf` triple. Visual feedback: dotted line follows the cursor during drag. On release over a target node, generate Turtle and call `onto_load`.

**Step 1:** Add drag-from-node state (source node, dragging flag).

**Step 2:** On drag start over a node with Shift held, enter connection mode.

**Step 3:** Paint a dotted line from source to cursor position.

**Step 4:** On release over a target node, call `onto_load` with `<source> rdfs:subClassOf <target> .`

**Step 5: Commit**

```bash
git add lib/features/graph/
git commit -m "feat: add shift-drag to create subClassOf edges between nodes"
```

---

## Phase 6: Command Palette + SPARQL

### Task 13: Command palette (Cmd+K)

**Files:**
- Create: `lib/features/command_palette/command_palette.dart`
- Modify: `lib/features/home_screen.dart` (keyboard shortcut)

**Step 1:** Create a `CommandPalette` overlay widget — a search field at top of screen with a list of matching commands below.

**Step 2:** Commands list: "Load ontology", "Save ontology", "Clear store", "Run SPARQL", "Export Turtle", "Refresh graph".

**Step 3:** Filter commands as user types. Enter selects. Escape closes.

**Step 4:** Register `Cmd+K` / `Ctrl+K` keyboard shortcut in HomeScreen using `Shortcuts` and `Actions` widgets.

**Step 5: Commit**

```bash
git add lib/features/command_palette/ lib/features/home_screen.dart
git commit -m "feat: add Cmd+K command palette"
```

---

### Task 14: SPARQL editor panel

**Files:**
- Create: `lib/features/sparql/sparql_editor.dart`

**Step 1:** Create a panel with a multi-line text field for SPARQL input and a results table below.

**Step 2:** "Run" button calls `onto_query` with the SPARQL text.

**Step 3:** Parse the JSON results into a `DataTable`.

**Step 4:** Add a keyboard shortcut `Cmd+Enter` to run the query.

**Step 5: Commit**

```bash
git add lib/features/sparql/
git commit -m "feat: add SPARQL editor panel with query execution"
```

---

## Phase 7: Lineage + Version History

### Task 15: Lineage timeline panel

**Files:**
- Create: `lib/features/lineage/lineage_panel.dart`
- Add provider for `onto_history`

**Step 1:** Add `historyProvider` that calls `onto_history` and returns a list of version snapshots.

**Step 2:** Render as a vertical timeline with version name, timestamp, and source (claude/human).

**Step 3:** Each entry has [Load] and [Diff] buttons. Load calls `onto_rollback`, Diff calls `onto_diff`.

**Step 4: Commit**

```bash
git add lib/features/lineage/
git commit -m "feat: add lineage timeline with version history"
```

---

## Phase 8: CI + Packaging

### Task 16: GitHub Actions CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1:** Create workflow that runs on push/PR:
- `flutter analyze`
- `flutter test`
- Build for web (`flutter build web`)

**Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add Flutter CI workflow for analyze, test, web build"
```

---

### Task 17: GitHub Actions release workflow for desktop builds

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1:** Create workflow triggered on tags (`v*`):
- Matrix build: macOS, Windows, Linux
- `flutter build macos` / `flutter build windows` / `flutter build linux`
- Package as `.dmg` / `.msix` / `.AppImage`
- Upload to GitHub Release

**Step 2: Commit**

```bash
git add .github/
git commit -m "ci: add release workflow for macOS, Windows, Linux desktop builds"
```

---

## Phase 9: Polish

### Task 18: Dark theme + visual polish

Refine colors, spacing, typography. Node colors per type, edge styles per predicate, hover effects, smooth transitions.

### Task 19: LOD rendering (level-of-detail)

When zoom is far out and >500 nodes visible, collapse into clusters. Show labels only when zoomed in enough.

### Task 20: Add README and documentation

Create README.md with screenshots, install instructions, and architecture overview.

---

## Execution Order Summary

| Phase | Tasks | What you get |
|---|---|---|
| 1. Scaffold | 1-4 | Flutter project, models, engine client, providers |
| 2. Graph Canvas | 5-6 | Visual graph with pan/zoom/drag, force layout |
| 3. UI Shell | 7-8 | Main screen, property inspector |
| 4. Load + Validate | 9-10 | File picker, validation panel |
| 5. Editing | 11-12 | Add classes, create relationships on canvas |
| 6. Cmd Palette + SPARQL | 13-14 | Keyboard shortcuts, query editor |
| 7. Lineage | 15 | Version history and rollback |
| 8. CI | 16-17 | Automated testing and desktop packaging |
| 9. Polish | 18-20 | Visual refinement, LOD, docs |

**After Phase 3 (Task 8) you have a working visual ontology viewer.**
**After Phase 5 (Task 12) you have a working visual ontology editor.**
