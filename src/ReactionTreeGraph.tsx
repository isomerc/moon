import { useMemo } from "react";
import {
  ReactFlow,
  Node,
  Edge,
  Position,
  Handle,
  Controls,
  useNodesState,
  useEdgesState,
} from "@xyflow/react";
import dagre from "dagre";
import "@xyflow/react/dist/style.css";

type SourceType = "moon" | "buy" | "react" | "output";

interface ReactionTreeNode {
  name: string;
  id: number;
  quantity: number;
  source: SourceType;
  unit_price: number;
  total_price: number;
  reaction_name: string | null;
  children: ReactionTreeNode[];
}

interface ReactionTreeGraphProps {
  tree: ReactionTreeNode;
  formatIsk: (value: number) => string;
}

// Custom node component
function ReactionNode({ data }: { data: { label: string; source: SourceType; quantity: number; price: string } }) {
  const sourceColors: Record<SourceType, { bg: string; border: string; text: string }> = {
    output: { bg: "#4f46e5", border: "#6366f1", text: "#fff" },
    react: { bg: "#d97706", border: "#f59e0b", text: "#fff" },
    moon: { bg: "#059669", border: "#10b981", text: "#fff" },
    buy: { bg: "#dc2626", border: "#ef4444", text: "#fff" },
  };

  const sourceLabels: Record<SourceType, string> = {
    output: "OUTPUT",
    react: "REACT",
    moon: "MOON",
    buy: "BUY",
  };

  const colors = sourceColors[data.source];

  return (
    <div
      style={{
        background: "#0a3d30",
        border: `2px solid ${colors.border}`,
        borderRadius: "8px",
        padding: "10px 14px",
        minWidth: "140px",
        boxShadow: "0 4px 12px rgba(0,0,0,0.3)",
      }}
    >
      <Handle type="target" position={Position.Top} style={{ background: colors.border }} />

      <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: "8px" }}>
          <span
            style={{
              fontSize: "10px",
              fontWeight: 600,
              padding: "2px 6px",
              borderRadius: "4px",
              background: colors.bg,
              color: colors.text,
            }}
          >
            {sourceLabels[data.source]}
          </span>
          <span style={{ fontSize: "11px", color: "#b8e6d4" }}>
            x{data.quantity.toLocaleString()}
          </span>
        </div>

        <div style={{
          fontSize: "13px",
          fontWeight: 600,
          color: "#fff",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}>
          {data.label}
        </div>

        <div style={{ fontSize: "11px", color: "#00e5a0", fontFamily: "monospace" }}>
          {data.price}
        </div>
      </div>

      <Handle type="source" position={Position.Bottom} style={{ background: colors.border }} />
    </div>
  );
}

const nodeTypes = {
  reaction: ReactionNode,
};

// Layout the graph using dagre
function getLayoutedElements(nodes: Node[], edges: Edge[]) {
  const g = new dagre.graphlib.Graph().setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: "TB", nodesep: 50, ranksep: 80 });

  nodes.forEach((node) => {
    g.setNode(node.id, { width: 160, height: 80 });
  });

  edges.forEach((edge) => {
    g.setEdge(edge.source, edge.target);
  });

  dagre.layout(g);

  const layoutedNodes = nodes.map((node) => {
    const nodeWithPosition = g.node(node.id);
    return {
      ...node,
      position: {
        x: nodeWithPosition.x - 80,
        y: nodeWithPosition.y - 40,
      },
    };
  });

  return { nodes: layoutedNodes, edges };
}

// Convert our tree structure to React Flow nodes and edges
function treeToFlow(
  tree: ReactionTreeNode,
  formatIsk: (value: number) => string,
  parentId: string | null = null,
  nodes: Node[] = [],
  edges: Edge[] = [],
  idCounter = { value: 0 }
): { nodes: Node[]; edges: Edge[] } {
  const nodeId = `node-${idCounter.value++}`;

  nodes.push({
    id: nodeId,
    type: "reaction",
    position: { x: 0, y: 0 }, // Will be set by dagre
    data: {
      label: tree.name,
      source: tree.source,
      quantity: tree.quantity,
      price: formatIsk(tree.total_price),
    },
  });

  if (parentId) {
    edges.push({
      id: `edge-${parentId}-${nodeId}`,
      source: nodeId,
      target: parentId,
      style: { stroke: "#1a8c6b", strokeWidth: 2 },
      animated: tree.source === "react",
    });
  }

  for (const child of tree.children) {
    treeToFlow(child, formatIsk, nodeId, nodes, edges, idCounter);
  }

  return { nodes, edges };
}

export default function ReactionTreeGraph({ tree, formatIsk }: ReactionTreeGraphProps) {
  const { nodes: initialNodes, edges: initialEdges } = useMemo(() => {
    const { nodes, edges } = treeToFlow(tree, formatIsk);
    return getLayoutedElements(nodes, edges);
  }, [tree, formatIsk]);

  const [nodes, , onNodesChange] = useNodesState(initialNodes);
  const [edges, , onEdgesChange] = useEdgesState(initialEdges);

  return (
    <div style={{ width: "100%", height: "400px", background: "#0d4a3a", borderRadius: "8px" }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        nodeTypes={nodeTypes}
        fitView
        fitViewOptions={{ padding: 0.2 }}
        proOptions={{ hideAttribution: true }}
        nodesDraggable={false}
        nodesConnectable={false}
        elementsSelectable={false}
        panOnScroll
        zoomOnScroll
        minZoom={0.3}
        maxZoom={1.5}
      >
        <Controls
          showInteractive={false}
          style={{
            background: "#0a3d30",
            border: "1px solid #1a8c6b",
            borderRadius: "6px",
          }}
        />
      </ReactFlow>
    </div>
  );
}
