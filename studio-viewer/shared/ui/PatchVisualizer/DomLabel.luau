--[[
	One instance row, after Rojo's DomLabel: line guides per depth with a
	connector, a kind marker at the far left, the class icon, the name in
	the kind's color (bold when changed, plain for ancestors), and the number
	of changes right-aligned. Single click toggles the change list, double
	click selects the instance.
]]

local Element = require("@shared/packages").miu.Element
local tags = require("@shared/packages").wind.tags

local Icon = require("@shared/ui/Icon")
local classIcon = require("@shared/ui/classIcon")
local format = require("@shared/library/format")
local theme = require("@shared/library/theme")
local types = require("@shared/ui/PatchVisualizer/types")

type PatchNode = types.PatchNode
type Row = types.Row
type Id = types.Id

local ICON_BY_KIND = {
	Add = "lucide:plus",
	Remove = "lucide:minus",
	Edit = "lucide:pencil",
	Reparent = "lucide:move-right",
	Pivot = "lucide:rotate-3d",
}

local DomLabel = {}

function DomLabel.colorOf(kind: types.NodeKind?, warning: boolean?): Color3
	if warning then
		return theme.warning
	elseif kind == "Add" then
		return theme.added
	elseif kind == "Remove" then
		return theme.removed
	elseif kind == "Edit" then
		return theme.modified
	elseif kind == "Reparent" then
		return theme.reparented
	elseif kind == "Pivot" then
		return theme.pivoted
	end
	return theme.text
end

export type Props = {
	row: Row,
	rowHeight: number,
	expanded: boolean,
	selected: boolean,
	onToggle: () -> (),
	onSelect: (() -> ())?,
}

local lastDoubleClick: { [Id]: number } = {}

function DomLabel.render(props: Props)
	local row = props.row
	local node = row.node
	local depth = node.depth
	local color = DomLabel.colorOf(node.kind, node.warning)
	local indent = (depth - 1) * 12 + 15
	local hasChanges = #node.rows > 0
	local half = props.rowHeight / 2

	Element("Frame", {
		id = "node",
		Size = UDim2.fromScale(1, 1),
		ClipsDescendants = true,
		tags = tags("row", row.index % 2 == 0 and "stripe", props.selected and "selected"),
	}, function()
		Element("UIPadding", {
			id = "padding",
			PaddingLeft = UDim.new(0, 10),
			PaddingRight = UDim.new(0, 10),
		})

		-- Line guides: one per ancestor depth still in progress; the final
		-- child's own guide stops halfway to meet the connector.
		for i = 2, depth do
			if row.depthsComplete[i] then
				continue
			end
			local short = node.isFinalChild and i == depth
			Element("Frame", {
				id = `guide-{i}`,
				Size = if short
					then UDim2.fromOffset(2, half + 3)
					else UDim2.new(0, 2, 1, if row.isFinalElement then -(half - 3) else 2),
				Position = UDim2.fromOffset((12 * (i - 1)) + 6, -1),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.border,
				BackgroundTransparency = 0,
			})
		end
		if depth ~= 1 then
			Element("Frame", {
				id = "connector",
				Size = UDim2.fromOffset(8, 2),
				Position = UDim2.fromOffset(2 + (12 * depth), half),
				AnchorPoint = Vector2.new(1, 0),
				BorderSizePixel = 0,
				BackgroundColor3 = theme.border,
				BackgroundTransparency = 0,
			})
		end

		Element("TextButton", {
			id = "button",
			Text = "",
			Size = UDim2.fromScale(1, 1),
			AutoButtonColor = false,
			tags = { "bg-none" },
			events = {
				Activated = function(_input: InputObject, clickCount: number)
					if clickCount == 1 then
						lastDoubleClick[node.id] = os.clock()
						if props.onSelect then
							props.onSelect()
						end
					elseif clickCount == 0 and hasChanges then
						task.delay(0.25, function()
							if os.clock() - (lastDoubleClick[node.id] or 0) <= 0.3 then
								return
							end
							props.onToggle()
						end)
					end
				end,
			},
		})

		if node.kind then
			Element("Frame", {
				id = "diff-icon",
				Size = UDim2.fromOffset(14, 14),
				Position = UDim2.fromScale(0, 0.5),
				AnchorPoint = Vector2.new(0, 0.5),
				BackgroundTransparency = 1,
			}, function()
				Icon("icon", ICON_BY_KIND[node.kind], color, UDim2.fromOffset(14, 14))
			end)
		end

		local icon = classIcon(node.className)
		Element("ImageLabel", {
			id = "class-icon",
			Image = icon.Image,
			ImageRectOffset = icon.ImageRectOffset,
			ImageRectSize = icon.ImageRectSize,
			ImageColor3 = color,
			Size = UDim2.fromOffset(16, 16),
			Position = UDim2.new(0, indent + 2, 0.5, 0),
			AnchorPoint = Vector2.new(0, 0.5),
			BackgroundTransparency = 1,
		})

		Element("TextLabel", {
			id = "name",
			RichText = true,
			Text = (if node.warning then "⚠ " else "") .. format.escapeRich(node.name),
			Position = UDim2.fromOffset(indent + 22, 0),
			Size = UDim2.new(1, -indent - 22 - 50, 1, 0),
			tags = { "text" },
			TextColor3 = color,
			FontFace = if node.kind then theme.fontFaceBold else theme.fontFace,
			TextTruncate = Enum.TextTruncate.AtEnd,
		})

		-- Rojo shows the edit count for edited and added nodes and nothing
		-- for removals; a removed subtree is one row with no children.
		local count = if node.kind ~= "Remove" and hasChanges then #node.rows else nil
		if count then
			Element("TextLabel", {
				id = "count",
				Text = tostring(count),
				Position = UDim2.fromScale(1, 0),
				AnchorPoint = Vector2.new(1, 0),
				Size = UDim2.new(0, 48, 1, 0),
				tags = { "text", "muted" },
				TextXAlignment = Enum.TextXAlignment.Right,
			})
		end
	end)
end

return DomLabel
