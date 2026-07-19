--[[
	Full explorer over the preview DOM, in the old rbx-diff-viewer's style:
	expandable 22px rows with carets, dim class names, conflict rows tinted
	and badged by state, #094771 selection. Because staging actually
	reparents/renames targets, the tree follows the staged state live.
	MoveTarget conflicts additionally show phantom entries at each candidate
	parent they are NOT currently under — clicking a phantom stages that side
	("base" = unpick).

	Conflict ancestor chains start expanded; everything else starts collapsed,
	so conflict-scale stays readable on real maps without virtualization.
]]

local Element = require("../miu/Instance")

local format = require("../format")
local state = require("../state")
local theme = require("../theme")

type Conflict = state.Conflict
type SideInfo = state.SideInfo

type Row = {
	inst: Instance?,
	label: string,
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

local function flattenTree(): { Row }
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
		local name = format.escapeRich(inst.Name)
		table.insert(rows, {
			inst = inst,
			label = if conflict
				then `● {name}  <font color="{theme.textDimHex}">{inst.ClassName}</font>`
				else `{name}  <font color="{theme.textDimHex}">{inst.ClassName}</font>`,
			depth = depth,
			color = if conflict then state.statusColor(conflict) else theme.text,
			conflict = conflict,
			hasChildren = #children > 0 or phantoms[inst] ~= nil,
			isExpanded = isExpanded,
			tint = if conflict then state.statusColor(conflict) else nil,
		})
		if isExpanded then
			for _, phantom in phantoms[inst] or {} do
				local sideColor = if phantom.side == "ours"
					then state.OURS_COLOR
					elseif phantom.side == "theirs" then state.THEIRS_COLOR
					else theme.textDim
				table.insert(rows, {
					inst = nil,
					label = `◌ {format.escapeRich(phantom.conflict.target.Name)} ({phantom.side})`,
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

local function ExplorerRow(index: number, row: Row)
	local inst = row.inst
	local isSelected = inst ~= nil and inst == state.selectedInstance

	local background = theme.selection
	local backgroundTransparency = 1
	if isSelected then
		backgroundTransparency = 0
	elseif row.tint then
		background = row.tint
		backgroundTransparency = 0.85
	end

	Element("Frame", {
		id = `row{index}`,
		LayoutOrder = index,
		Size = UDim2.new(1, 0, 0, 22),
		BackgroundColor3 = background,
		BackgroundTransparency = backgroundTransparency,
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4 + row.depth * 12),
		})
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})

		Element("TextButton", {
			id = "caret",
			Text = if not row.hasChildren then "" elseif row.isExpanded then "▾" else "▸",
			Size = UDim2.new(0, 16, 1, 0),
			BackgroundTransparency = 1,
			TextColor3 = theme.textMuted,
			TextSize = 10,
			Font = theme.font,
			events = {
				Activated = function()
					if inst and row.hasChildren then
						expanded[inst] = not row.isExpanded
					end
				end,
			},
		})

		Element("TextButton", {
			id = "name",
			RichText = true,
			Text = row.label,
			Size = UDim2.new(1, -16, 1, 0),
			BackgroundTransparency = 1,
			TextColor3 = row.color,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTruncate = Enum.TextTruncate.AtEnd,
			TextSize = theme.textSize,
			Font = theme.font,
			events = {
				Activated = function()
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
			},
		})
	end)
end

local function Explorer()
	Element("ScrollingFrame", {
		id = "explorer",
		Position = UDim2.new(0, 16, 0, 16),
		Size = UDim2.new(0, 300, 0.5, -24),
		tags = { "bg-none", "flex-col", "gap-sm", "pr-md" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "MERGE EXPLORER",
			Size = UDim2.new(1, 0, 0, 24),
			tags = { "corner-sm", "surface", "vs-border" },
			TextColor3 = theme.textMuted,
			TextSize = theme.headerSize,
			Font = theme.font,
		})

		if state.finished then
			return
		end

		Element("Frame", {
			id = "tree",
			tags = { "auto-size-y", "flex-col", "p-sm", "corner-sm", "surface", "vs-border" },
		}, function()
			for index, row in flattenTree() do
				ExplorerRow(index, row)
			end
		end)
	end)
end

return Explorer
