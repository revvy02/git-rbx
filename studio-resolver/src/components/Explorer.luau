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

local Element = require("../packages").miu.Instance

local TreeRow = require("./TreeRow")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

local cts = require("../ecs/cts")
local mirror = require("../ecs/systems/mirror")
local world = require("../ecs/world")

type Conflict = state.Conflict
type SideInfo = state.SideInfo

type Row = {
	inst: Instance,
	label: string,
	className: string,
	depth: number,
	hasChildren: boolean,
	isExpanded: boolean,
}

local expanded: { [Instance]: boolean } = {}
local seeded = false
local lastRevealed: Instance? = nil

local function seedExpansion()
	if seeded then
		return
	end
	seeded = true
	for _, conflict in state.conflicts do
		local node = conflict.target.Parent
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
	local inst = state.selectedInstance
	if inst == nil or inst == lastRevealed then
		return
	end
	lastRevealed = inst
	local node = inst.Parent
	while node and node ~= game do
		expanded[node] = true
		node = node.Parent
	end
end

-- Model previews root at the imported content; places root at Workspace plus
-- any service holding a conflict target.
local function treeRoots(): { Instance }
	local previewFolder = workspace:FindFirstChild("RbxDiffModelPreview")
	if previewFolder then
		return previewFolder:GetChildren()
	end
	local roots: { Instance } = { workspace }
	local seen: { [Instance]: boolean } = { [workspace :: Instance] = true }
	for _, conflict in state.conflicts do
		local top: Instance = conflict.target
		while top.Parent and top.Parent ~= game do
			top = top.Parent :: Instance
		end
		if not seen[top] then
			seen[top] = true
			table.insert(roots, top)
		end
	end
	return roots
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

-- RESULT: the live preview DOM, read through the jecs mirror — syncTree
-- frame-diffs instances into entities with ChildOf pairs, and the walk here
-- is pure entity traversal. The diff layers will hang their components off
-- these entities.
local function resultRows(): { Row }
	seedExpansion()
	revealSelected()

	local roots = treeRoots()
	mirror.syncTree(roots)

	local rows: { Row } = {}
	local function visit(entity: any, depth: number)
		local inst = world:get(entity, cts.Ref) :: Instance
		local children = mirror.childrenOf(entity)
		local isExpanded = expanded[inst] == true
		table.insert(rows, instanceRow(inst, depth, #children > 0, isExpanded))
		if isExpanded then
			for _, child in children do
				visit(child, depth + 1)
			end
		end
	end
	for _, root in roots do
		visit(mirror.entityFor(root), 0)
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

	for _, conflict in state.conflicts do
		local content: Instance?
		local sideParent: Instance?
		if tab == "base" then
			content = conflict.target
			sideParent = if conflict.kind == "MoveTarget" then conflict.baseParent else conflict.target.Parent
		else
			local info = (if tab == "ours" then conflict.ours else conflict.theirs) :: SideInfo
			content = info.clone
			sideParent = if conflict.kind == "MoveTarget"
				then (info.destination or conflict.baseParent)
				else conflict.target.Parent
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
		return state.OURS_COLOR
	elseif tab == "theirs" then
		return state.THEIRS_COLOR
	elseif tab == "base" then
		return theme.textMuted
	end
	return theme.text
end

local function TabBar()
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
			local isActive = state.viewTab == tab
			Element("TextButton", {
				id = tab,
				Text = tab:upper(),
				Size = UDim2.new(0.25, -8, 1, 0),
				tags = { "corner-sm", "vs-border" },
				BackgroundColor3 = if isActive then theme.selection else theme.surface,
				TextColor3 = if isActive then theme.textBright else tabColor(tab),
				TextSize = 11,
				Font = theme.font,
				events = {
					Activated = function()
						state.viewTab = tab
					end,
				},
			})
		end
	end)
end

-- Ancestors of the selection light up as on-path.
local function ancestorSet(): { [Instance]: boolean }
	local set: { [Instance]: boolean } = {}
	local node = state.selectedInstance
	node = node and node.Parent
	while node and node ~= game do
		set[node] = true
		node = node.Parent
	end
	return set
end

local function renderRows(rows: { Row })
	local onPath = ancestorSet()
	for index, row in rows do
		local inst = row.inst
		TreeRow({
			id = `row{index}`,
			layoutOrder = index,
			depth = row.depth,
			label = row.label,
			className = row.className,
			hasChildren = row.hasChildren,
			expanded = row.isExpanded,
			selected = inst == state.selectedInstance,
			onPath = onPath[inst] == true,
			stripe = index % 2 == 0,
			onToggle = if row.hasChildren
				then function()
					expanded[inst] = not row.isExpanded
				end
				else nil,
			onActivated = function()
				state.selectedInstance = inst
			end,
		})
	end
end

local function Explorer()
	Element("ScrollingFrame", {
		id = "explorer",
		Size = UDim2.new(1, 0, 0.5, 0),
		tags = { "bg-none", "flex-col" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "MERGE EXPLORER",
			Size = UDim2.new(1, 0, 0, 24),
			BackgroundColor3 = theme.surfaceHigh,
			TextColor3 = theme.textMuted,
			TextSize = theme.headerSize,
			Font = theme.font,
		})

		if state.finished then
			return
		end

		TabBar()

		renderRows(if state.viewTab == "result" then resultRows() else sideRows(state.viewTab))
	end)
end

return Explorer
