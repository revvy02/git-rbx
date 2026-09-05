--[[
	Pure tree construction after Rojo's PatchTree: nodes hang off their
	parents, siblings sort alphabetically by name at every level, and the
	tree flattens to positioned rows for the virtual list. Expansion state is
	owned by the caller and passed in per flatten.
]]

local types = require("@shared/ui/PatchVisualizer/types")

type Id = types.Id
type Patch = types.Patch
type PatchNode = types.PatchNode
type ChangeRow = types.ChangeRow
type Row = types.Row

local patchTree = {}

local KIND_RANK = { Add = 0, Remove = 0, Reparent = 1, Pivot = 2, Edit = 3 }

-- Marker precedence when a node carries several kinds of change.
local function deriveKind(input: types.PatchNodeInput): types.NodeKind?
	if input.kind then
		return input.kind
	end
	local best: types.NodeKind? = nil
	for _, row in input.rows or {} do
		local kind: types.NodeKind = if row.kind == "Add" then "Edit" else row.kind
		if not best or KIND_RANK[kind] < KIND_RANK[best] then
			best = kind
		end
	end
	return best
end

local function sortRows(rows: { ChangeRow }): { ChangeRow }
	local sorted = table.clone(rows)
	table.sort(sorted, function(a, b)
		return a.property < b.property
	end)
	return sorted
end

function patchTree.build(patch: Patch): { PatchNode }
	local nodes: { [Id]: PatchNode } = {}
	for id, input in patch.nodes do
		nodes[id] = {
			id = id,
			name = input.name,
			className = input.className,
			instance = input.instance,
			kind = deriveKind(input),
			rows = sortRows(input.rows or {}),
			subtreeCount = input.subtreeCount,
			warning = input.warning,
			children = {},
			depth = 1,
			isFinalChild = true,
		}
	end

	local roots: { PatchNode } = {}
	for id, node in nodes do
		local parentId = patch.nodes[id].parent
		local parent = if parentId ~= nil then nodes[parentId] else nil
		if parent then
			node.parent = parent
			table.insert(parent.children, node)
		else
			table.insert(roots, node)
		end
	end

	local function finish(list: { PatchNode }, depth: number)
		table.sort(list, function(a, b)
			if a.name == b.name then
				return tostring(a.id) < tostring(b.id)
			end
			return a.name < b.name
		end)
		for index, node in list do
			node.depth = depth
			node.isFinalChild = index == #list
			finish(node.children, depth + 1)
		end
	end
	finish(roots, 1)
	return roots
end

function patchTree.count(roots: { PatchNode }): number
	local total = 0
	local function visit(node: PatchNode)
		total += 1
		for _, child in node.children do
			visit(child)
		end
	end
	for _, root in roots do
		visit(root)
	end
	return total
end

-- Flatten to rows. `isExpanded(node)` decides whether a node's change list
-- is shown as a header row plus one row per change. `depthsComplete` is a
-- snapshot per row so line guides know which ancestor columns are finished.
function patchTree.flatten(roots: { PatchNode }, rowHeight: number, isExpanded: (PatchNode) -> boolean): { Row }
	local rows: { Row } = {}
	local y = 0
	local depthsComplete: { [number]: boolean } = {}
	local total = patchTree.count(roots)
	local seen = 0

	local function push(kind: types.RowKind, node: PatchNode, change: ChangeRow?)
		table.insert(rows, {
			y = y,
			h = rowHeight,
			kind = kind,
			index = #rows + 1,
			node = node,
			change = change,
			isFinalElement = seen == total,
			depthsComplete = table.clone(depthsComplete),
		})
		y += rowHeight
	end

	local function visit(node: PatchNode)
		seen += 1
		depthsComplete[node.depth] = false
		for i = node.depth + 1, #depthsComplete do
			depthsComplete[i] = nil
		end
		push("node", node, nil)
		if node.isFinalChild then
			depthsComplete[node.depth] = true
		end
		if #node.rows > 0 and isExpanded(node) then
			push("header", node, nil)
			for _, change in node.rows do
				push("change", node, change)
			end
		end
		for _, child in node.children do
			visit(child)
		end
	end
	for _, root in roots do
		visit(root)
	end
	return rows
end

return patchTree
