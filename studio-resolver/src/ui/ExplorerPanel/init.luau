--[[
	Plain explorer over the merge's four states, tabbed BASE | OURS | THEIRS
	| RESULT. No diff presentation here yet (badges, tints, phantoms) — this
	is a normal explorer per state; the diff layers come back iteratively.

	RESULT walks the live preview DOM. The side tabs reconstruct that side's
	world from what the stamped state can support: ancestor chains as a
	skeleton with the side's version of each contested subtree grafted at its
	position in that side — a moved instance sits under that side's parent,
	and content a side deletes simply isn't there. Content is the stamped
	Ours/Theirs clones (the target itself for BASE).

	Conflict ancestor chains start expanded in RESULT and side trees start
	fully expanded; collapses stick either way.
]]

local packages = require("../packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local TreeRow = require("@self/TreeRow")

local format = require("../library/format")
local theme = require("../library/theme")

local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")

type SideInfo = cts.SideInfo
type Base = cts.Base

type Row = {
	inst: Instance,
	label: string,
	className: string,
	depth: number,
	hasChildren: boolean,
	isExpanded: boolean,
}

local expanded: { [Instance]: boolean } = {}
local expansionGen = 0
local seeded = false
local lastRevealed: Instance? = nil

local function setExpanded(inst: Instance, isExpanded: boolean)
	expanded[inst] = isExpanded
	expansionGen += 1
end

local function seedExpansion()
	if seeded then
		return
	end
	seeded = true
	expansionGen += 1
	for _, target in world:query(cts.Target) do
		local node = (target :: Instance).Parent
		while node and node ~= game do
			expanded[node] = true
			node = node.Parent
		end
	end
end

-- Expand the selected instance's ancestor chain when the selection changes
-- (e.g. clicked from the conflict list) — but only then, so a manual
-- collapse afterwards sticks.
local function revealSelected()
	local inst = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	if inst == nil or inst == lastRevealed then
		return
	end
	lastRevealed = inst
	expansionGen += 1
	local node = inst.Parent
	while node and node ~= game do
		expanded[node] = true
		node = node.Parent
	end
end

-- Session artifacts stay out of the tree.
local function visibleChildren(inst: Instance): { Instance }
	local out: { Instance } = {}
	for _, child in inst:GetChildren() do
		if child.ClassName ~= "Highlight" and child.Name ~= "__RbxDiffMerge" then
			table.insert(out, child)
		end
	end
	table.sort(out, function(a: Instance, b: Instance)
		return a.Name < b.Name
	end)
	return out
end

local function instanceRow(inst: Instance, depth: number, hasChildren: boolean, isExpanded: boolean): Row
	return {
		inst = inst,
		label = `{format.escapeRich(inst.Name)}  <font color="{theme.textDimHex}">{inst.ClassName}</font>`,
		className = inst.ClassName,
		depth = depth,
		hasChildren = hasChildren,
		isExpanded = isExpanded,
	}
end

-- RESULT: the live preview DOM, read through the mirror's world data — the
-- walk is pure component traversal (Roots singleton + per-node Children
-- arrays the mirror system maintains). The diff layers will hang their
-- components off these entities.
local function resultRows(): { Row }
	local rows: { Row } = {}
	local function visit(entity: any, depth: number)
		local inst = world:get(entity, cts.Ref) :: Instance
		local children = (world:get(entity, cts.Children) :: { any }?) or {}
		local isExpanded = expanded[inst] == true
		table.insert(rows, instanceRow(inst, depth, #children > 0, isExpanded))
		if isExpanded then
			for _, child in children do
				visit(child, depth + 1)
			end
		end
	end
	for _, root in (world:get(cts.Roots, cts.Roots) :: { any }?) or {} do
		visit(root, 0)
	end
	return rows
end

-- BASE / OURS / THEIRS: that side's world — ancestor skeleton with the
-- side's version of each contested subtree grafted at its position in that
-- side. Content a side deletes simply is not in its tree.
local function sideRows(tab: string): { Row }
	type SideNode = { inst: Instance, children: { SideNode }, grafts: { Instance } }
	local nodes: { [Instance]: SideNode } = {}
	local roots: { SideNode } = {}

	local function nodeFor(inst: Instance): SideNode
		local existing = nodes[inst]
		if existing then
			return existing
		end
		local node: SideNode = { inst = inst, children = {}, grafts = {} }
		nodes[inst] = node
		local parent = inst.Parent
		if parent and parent ~= game and parent ~= workspace then
			table.insert(nodeFor(parent).children, node)
		else
			table.insert(roots, node)
		end
		return node
	end

	for _, entity in conflicts.list() do
		local kind = world:get(entity, cts.Kind)
		local target = world:get(entity, cts.Target) :: Instance
		local base = world:get(entity, cts.Base) :: Base
		local content: Instance?
		local sideParent: Instance?
		if tab == "base" then
			content = target
			sideParent = if kind == "MoveTarget" then base.parent else target.Parent
		else
			local info = (
				if tab == "ours" then world:get(entity, cts.Ours) else world:get(entity, cts.Theirs)
			) :: SideInfo
			content = info.clone
			sideParent = if kind == "MoveTarget" then (info.destination or base.parent) else target.Parent
		end
		if content and sideParent then
			table.insert(nodeFor(sideParent).grafts, content)
		end
	end

	local rows: { Row } = {}

	local function visitContent(inst: Instance, depth: number)
		local children = visibleChildren(inst)
		local isExpanded = expanded[inst] ~= false
		table.insert(rows, instanceRow(inst, depth, #children > 0, isExpanded))
		if isExpanded then
			for _, child in children do
				visitContent(child, depth + 1)
			end
		end
	end

	local function visitNode(node: SideNode, depth: number)
		local isExpanded = expanded[node.inst] ~= false
		table.insert(rows, instanceRow(node.inst, depth, true, isExpanded))
		if not isExpanded then
			return
		end
		for _, graft in node.grafts do
			visitContent(graft, depth + 1)
		end
		table.sort(node.children, function(a: SideNode, b: SideNode)
			return a.inst.Name < b.inst.Name
		end)
		for _, child in node.children do
			visitNode(child, depth + 1)
		end
	end

	for _, root in roots do
		visitNode(root, 0)
	end
	return rows
end

local TABS = { "base", "ours", "theirs", "result" }

local function tabColor(tab: string): Color3
	if tab == "ours" then
		return theme.ours
	elseif tab == "theirs" then
		return theme.theirs
	elseif tab == "base" then
		return theme.textMuted
	end
	return theme.text
end

local function TabBar()
	local viewTab = world:get(cts.ViewTab, cts.ViewTab)
	Element("Frame", {
		id = "tabs",
		Size = UDim2.new(1, 0, 0, 26),
		tags = { "bg-none", "flex-row", "gap-xs" },
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4),
			PaddingRight = UDim.new(0, 4),
			PaddingTop = UDim.new(0, 3),
			PaddingBottom = UDim.new(0, 3),
		})
		for _, tab in TABS do
			local isActive = viewTab == tab
			Element("TextButton", {
				id = tab,
				Text = tab:upper(),
				Size = UDim2.new(0.25, -8, 1, 0),
				tags = { "corner-sm", "vs-border", "button" },
				BackgroundColor3 = if isActive then theme.selection else theme.surface,
				TextColor3 = if isActive then theme.textBright else tabColor(tab),
				TextSize = 11,
				events = {
					Activated = function()
						world:set(cts.ViewTab, cts.ViewTab, tab)
					end,
				},
			})
		end
	end)
end

-- Ancestors of the selection light up as on-path.
local function ancestorSet(): { [Instance]: boolean }
	local set: { [Instance]: boolean } = {}
	local node = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	node = node and node.Parent
	while node and node ~= game do
		set[node] = true
		node = node.Parent
	end
	return set
end

local ROW_HEIGHT = 22
local HEADER_HEIGHT = 24
local TABS_HEIGHT = 26

local rowsCacheKey: string? = nil
local rowsCache: { Row } = {}

-- Rows render through VirtualList: only the visible index range builds
-- TreeRows, so a fully-expanded real place stays cheap.
local function renderRows(rows: { Row })
	local onPath = ancestorSet()
	local selectedInstance = world:get(cts.SelectedInstance, cts.SelectedInstance) :: Instance?
	VirtualList({
		id = "rows",
		data = rows,
		dimensions = Dimensions.consistentSize(ROW_HEIGHT),
		direction = "y",
		Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT - TABS_HEIGHT),
		tags = { "bg-none" },
	}, function(row: Row, index: number)
		local inst = row.inst
		TreeRow({
			id = "row",
			depth = row.depth,
			label = row.label,
			className = row.className,
			hasChildren = row.hasChildren,
			expanded = row.isExpanded,
			selected = inst == selectedInstance,
			onPath = onPath[inst] == true,
			stripe = index % 2 == 0,
			onToggle = if row.hasChildren
				then function()
					setExpanded(inst, not row.isExpanded)
				end
				else nil,
			onActivated = function()
				world:set(cts.SelectedInstance, cts.SelectedInstance, inst)
			end,
		})
	end)
end

local function ExplorerPanel()
	Element("Frame", {
		id = "explorer",
		Size = UDim2.new(1, 0, 0.5, 0),
		tags = { "bg-none", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "MERGE EXPLORER",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		if world:has(cts.Finished, cts.Finished) then
			return
		end

		TabBar()

		seedExpansion()
		revealSelected()

		-- Rows are derived data: rebuilt only when the tree (MirrorGeneration),
		-- expansion, or tab changes. Selection/on-path stay out of the key —
		-- renderRows applies them per frame over the cached array.
		local viewTab = world:get(cts.ViewTab, cts.ViewTab) :: string
		local mirrorGen = (world:get(cts.MirrorGeneration, cts.MirrorGeneration) :: number?) or 0
		local key = `{viewTab}:{expansionGen}:{mirrorGen}`
		if key ~= rowsCacheKey then
			rowsCacheKey = key
			debug.profilebegin("rbxdiff_explorer_rows")
			rowsCache = if viewTab == "result" then resultRows() else sideRows(viewTab)
			debug.profileend()
		end
		renderRows(rowsCache)
	end)
end

return ExplorerPanel
