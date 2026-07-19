--[[
	Merge explorer: a conflict-scoped tree (targets + their ancestor chains,
	badged by state), not a DOM clone. A MoveTarget appears at its CURRENT
	previewed position solid, with phantom entries at the other candidate
	parents — clicking a phantom stages that side. Below it, the comparison
	card for the selected conflict.
]]

local Element = require("../miu/Instance")

local state = require("../state")

local ComparisonCard = require("./ComparisonCard")

type Conflict = state.Conflict
type SideInfo = state.SideInfo

type ExplorerRow = {
	label: string,
	depth: number,
	color: Color3,
	conflict: Conflict?, -- clicking selects this conflict
	stageSide: string?, -- phantom rows: clicking stages this side ("base" = unpick)
}

type TreeNode = {
	inst: Instance,
	children: { TreeNode },
	conflicts: { Conflict },
	phantoms: { { conflict: Conflict, side: string } },
}

-- Conflict-scoped tree over the LIVE preview DOM: because staging actually
-- reparents/renames targets, the solid tree follows the staged state for free.
local function flattenTree(): { ExplorerRow }
	local nodes: { [Instance]: TreeNode } = {}
	local roots: { TreeNode } = {}

	local function nodeFor(inst: Instance): TreeNode
		local existing = nodes[inst]
		if existing then
			return existing
		end
		local node: TreeNode = { inst = inst, children = {}, conflicts = {}, phantoms = {} }
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
		local node = nodeFor(conflict.target)
		table.insert(node.conflicts, conflict)
		if conflict.kind == "MoveTarget" then
			-- Phantoms at every candidate parent the target is NOT currently
			-- under (including base when staged away from it).
			local candidates: { [string]: Instance? } = {
				ours = (conflict.ours :: SideInfo).destination,
				theirs = (conflict.theirs :: SideInfo).destination,
				base = conflict.baseParent,
			}
			for side, parent in candidates :: { [string]: Instance } do
				if parent ~= conflict.target.Parent then
					table.insert(nodeFor(parent).phantoms, { conflict = conflict, side = side })
				end
			end
		end
	end

	local rows: { ExplorerRow } = {}
	local function visit(node: TreeNode, depth: number)
		local conflict = node.conflicts[1]
		table.insert(rows, {
			label = if conflict then `● {node.inst.Name}` else node.inst.Name,
			depth = depth,
			color = if conflict then state.statusColor(conflict) else Color3.fromRGB(150, 150, 158),
			conflict = conflict,
		})
		for _, phantom in node.phantoms do
			local sideColor = if phantom.side == "ours"
				then state.OURS_COLOR
				elseif phantom.side == "theirs" then state.THEIRS_COLOR
				else Color3.fromRGB(120, 120, 128)
			table.insert(rows, {
				label = `◌ {phantom.conflict.target.Name} ({phantom.side})`,
				depth = depth + 1,
				color = sideColor,
				conflict = phantom.conflict,
				stageSide = phantom.side,
			})
		end
		table.sort(node.children, function(a: TreeNode, b: TreeNode)
			return a.inst.Name < b.inst.Name
		end)
		for _, child in node.children do
			visit(child, depth + 1)
		end
	end
	for _, root in roots do
		visit(root, 0)
	end
	return rows
end

local function Explorer()
	Element("ScrollingFrame", {
		id = "explorer",
		Position = UDim2.new(0, 16, 0, 16),
		Size = UDim2.new(0, 300, 1, -32),
		tags = { "bg-none", "flex-col", "gap-sm", "pr-md" },
		ScrollingDirection = Enum.ScrollingDirection.Y,
		AutomaticCanvasSize = Enum.AutomaticSize.Y,
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = "rbx-diff: merge explorer",
			Size = UDim2.new(1, 0, 0, 26),
			tags = { "corner-md" },
			BackgroundColor3 = Color3.fromRGB(28, 28, 32),
			TextColor3 = Color3.fromRGB(230, 230, 230),
			TextSize = 14,
			Font = Enum.Font.GothamMedium,
		})

		if state.finished then
			return
		end

		Element("Frame", {
			id = "tree",
			tags = { "auto-size-y", "flex-col", "p-md", "corner-md" },
			BackgroundColor3 = Color3.fromRGB(40, 40, 46),
		}, function()
			for index, row in flattenTree() do
				local isSelected = row.conflict ~= nil and row.conflict == state.selected and row.stageSide == nil
				Element("TextButton", {
					id = `row{index}`,
					Text = "",
					LayoutOrder = index,
					Size = UDim2.new(1, 0, 0, 18),
					tags = { "corner-sm" },
					BackgroundColor3 = Color3.fromRGB(55, 55, 62),
					BackgroundTransparency = if isSelected then 0 else 1,
					AutoButtonColor = row.conflict ~= nil,
					events = {
						Activated = function()
							local conflict = row.conflict
							if not conflict then
								return
							end
							if row.stageSide then
								state.stage(conflict, if row.stageSide == "base" then nil else row.stageSide)
							end
							state.selected = conflict
						end,
					},
				}, function()
					Element("UIPadding", {
						id = "pad",
						PaddingLeft = UDim.new(0, 4 + row.depth * 12),
					})
					Element("TextLabel", {
						id = "label",
						Text = row.label,
						Size = UDim2.new(1, 0, 1, 0),
						BackgroundTransparency = 1,
						TextColor3 = row.color,
						TextXAlignment = Enum.TextXAlignment.Left,
						TextTruncate = Enum.TextTruncate.AtEnd,
						TextSize = 13,
						Font = Enum.Font.Gotham,
					})
				end)
			end
		end)

		ComparisonCard()
	end)
end

return Explorer
