import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import ReactionTreeGraph from "./ReactionTreeGraph";
import "./App.css";

// Types matching the Rust backend
interface MaterialEntry {
  name: string;
  quantity: number;
  item_id: number;
  system_id: number;
  region_id: number;
  additional_id: number;
}

interface MoonComposition {
  name: string;
  materials: MaterialEntry[];
}

interface InputBreakdown {
  name: string;
  quantity: number;
  unit_price: number;
  total_price: number;
  from_moon: boolean;
}

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

interface ReactionProfit {
  formula_id: number;
  formula_name: string;
  output_name: string;
  output_id: number;
  output_quantity: number;
  output_unit_price: number;
  output_value: number;
  input_cost: number;
  profit: number;
  margin: number;
  inputs: InputBreakdown[];
  uses_user_materials: boolean;
  reaction_tree: ReactionTreeNode | null;
}

interface Tab {
  id: string;
  name: string;
  results?: ReactionProfit[];
}

type SortField = "output_name" | "output_quantity" | "input_cost" | "output_value" | "profit" | "margin";
type SortDirection = "asc" | "desc";

function App() {
  const [inputText, setInputText] = useState("");
  const [moons, setMoons] = useState<MoonComposition[]>([]);
  const [uniqueMaterials, setUniqueMaterials] = useState<string[]>([]);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [tabs, setTabs] = useState<Tab[]>([{ id: "home", name: "Home" }]);
  const [activeTab, setActiveTab] = useState("home");
  const [isAnalyzing, setIsAnalyzing] = useState(false);
  const [expandedReactions, setExpandedReactions] = useState<Set<number>>(new Set());
  const [sortField, setSortField] = useState<SortField>("margin");
  const [sortDirection, setSortDirection] = useState<SortDirection>("desc");
  const [reactionDetailTab, setReactionDetailTab] = useState<Record<number, "graph" | "text">>({});

  // Load moons and materials on mount and after changes
  const refreshData = async () => {
    try {
      const loadedMoons = await invoke<MoonComposition[]>("get_moons");
      const materials = await invoke<string[]>("get_unique_materials");
      setMoons(loadedMoons);
      setUniqueMaterials(materials);
    } catch (err) {
      console.error("Failed to load data:", err);
    }
  };

  useEffect(() => {
    refreshData();
  }, []);

  const handleAddMoon = async () => {
    setErrorMessage(null);

    if (!inputText.trim()) {
      setErrorMessage("Please enter moon scan data");
      return;
    }

    try {
      // Parse the input
      const parsed = await invoke<MoonComposition[]>("parse_moon_data", {
        input: inputText,
      });

      // Add to state
      await invoke("add_moon", { moonsToAdd: parsed });

      // Clear input and refresh
      setInputText("");
      await refreshData();
    } catch (err) {
      setErrorMessage(String(err));
    }
  };

  const handleDeleteMoon = async (index: number) => {
    try {
      await invoke("delete_moon", { index });
      await refreshData();
    } catch (err) {
      setErrorMessage(String(err));
    }
  };

  const handleGo = async () => {
    if (moons.length === 0) {
      setErrorMessage("Add some moons first before analyzing");
      return;
    }

    setIsAnalyzing(true);
    setErrorMessage(null);

    try {
      const results = await invoke<ReactionProfit[]>("analyze_reactions");

      // Create a new tab with results
      const newTabId = `analysis-${Date.now()}`;
      const newTab: Tab = {
        id: newTabId,
        name: `Analysis ${tabs.filter((t) => t.id.startsWith("analysis")).length + 1}`,
        results,
      };

      setTabs([...tabs, newTab]);
      setActiveTab(newTabId);
    } catch (err) {
      setErrorMessage(String(err));
    } finally {
      setIsAnalyzing(false);
    }
  };

  const handleCloseTab = (tabId: string) => {
    if (tabId === "home") return; // Can't close home tab

    const newTabs = tabs.filter((t) => t.id !== tabId);
    setTabs(newTabs);

    // If closing active tab, switch to home
    if (activeTab === tabId) {
      setActiveTab("home");
    }
  };

  const formatIsk = (value: number) => {
    if (Math.abs(value) >= 1_000_000_000) {
      return (value / 1_000_000_000).toFixed(2) + "B ISK";
    } else if (Math.abs(value) >= 1_000_000) {
      return (value / 1_000_000).toFixed(2) + "M ISK";
    } else if (Math.abs(value) >= 1_000) {
      return (value / 1_000).toFixed(2) + "K ISK";
    }
    return value.toFixed(2) + " ISK";
  };

  const activeTabData = tabs.find((t) => t.id === activeTab);

  const toggleReactionExpanded = (formulaId: number) => {
    setExpandedReactions((prev) => {
      const next = new Set(prev);
      if (next.has(formulaId)) {
        next.delete(formulaId);
      } else {
        next.add(formulaId);
      }
      return next;
    });
  };

  const handleSort = (field: SortField) => {
    if (sortField === field) {
      setSortDirection(sortDirection === "asc" ? "desc" : "asc");
    } else {
      setSortField(field);
      setSortDirection("desc");
    }
  };

  const getSortedResults = (results: ReactionProfit[]) => {
    return [...results].sort((a, b) => {
      let comparison = 0;
      if (sortField === "output_name") {
        comparison = a.output_name.localeCompare(b.output_name);
      } else {
        comparison = a[sortField] - b[sortField];
      }
      return sortDirection === "asc" ? comparison : -comparison;
    });
  };

  const SortIcon = ({ field }: { field: SortField }) => {
    if (sortField !== field) return <span className="sort-icon">⇅</span>;
    return <span className="sort-icon active">{sortDirection === "asc" ? "↑" : "↓"}</span>;
  };

  const getDetailTab = (formulaId: number) => reactionDetailTab[formulaId] || "graph";

  const setDetailTab = (formulaId: number, tab: "graph" | "text") => {
    setReactionDetailTab(prev => ({ ...prev, [formulaId]: tab }));
  };

  // Generate text instructions from reaction tree
  const generateTextInstructions = (tree: ReactionTreeNode): string[] => {
    const instructions: string[] = [];
    const moonMaterials: { name: string; qty: number; price: string }[] = [];
    const buyMaterials: { name: string; qty: number; price: string }[] = [];
    const reactions: { output: string; inputs: string[]; qty: number }[] = [];

    // Recursively collect all materials and reactions
    const collectFromTree = (node: ReactionTreeNode) => {
      if (node.source === "moon") {
        moonMaterials.push({ name: node.name, qty: node.quantity, price: formatIsk(node.total_price) });
      } else if (node.source === "buy") {
        buyMaterials.push({ name: node.name, qty: node.quantity, price: formatIsk(node.total_price) });
      } else if (node.source === "react" || node.source === "output") {
        if (node.children.length > 0) {
          reactions.push({
            output: node.name,
            inputs: node.children.map(c => `${c.name} x${c.quantity.toLocaleString()}`),
            qty: node.quantity
          });
        }
      }
      node.children.forEach(collectFromTree);
    };

    collectFromTree(tree);

    // Build instructions
    if (moonMaterials.length > 0) {
      instructions.push("EXTRACT FROM MOONS:");
      moonMaterials.forEach(m => {
        instructions.push(`  ${m.name} x${m.qty.toLocaleString()} (worth ${m.price})`);
      });
      instructions.push("");
    }

    if (buyMaterials.length > 0) {
      instructions.push("PURCHASE:");
      buyMaterials.forEach(m => {
        instructions.push(`  ${m.name} x${m.qty.toLocaleString()} for ${m.price}`);
      });
      instructions.push("");
    }

    if (reactions.length > 0) {
      instructions.push("REACT (in order):");
      // Reverse so we show from base reactions up to final product
      [...reactions].reverse().forEach((r, i) => {
        instructions.push(`  ${i + 1}. ${r.inputs.join(" + ")}`);
        instructions.push(`     -> ${r.output} x${r.qty.toLocaleString()}`);
      });
      instructions.push("");
    }

    instructions.push(`SELL: ${tree.name} x${tree.quantity.toLocaleString()} for ${formatIsk(tree.total_price)}`);

    return instructions;
  };

  return (
    <div className="app">
      <img src="/logo.png" alt="MOON" className="app-logo" />

      {/* Tab Bar */}
      <div className="tab-bar">
        {tabs.map((tab) => (
          <div
            key={tab.id}
            className={`tab ${activeTab === tab.id ? "active" : ""}`}
            onClick={() => setActiveTab(tab.id)}
          >
            <span className="tab-name">{tab.name}</span>
            {tab.id !== "home" && (
              <button
                className="tab-close"
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseTab(tab.id);
                }}
              >
                ✕
              </button>
            )}
          </div>
        ))}
      </div>

      <div className="separator" />

      {/* Tab Content */}
      {activeTab === "home" && (
        <div className="home-content">
          {/* Input Section */}
          <div className="input-section">
            <label className="input-label">Paste moon scan data:</label>
            <div className="input-row">
              <textarea
                className="moon-input"
                value={inputText}
                onChange={(e) => setInputText(e.target.value)}
                placeholder="Paste moon scan data here..."
                rows={8}
              />
              <button
                className="go-button"
                onClick={handleGo}
                disabled={isAnalyzing}
              >
                {isAnalyzing ? "..." : "GO"}
              </button>
            </div>
          </div>

          {/* Action Buttons */}
          <div className="action-section">
            <button className="add-moon-button" onClick={handleAddMoon}>
              Add Moon
            </button>
            {errorMessage && (
              <div className="error-message">{errorMessage}</div>
            )}
          </div>

          {/* Dual Panel Layout */}
          <div className="panels-container">
            {/* Loaded Moons Panel */}
            <div className="moons-panel">
              <h2 className="panel-heading">Loaded Moons ({moons.length})</h2>
              <div className="moons-list">
                {moons.map((moon, index) => (
                  <div key={moon.name} className="moon-card">
                    <div className="moon-header">
                      <span className="moon-name">
                        {index + 1}. {moon.name}
                      </span>
                      <button
                        className="delete-button"
                        onClick={() => handleDeleteMoon(index)}
                      >
                        ✕
                      </button>
                    </div>
                    <div className="materials-list">
                      {moon.materials.map((material) => (
                        <div key={material.name} className="material-item">
                          • {material.name}:{" "}
                          {(material.quantity * 100).toFixed(2)}%
                        </div>
                      ))}
                    </div>
                  </div>
                ))}
                {moons.length === 0 && (
                  <div className="empty-message">No moons loaded yet</div>
                )}
              </div>
            </div>

            {/* Unique Materials Panel */}
            <div className="materials-panel">
              <h2 className="panel-heading">
                Unique Materials ({uniqueMaterials.length})
              </h2>
              <div className="unique-materials-list">
                {uniqueMaterials.map((material, index) => (
                  <span key={material} className="unique-material">
                    {index > 0 && " • "}
                    {material}
                  </span>
                ))}
                {uniqueMaterials.length === 0 && (
                  <div className="empty-message">No materials yet</div>
                )}
              </div>
            </div>
          </div>

          <div className="recruiting-link-wrapper">
            <a
              href="https://www.illuminatedcorp.com"
              target="_blank"
              rel="noopener noreferrer"
              className="recruiting-link"
            >
              ILLUMINATED IS RECRUITING
            </a>
          </div>
        </div>
      )}

      {/* Analysis Results Tab */}
      {activeTab !== "home" && activeTabData?.results && (
        <div className="results-content">
          <div className="results-header-row">
            <h2 className="results-heading">
              Profitable Reactions ({activeTabData.results.length})
            </h2>
            <div className="expand-buttons">
              <button
                className="expand-btn"
                onClick={() => {
                  const allIds = new Set(activeTabData.results!.map(r => r.formula_id));
                  setExpandedReactions(allIds);
                }}
              >
                Expand All
              </button>
              <button
                className="expand-btn"
                onClick={() => setExpandedReactions(new Set())}
              >
                Collapse All
              </button>
            </div>
          </div>

          {/* Column Headers */}
          <div className="results-table-header">
            <div className="header-cell name sortable" onClick={() => handleSort("output_name")}>
              Reaction <SortIcon field="output_name" />
            </div>
            <div className="header-cell sortable" onClick={() => handleSort("output_quantity")}>
              Qty <SortIcon field="output_quantity" />
            </div>
            <div className="header-cell sortable" onClick={() => handleSort("input_cost")}>
              Input Cost <SortIcon field="input_cost" />
            </div>
            <div className="header-cell sortable" onClick={() => handleSort("output_value")}>
              Output Value <SortIcon field="output_value" />
            </div>
            <div className="header-cell sortable" onClick={() => handleSort("profit")}>
              Profit <SortIcon field="profit" />
            </div>
            <div className="header-cell sortable" onClick={() => handleSort("margin")}>
              Margin <SortIcon field="margin" />
            </div>
            <div className="header-cell expand"></div>
          </div>

          <div className="results-list">
            {getSortedResults(activeTabData.results).map((result) => {
              const isExpanded = expandedReactions.has(result.formula_id);
              return (
                <div
                  key={result.formula_id}
                  className={`result-card ${result.profit > 0 ? "profitable" : "unprofitable"} ${isExpanded ? "expanded" : ""}`}
                >
                  <div
                    className="result-header"
                    onClick={() => toggleReactionExpanded(result.formula_id)}
                  >
                    <div className="result-cell name">
                      <div className="result-name">{result.output_name}</div>
                      <div className="result-formula">{result.formula_name}</div>
                    </div>
                    <div className="result-cell">{result.output_quantity.toLocaleString()}</div>
                    <div className="result-cell">{formatIsk(result.input_cost)}</div>
                    <div className="result-cell">{formatIsk(result.output_value)}</div>
                    <div className={`result-cell ${result.profit > 0 ? "positive" : "negative"}`}>
                      {formatIsk(result.profit)}
                    </div>
                    <div className={`result-cell ${result.margin > 0 ? "positive" : "negative"}`}>
                      {result.margin.toFixed(1)}%
                    </div>
                    <div className="result-cell expand">{isExpanded ? "▼" : "▶"}</div>
                  </div>

                  {isExpanded && (
                    <div className="result-details">
                      <div className="details-section tree-section">
                        <div className="detail-tabs">
                          <button
                            className={`detail-tab ${getDetailTab(result.formula_id) === "graph" ? "active" : ""}`}
                            onClick={(e) => { e.stopPropagation(); setDetailTab(result.formula_id, "graph"); }}
                          >
                            Graph View
                          </button>
                          <button
                            className={`detail-tab ${getDetailTab(result.formula_id) === "text" ? "active" : ""}`}
                            onClick={(e) => { e.stopPropagation(); setDetailTab(result.formula_id, "text"); }}
                          >
                            Instructions
                          </button>
                        </div>

                        {getDetailTab(result.formula_id) === "graph" && result.reaction_tree && (
                          <>
                            <div className="zoom-hint">Ctrl + Scroll to zoom</div>
                            <ReactionTreeGraph tree={result.reaction_tree} formatIsk={formatIsk} />
                          </>
                        )}

                        {getDetailTab(result.formula_id) === "text" && result.reaction_tree && (
                          <div className="text-instructions">
                            {generateTextInstructions(result.reaction_tree).map((line, i) => (
                              <div key={i} className={line === "" ? "instruction-spacer" : line.startsWith("  ") ? "instruction-item" : "instruction-header"}>
                                {line}
                              </div>
                            ))}
                          </div>
                        )}
                      </div>

                      <div className="profit-summary">
                        <span>Profit per run:</span>
                        <span className={result.profit > 0 ? "positive" : "negative"}>
                          {formatIsk(result.profit)} ({result.margin.toFixed(1)}%)
                        </span>
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
            {activeTabData.results.length === 0 && (
              <div className="empty-message">No profitable reactions found</div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
