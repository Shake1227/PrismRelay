import { useI18n } from "../i18n";
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
  const { t } = useI18n();
  const [expanded, setExpanded] = useState(depth < 2);
  const input = useRef<HTMLInputElement>(null);
  const state = selectionState(node.ids, selected);
  useEffect(() => {
    if (input.current) input.current.indeterminate = state.indeterminate;
  }, [state.indeterminate]);
  const branch = node.children.length > 0;
  const label = t(node.label);
  const isOpen = searching || expanded;
  return (
    <li className={`tree-node depth-${depth}`}>
      <div
        className={`tree-row ${branch ? "branch" : "leaf"}`}
        style={{
          paddingLeft: `${16 + depth * 19}px`,
        }}
      >
        <button
          className={`tree-chevron ${branch ? "" : "hidden"}`}
          aria-label={t("{0} を{1}", {
            "0": label,
            "1": isOpen ? t("折りたたむ") : t("展開"),
          })}
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
          aria-label={t("{0}{1} を選択", {
            "0": node.setting?.group ? `${t(node.setting.group)} · ` : "",
            "1": label,
          })}
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
          <span>{label}</span>
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
  const { t } = useI18n();
  const [search, setSearch] = useState("");
  const roots = buildTree(settings, search, t);
  return (
    <div className="tree-picker">
      <div className="tree-toolbar">
        <div className="search-field">
          <Search size={16} />
          <input
            aria-label={t("設定を検索")}
            placeholder={t("設定を検索…")}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
          <kbd>⌕</kbd>
        </div>
        <span>
          {selected.size} {t("項目を選択")}
        </span>
      </div>
      <ul className="tree-root" aria-label={t("共有する設定")}>
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
              ? t("一致する設定が見つかりません")
              : t("共有できる設定が見つかりません")}
          </p>
        </div>
      )}
      <div className="tree-footer">
        <span>{t("検出した設定のみを表示")}</span>
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
          {selected.size ? t("選択を解除") : t("すべて選択")}
        </button>
      </div>
    </div>
  );
}
