--[[
	Direct conflict patch, modeled after Rojo's initial-sync visualizer.

	Each side is a literal Base -> Side patch: only changed instances and the
	ancestor paths needed to locate them are shown. Changed nodes carry explicit
	Add / Remove / Reparent / Edit markers; edited nodes expand into two-line
	Base/Side property values. Rust emits the exact operations and serialized
	values, so this remains truthful for properties Studio cannot read.
]]

local packages = require("../packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions
local tags = packages.wind.tags

local Icon = require("./Icon")
local classIcon = require("./classIcon")
local conflicts = require("../ecs/systems/library/conflicts")
local cts = require("../ecs/cts")
local world = require("../ecs/world")
local format = require("../library/format")
local theme = require("../library/theme")

type ImpactOperation = cts.ImpactOperation
type ImpactSide = cts.ImpactSide
type VersionTreeNode = cts.VersionTreeNode
type VersionTrees = cts.VersionTrees

type PatchNode = {
	id: number,
	name: string,
	className: string,
	parent: PatchNode?,
	children: { PatchNode },
	operations: { ImpactOperation },
	order: number,
}

type Row = {
	y: number,
	h: number,
	kind: "node" | "header" | "property",
	depth: number,
	node: PatchNode,
	operation: ImpactOperation?,
	stripe: boolean,
}

local HEADER_HEIGHT = 24
local NODE_HEIGHT = 24
local CHANGE_HEIGHT = 24

local lastEntity: any? = nil
local expandedNodes: { [string]: boolean } = {}

local indexedTrees: VersionTrees? = nil
local treeIndexes: { [string]: { [number]: VersionTreeNode } } = {}
local treeOrder: { [string]: { [number]: number } } = {}

local function indexes(): ({ [string]: { [number]: VersionTreeNode } }, { [string]: { [number]: number } })
	local trees = (world:get(cts.VersionTrees, cts.VersionTrees) :: VersionTrees?) or {}
	if indexedTrees == trees then
		return treeIndexes, treeOrder
	end
	indexedTrees = trees
	treeIndexes = {}
	treeOrder = {}
	for side, records in trees do
		local index: { [number]: VersionTreeNode } = {}
		local order: { [number]: number } = {}
		for position, record in records do
			index[record.id] = record
			order[record.id] = position
		end
		treeIndexes[side] = index
		treeOrder[side] = order
	end
	return treeIndexes, treeOrder
end

local function patchKind(node: PatchNode): string?
	local result: string? = nil
	for _, operation in node.operations do
		if operation.kind == "Delete" or operation.kind == "Add" then
			return operation.kind
		elseif operation.kind == "Reparent" then
			result = "Reparent"
		elseif not result then
			result = "Property"
		end
	end
	return result
end

local function kindColor(kind: string?): Color3
	if kind == "Add" then
		return theme.added
	elseif kind == "Delete" then
		return theme.removed
	elseif kind == "Reparent" then
		return theme.reparented
	elseif kind == "Property" then
		return theme.modified
	end
	return theme.textMuted
end

local function renderDiffIcon(kind: string, color: Color3)
	Element("Frame", {
		id = "diff-icon",
		Size = UDim2.fromOffset(14, 14),
		BackgroundTransparency = 1,
	}, function()
		if kind == "Add" then
			Icon("icon", "lucide:plus", color, UDim2.fromOffset(14, 14))
		elseif kind == "Delete" then
			Icon("icon", "lucide:minus", color, UDim2.fromOffset(14, 14))
		elseif kind == "Reparent" then
			Icon("icon", "lucide:move-right", color, UDim2.fromOffset(14, 14))
		else
			Icon("icon", "lucide:pencil", color, UDim2.fromOffset(14, 14))
		end
	end)
end

local function buildPatch(entity: any, side: string): ({ PatchNode }, ImpactSide)
	local impact = conflicts.impact(entity, side)
	local bySide, orderBySide = indexes()
	local nodes: { [number]: PatchNode } = {}
	local roots: { PatchNode } = {}

	local function ensure(id: number, version: string): PatchNode?
		if nodes[id] then
			return nodes[id]
		end
		local records = bySide[version] or {}
		local record = records[id] or (bySide.base and bySide.base[id])
		if not record then
			return nil
		end
		local node: PatchNode = {
			id = id,
			name = record.name,
			className = record.className,
			children = {},
			operations = {},
			order = (orderBySide[version] and orderBySide[version][id])
				or (orderBySide.base and orderBySide.base[id])
				or math.huge,
		}
		nodes[id] = node
		if record.parent then
			local parent = ensure(record.parent, version)
			node.parent = parent
			table.insert(if parent then parent.children else roots, node)
		else
			table.insert(roots, node)
		end
		return node
	end

	for _, operation in impact.operations do
		if operation.nodeId then
			local version = if operation.kind == "Delete" then "base" else side
			local node = ensure(operation.nodeId, version)
			if node then
				table.insert(node.operations, operation)
			end
		end
	end

	local function sort(nodesToSort: { PatchNode })
		table.sort(nodesToSort, function(a, b)
			return if a.order == b.order then a.name < b.name else a.order < b.order
		end)
		for _, node in nodesToSort do
			sort(node.children)
		end
	end
	sort(roots)
	return roots, impact
end

local function expansionKey(side: string, node: PatchNode): string
	return `{side}:{node.id}`
end

local function buildRows(roots: { PatchNode }, side: string): { Row }
	local rows: { Row } = {}
	local y = 0
	local stripe = false
	local function push(
		kind: "node" | "header" | "property",
		h: number,
		depth: number,
		node: PatchNode,
		operation: ImpactOperation?
	)
		stripe = not stripe
		table.insert(rows, {
			y = y,
			h = h,
			kind = kind,
			depth = depth,
			node = node,
			operation = operation,
			stripe = stripe,
		})
		y += h
	end

	local function visit(node: PatchNode, depth: number)
		push("node", NODE_HEIGHT, depth, node, nil)
		local key = expansionKey(side, node)
		local expanded = expandedNodes[key] == true
		if expanded then
			local hasChanges = false
			for _, operation in node.operations do
				if operation.kind == "Property" or operation.kind == "Reparent" then
					hasChanges = true
				end
			end
			if hasChanges then
				push("header", CHANGE_HEIGHT, depth + 1, node, nil)
				for _, operation in node.operations do
					if operation.kind == "Property" or operation.kind == "Reparent" then
						push("property", CHANGE_HEIGHT, depth + 1, node, operation)
					end
				end
			end
		end
		for _, child in node.children do
			visit(child, depth + 1)
		end
	end
	for _, root in roots do
		visit(root, 0)
	end
	return rows
end

local rowDimensions = Dimensions.getter(function(row: Row)
	return {
		size = UDim2.new(1, 0, 0, row.h),
		position = UDim2.fromOffset(0, row.y),
	}
end)

local function renderNode(row: Row, side: string)
	local node = row.node
	local kind = patchKind(node)
	local color = kindColor(kind)
	local selectedId = world:get(cts.SelectedExplorerId, cts.SelectedExplorerId)
	local details = 0
	local subtreeCount: number? = nil
	for _, operation in node.operations do
		if operation.kind == "Property" or operation.kind == "Reparent" then
			details += 1
		elseif operation.kind == "Add" or operation.kind == "Delete" then
			subtreeCount = operation.instanceCount
		end
	end
	local suffix = if subtreeCount and subtreeCount > 1
		then `{subtreeCount} nodes`
		elseif details > 0 then `{details} {if details == 1 then "change" else "changes"}`
		else ""
	local hasDetails = details > 0
	local key = expansionKey(side, node)

	Element("Frame", {
		id = "node",
		Size = UDim2.fromScale(1, 1),
		tags = tags("row", row.stripe and "stripe", selectedId == node.id and "selected"),
	}, function()
		if kind then
			Element("Frame", {
				id = "diff",
				Position = UDim2.fromOffset(5, 5),
				Size = UDim2.fromOffset(14, 14),
				BackgroundTransparency = 1,
			}, function()
				renderDiffIcon(kind :: string, color)
			end)
		end

		for guide = 1, row.depth do
			Element("Frame", {
				id = `guide-{guide}`,
				Position = UDim2.fromOffset(25 + (guide - 1) * 12, 0),
				Size = UDim2.new(0, 1, 1, 0),
				BackgroundColor3 = theme.border,
				BackgroundTransparency = 0.35,
				BorderSizePixel = 0,
			})
		end

		local disclosureX = 22 + row.depth * 12
		local iconX = disclosureX + 12
		Element("TextButton", {
			id = "disclosure",
			Text = "",
			Position = UDim2.fromOffset(disclosureX, 0),
			Size = UDim2.fromOffset(12, NODE_HEIGHT),
			AutoButtonColor = false,
			tags = { "bg-none" },
			events = {
				Activated = function()
					if hasDetails then
						expandedNodes[key] = not expandedNodes[key]
					end
				end,
			},
		}, function()
			if hasDetails then
				Icon(
					"icon",
					if expandedNodes[key] then "bxs:down-arrow" else "bxs:right-arrow",
					theme.textMuted,
					UDim2.fromOffset(8, 8)
				)
			end
		end)

		local icon = classIcon(node.className)
		Element("ImageLabel", {
			id = "class-icon",
			Image = icon.Image,
			ImageRectOffset = icon.ImageRectOffset,
			ImageRectSize = icon.ImageRectSize,
			ImageColor3 = color,
			Position = UDim2.fromOffset(iconX, 4),
			Size = UDim2.fromOffset(16, 16),
			BackgroundTransparency = 1,
		})

		Element("TextButton", {
			id = "main",
			Text = node.name,
			Position = UDim2.fromOffset(iconX + 19, 0),
			Size = UDim2.new(1, -99 - iconX, 1, 0),
			tags = { "text", "button", "bg-none" },
			TextColor3 = color,
			TextTruncate = Enum.TextTruncate.AtEnd,
			events = {
				Activated = function()
					conflicts.selectExplorerId(node.id)
				end,
			},
		})
		if suffix ~= "" then
			Element("TextLabel", {
				id = "count",
				Text = suffix,
				Position = UDim2.new(1, -78, 0, 0),
				Size = UDim2.fromOffset(74, NODE_HEIGHT),
				tags = { "text", "text-small", "muted" },
				TextXAlignment = Enum.TextXAlignment.Right,
			})
		end
	end)
end

local function changeColumns(row: Row, callback: () -> ())
	local indent = 41 + row.depth * 12
	Element("Frame", {
		id = "columns",
		Position = UDim2.fromOffset(indent, 0),
		Size = UDim2.new(1, -indent - 4, 1, 0),
		BackgroundTransparency = 1,
	}, function()
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			SortOrder = Enum.SortOrder.LayoutOrder,
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})
		callback()
	end)
end

local function renderCell(id: string, text: string, width: number, color: Color3, order: number)
	Element("TextLabel", {
		id = id,
		RichText = true,
		Text = text,
		Size = UDim2.fromScale(width, 1),
		LayoutOrder = order,
		tags = { "text", "text-small", "bg-none" },
		TextColor3 = color,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

local function renderChangeHeader(row: Row, side: string)
	Element("Frame", {
		id = "change-header",
		Size = UDim2.fromScale(1, 1),
		tags = tags("row", row.stripe and "stripe"),
	}, function()
		changeColumns(row, function()
			renderCell("property", "PROPERTY", 0.3, theme.text, 1)
			renderCell("base", "BASE", 0.35, theme.text, 2)
			renderCell("incoming", string.upper(side), 0.35, theme.text, 3)
		end)
	end)
end

local function renderProperty(row: Row)
	local operation = row.operation :: ImpactOperation
	local property = operation.property or (if operation.kind == "Reparent" then "Parent" else "Property")
	local before = if operation.kind == "Reparent"
		then format.escapeRich(operation.path)
		else format.patchValue(operation.before)
	local after = if operation.kind == "Reparent"
		then format.escapeRich(operation.destination or "?")
		else format.patchValue(operation.after)
	Element("TextButton", {
		id = "change",
		Text = "",
		Size = UDim2.fromScale(1, 1),
		AutoButtonColor = false,
		tags = tags("row", row.stripe and "stripe"),
		events = {
			Activated = function()
				if operation.nodeId then
					conflicts.selectExplorerId(operation.nodeId)
				end
			end,
		},
	}, function()
		changeColumns(row, function()
			renderCell("property", format.escapeRich(property), 0.3, theme.propName, 1)
			renderCell("base", before, 0.35, theme.removed, 2)
			renderCell("incoming", after, 0.35, theme.added, 3)
		end)
	end)
end

local function resolutionOf(entity: any): string?
	return if world:get(entity, cts.Members)
		then conflicts.groupResolution(entity)
		else world:get(entity, cts.Resolution) :: string?
end

local function ImpactPanel()
	local entity = world:get(cts.SelectedConflict, cts.SelectedConflict)
	local resolution = entity and resolutionOf(entity) or nil
	local side = if resolution == "theirs" then "theirs" else "ours"
	if entity ~= lastEntity then
		lastEntity = entity
		table.clear(expandedNodes)
	end

	local roots: { PatchNode } = {}
	local impact: ImpactSide? = nil
	if entity then
		roots, impact = buildPatch(entity, side)
	end
	local rows = buildRows(roots, side)

	Element("Frame", {
		id = "impact",
		Size = UDim2.fromScale(1, 1),
		tags = { "surface", "vs-border", "flex-col" },
	}, function()
		Element("TextLabel", {
			id = "title",
			Text = if impact
				then `DIRECT DIFF — {impact.instanceCount} CHANGED · {impact.propertyCount} {if impact.propertyCount
						== 1
					then "PROPERTY"
					else "PROPERTIES"}`
				else "DIRECT DIFF",
			Size = UDim2.new(1, 0, 0, HEADER_HEIGHT),
			tags = { "header-band" },
		})

		if not entity or #rows == 0 then
			Element("TextLabel", {
				id = "empty",
				Text = "No direct changes on this side",
				Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
				tags = { "text", "muted" },
				TextXAlignment = Enum.TextXAlignment.Center,
				TextYAlignment = Enum.TextYAlignment.Center,
			})
			return
		end

		VirtualList({
			id = "rows",
			data = rows,
			dimensions = rowDimensions,
			direction = "y",
			Size = UDim2.new(1, 0, 1, -HEADER_HEIGHT),
			tags = { "bg-none" },
		}, function(row: Row)
			if row.kind == "node" then
				renderNode(row, side)
			elseif row.kind == "header" then
				renderChangeHeader(row, side)
			else
				renderProperty(row)
			end
		end)
	end)
end

return ImpactPanel
