import { useEffect, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  Moon,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import type { Setting } from "../models";
import { buildTree, selectionState, toggleSelection } from "../utils/tree";
import type { SettingNode } from "../utils/tree";
import { formatValue } from "../utils/format";

function Node({
  node,
  selected,
  onChange,
  depth,
  searching,
}: {
  node: SettingNode;
  selected: Set<string>;
  onChange: (selected: Set<string>) => void;
  depth: number;
  searching: boolean;
}) {
  const [expanded, setExpanded] = useState(depth < 2);
  const input = useRef<HTMLInputElement>(null);
  const state = selectionState(node.ids, selected);
  useEffect(() => {
    if (input.current) input.current.indeterminate = state.indeterminate;
  }, [state.indeterminate]);
  const branch = node.children.length > 0;
  const isOpen = searching || expanded;
  return (
    <li className={`tree-node depth-${depth}`}>
      <div
        className={`tree-row ${branch ? "branch" : "leaf"}`}
        style={{ paddingLeft: `${16 + depth * 19}px` }}
      >
        <button
          className={`tree-chevron ${branch ? "" : "hidden"}`}
          aria-label={`${node.label} を${isOpen ? "折りたたむ" : "展開"}`}
          aria-expanded={branch ? isOpen : undefined}
          onClick={() => setExpanded(!expanded)}
          tabIndex={branch ? 0 : -1}
        >
          {isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />}
        </button>
        <input
          ref={input}
          type="checkbox"
          checked={state.checked}
          aria-label={`${node.setting?.group ? `${node.setting.group} · ` : ""}${node.label} を選択`}
          onChange={() => onChange(toggleSelection(node.ids, selected))}
        />
        <button
          className="tree-label"
          aria-expanded={branch ? isOpen : undefined}
          onClick={() =>
            branch
              ? setExpanded(!expanded)
              : onChange(toggleSelection(node.ids, selected))
          }
        >
          {depth === 0 &&
            (node.label === "Lunar Client" ? (
              <Moon size={17} />
            ) : (
              <SlidersHorizontal size={17} />
            ))}
          <span>{node.label}</span>
        </button>
        {branch ? (
          <span className="tree-count">
            {node.ids.filter((id) => selected.has(id)).length}
            <span> / {node.ids.length}</span>
          </span>
        ) : (
          <span className="tree-value" title={formatValue(node.setting?.value)}>
            {formatValue(node.setting?.value)}
          </span>
        )}
      </div>
      {branch && isOpen && (
        <ul>
          {node.children.map((child) => (
            <Node
              key={child.key}
              node={child}
              selected={selected}
              onChange={onChange}
              depth={depth + 1}
              searching={searching}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

export function TreePicker({
  settings,
  selected,
  onChange,
}: {
  settings: Setting[];
  selected: Set<string>;
  onChange: (selected: Set<string>) => void;
}) {
  const [search, setSearch] = useState("");
  const roots = buildTree(settings, search);
  return (
    <div className="tree-picker">
      <div className="tree-toolbar">
        <div className="search-field">
          <Search size={16} />
          <input
            aria-label="設定を検索"
            placeholder="設定を検索…"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <kbd>⌕</kbd>
        </div>
        <span>{selected.size} 項目を選択</span>
      </div>
      <ul className="tree-root" aria-label="共有する設定">
        {roots.map((root) => (
          <Node
            key={root.key}
            node={root}
            selected={selected}
            onChange={onChange}
            depth={0}
            searching={Boolean(search.trim())}
          />
        ))}
      </ul>
      {roots.length === 0 && (
        <div className="empty-state small">
          <Search size={24} />
          <p>
            {settings.length
              ? "一致する設定が見つかりません"
              : "共有できる設定が見つかりません"}
          </p>
        </div>
      )}
      <div className="tree-footer">
        <span>検出した設定のみを表示</span>
        <button
          className="text-button"
          onClick={() =>
            onChange(
              selected.size
                ? new Set()
                : new Set(settings.map((setting) => setting.id)),
            )
          }
        >
          {selected.size ? "選択を解除" : "すべて選択"}
        </button>
      </div>
    </div>
  );
}
