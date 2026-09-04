--[[
	PatchVisualizer — Rojo's sync-changes tree for git-rbx's five change
	kinds (Add / Remove / Edit / Reparent / Pivot).

	Feed it a neutral Patch (see types.luau) and it renders the sparse tree:
	changed instances plus the ancestors needed to reach them, siblings
	alphabetical, one marker per node, edit counts on the right, and a
	Property / Current / Incoming table under any node you click. Double
	click hands the node to `onSelect`.

	Expansion state is keyed by `stateKey` so a caller showing a different
	patch (another conflict, another side) starts collapsed, as Rojo's rows
	do when their content changes.
]]

local packages = require("../packages")
local Element = packages.miu.Element
local VirtualList = packages.virtualList.VirtualList
local Dimensions = packages.virtualList.Dimensions

local ChangeList = require("@self/ChangeList")
local DomLabel = require("@self/DomLabel")
local patchTree = require("@self/patchTree")
local types = require("@self/types")

export type Patch = types.Patch
export type PatchNodeInput = types.PatchNodeInput
export type ChangeRow = types.ChangeRow
export type PatchNode = types.PatchNode
type Id = types.Id

export type Props = {
	patch: Patch,
	stateKey: any, -- identity of what is being shown; changing it collapses everything
	headers: { string }?, -- default { "Property", "Current", "Incoming" }
	selectedId: types.Id?,
	onSelect: ((PatchNode) -> ())?,
	emptyText: string?,
	Size: UDim2?,
	Position: UDim2?,
}

local ROW_HEIGHT = 24
local DEFAULT_HEADERS = { "Property", "Current", "Incoming" }

local expandedFor: any = nil
local expanded: { [Id]: boolean } = {}

local rowDimensions = Dimensions.getter(function(row: types.Row)
	return {
		size = UDim2.new(1, 0, 0, row.h),
		position = UDim2.fromOffset(0, row.y),
	}
end)

local function PatchVisualizer(props: Props)
	if props.stateKey ~= expandedFor then
		expandedFor = props.stateKey
		table.clear(expanded)
	end

	local roots = patchTree.build(props.patch)
	local rows = patchTree.flatten(roots, ROW_HEIGHT, function(node)
		return expanded[node.id] == true
	end)
	local headers = props.headers or DEFAULT_HEADERS

	Element("Frame", {
		id = "patch-visualizer",
		Size = props.Size or UDim2.fromScale(1, 1),
		Position = props.Position,
		BackgroundTransparency = 1,
	}, function()
		if #rows == 0 then
			Element("TextLabel", {
				id = "empty",
				Text = props.emptyText or "No changes",
				Size = UDim2.fromScale(1, 1),
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
			Size = UDim2.fromScale(1, 1),
			tags = { "bg-none" },
		}, function(row: types.Row)
			local node = row.node
			local select = if props.onSelect
				then function()
					(props.onSelect :: any)(node)
				end
				else nil
			if row.kind == "node" then
				DomLabel.render({
					row = row,
					rowHeight = ROW_HEIGHT,
					expanded = expanded[node.id] == true,
					selected = props.selectedId ~= nil and props.selectedId == node.id,
					onToggle = function()
						expanded[node.id] = not expanded[node.id]
					end,
					onSelect = select,
				})
			elseif row.kind == "header" then
				ChangeList.renderHeader(row, headers)
			else
				ChangeList.renderChange(row, select)
			end
		end)
	end)
end

return PatchVisualizer
