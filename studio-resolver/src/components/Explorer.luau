--[[
	Explorer over the merge, with a view-tab bar: BASE | OURS | THEIRS | RESULT.

	RESULT is the full preview DOM — expandable 22px rows with carets, class
	icons, dim class suffixes, conflict rows tinted and badged by status,
	#094771 selection, and MoveTarget phantoms at candidate parents (clicking
	one stages that side, "base" = unpick).

	The side tabs are conflict-scoped trees of that side's WORLD: ancestor
	chains with the side's version of each contested subtree grafted at its
	position in that side — so a moved instance simply sits under that side's
	destination parent, base sits under the base parent, and a side that
	deletes the target shows a placeholder where it would have been. Content
	comes from the stamped Ours/Theirs clones (the target itself for BASE,
	whose contested property value the properties pane overrides from the
	boot snapshot).

	Conflict ancestor chains start expanded in RESULT and side trees start
	fully expanded; collapses stick either way.
]]

local Element = require("../miu/Instance")

local TreeRow = require("./TreeRow")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

type Conflict = state.Conflict
type SideInfo = state.SideInfo

type Row = {
	inst: Instance?,
	label: string,
	className: string?, -- drives the class icon (phantoms use their target's)
	phantom: boolean,
	depth: number,
	color: Color3,
	conflict: Conflict?, -- clicking selects this conflict
	stageSide: string?, -- phantom rows: clicking stages this side ("base" = unpick)
	hasChildren: boolean,
	isExpanded: boolean,
	tint: Color3?, -- row background tint (conflict state)
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

local function conflictsByTarget(): { [Instance]: Conflict }
	local map: { [Instance]: Conflict } = {}
	for _, conflict in state.conflicts do
		map[conflict.target] = conflict
	end
	return map
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

local function instanceRow(
	inst: Instance,
	depth: number,
	conflict: Conflict?,
	hasExtra: boolean,
	isExpanded: boolean
): Row
	local name = format.escapeRich(inst.Name)
	return {
		inst = inst,
		label = `{name}  <font color="{theme.textDimHex}">{inst.ClassName}</font>`,
		className = inst.ClassName,
		phantom = false,
		depth = depth,
		color = if conflict then state.statusColor(conflict) else theme.text,
		conflict = conflict,
		hasChildren = hasExtra,
		isExpanded = isExpanded,
		tint = if conflict then state.statusColor(conflict) else nil,
	}
end

-- RESULT: the live preview DOM. Staging reparents/renames targets, so the
-- tree follows the staged state for free.
local function resultRows(): { Row }
	seedExpansion()
	revealSelected()

	local byTarget = conflictsByTarget()

	local phantoms: { [Instance]: { { conflict: Conflict, side: string } } } = {}
	for _, conflict in state.conflicts do
		if conflict.kind == "MoveTarget" then
			local candidates: { [string]: Instance? } = {
				ours = (conflict.ours :: SideInfo).destination,
				theirs = (conflict.theirs :: SideInfo).destination,
				base = conflict.baseParent,
			}
			for side, parent in candidates :: { [string]: Instance } do
				if parent ~= conflict.target.Parent then
					local list = phantoms[parent]
					if not list then
						list = {}
						phantoms[parent] = list
					end
					table.insert(list, { conflict = conflict, side = side })
				end
			end
		end
	end

	local rows: { Row } = {}
	local function visit(inst: Instance, depth: number)
		local children = visibleChildren(inst)
		local conflict = byTarget[inst]
		local isExpanded = expanded[inst] == true
		table.insert(rows, instanceRow(inst, depth, conflict, #children > 0 or phantoms[inst] ~= nil, isExpanded))
		if isExpanded then
			for _, phantom in phantoms[inst] or {} do
				local sideColor = if phantom.side == "ours"
					then state.OURS_COLOR
					elseif phantom.side == "theirs" then state.THEIRS_COLOR
					else theme.textDim
				table.insert(rows, {
					inst = nil,
					label = `{format.escapeRich(phantom.conflict.target.Name)} ({phantom.side})`,
					className = phantom.conflict.target.ClassName,
					phantom = true,
					depth = depth + 1,
					color = sideColor,
					conflict = phantom.conflict,
					stageSide = phantom.side,
					hasChildren = false,
					isExpanded = false,
				})
			end
			for _, child in children do
				visit(child, depth + 1)
			end
		end
	end
	for _, root in treeRoots() do
		visit(root, 0)
	end
	return rows
end

-- BASE / OURS / THEIRS: a conflict-scoped tree of that side's world —
-- ancestor chains (default-expanded skeleton) with the side's version of
-- each contested subtree grafted at its position IN THAT SIDE, so location
-- is conveyed structurally: a move simply sits under that side's destination.
local function sideRows(tab: string): { Row }
	type SideNode = { inst: Instance, children: { SideNode }, grafts: { { conflict: Conflict, content: Instance? } } }
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
		if sideParent then
			table.insert(nodeFor(sideParent).grafts, { conflict = conflict, content = content })
		end
	end

	local rows: { Row } = {}

	local function visitContent(inst: Instance, depth: number, conflict: Conflict?)
		local children = visibleChildren(inst)
		local isExpanded = expanded[inst] ~= false
		table.insert(rows, instanceRow(inst, depth, conflict, #children > 0, isExpanded))
		if isExpanded then
			for _, child in children do
				visitContent(child, depth + 1, nil)
			end
		end
	end

	local function visitNode(node: SideNode, depth: number)
		local isExpanded = expanded[node.inst] ~= false
		local skeleton = instanceRow(node.inst, depth, nil, true, isExpanded)
		skeleton.color = theme.textMuted
		table.insert(rows, skeleton)
		if not isExpanded then
			return
		end
		for _, graft in node.grafts do
			if graft.content then
				visitContent(graft.content, depth + 1, graft.conflict)
			else
				table.insert(rows, {
					inst = nil,
					label = `{format.escapeRich(graft.conflict.target.Name)} (deleted on {tab})`,
					className = graft.conflict.target.ClassName,
					phantom = true,
					depth = depth + 1,
					color = theme.removed,
					conflict = graft.conflict,
					hasChildren = false,
					isExpanded = false,
				})
			end
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
				TextSize = 10,
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
			phantom = row.phantom,
			hasChildren = row.hasChildren,
			expanded = row.isExpanded,
			selected = inst ~= nil and inst == state.selectedInstance,
			onPath = inst ~= nil and onPath[inst] == true,
			color = row.color,
			tint = row.tint,
			onToggle = if inst and row.hasChildren
				then function()
					expanded[inst :: Instance] = not row.isExpanded
				end
				else nil,
			onActivated = function()
				local conflict = row.conflict
				if row.stageSide and conflict then
					state.stage(conflict, if row.stageSide == "base" then nil else row.stageSide)
					state.selectedInstance = conflict.target
					state.selected = conflict
					return
				end
				if inst then
					state.selectedInstance = inst
				end
				if conflict then
					state.selected = conflict
				end
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
