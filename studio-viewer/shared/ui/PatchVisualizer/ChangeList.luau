--[[
	The expanded change table under a node, after Rojo's ChangeList: a bold
	header row then one row per change, Property / Current / Incoming at
	30 / 35 / 35 percent, indented under the node. Rows are rendered one at
	a time by the visualizer's virtual list, so this module exposes the two
	row renderers rather than a whole table.
]]

local Element = require("@shared/packages").miu.Element
local tags = require("@shared/packages").wind.tags

local DisplayValue = require("@shared/ui/PatchVisualizer/DisplayValue")
local format = require("@shared/library/format")
local theme = require("@shared/library/theme")
local types = require("@shared/ui/PatchVisualizer/types")

type Row = types.Row

local ChangeList = {}

export type Headers = { string }

local function columns(row: Row, callback: () -> ())
	-- Rojo indents the table by the node's own indent; the node row puts
	-- its class icon at indent + 2, so the table starts at the icon.
	local indent = (row.node.depth - 1) * 12 + 25
	Element("Frame", {
		id = "columns",
		Position = UDim2.fromOffset(indent, 0),
		Size = UDim2.new(1, -indent - 6, 1, 0),
		BackgroundTransparency = 1,
	}, function()
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			SortOrder = Enum.SortOrder.LayoutOrder,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			Padding = UDim.new(0, 4),
		})
		callback()
	end)
end

local function cell(id: string, width: number, order: number, callback: () -> ())
	Element("Frame", {
		id = id,
		Size = UDim2.fromScale(width, 1),
		LayoutOrder = order,
		BackgroundTransparency = 1,
	}, callback)
end

local function headerCell(id: string, text: string, width: number, order: number)
	Element("TextLabel", {
		id = id,
		Text = text,
		Size = UDim2.fromScale(width, 1),
		LayoutOrder = order,
		tags = { "text", "text-small" },
		TextColor3 = theme.text,
		FontFace = theme.fontFaceBold,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

function ChangeList.renderHeader(row: Row, headers: Headers)
	Element("Frame", {
		id = "change-header",
		Size = UDim2.fromScale(1, 1),
		tags = tags("row", row.index % 2 == 0 and "stripe"),
	}, function()
		columns(row, function()
			headerCell("property", headers[1], 0.3, 1)
			headerCell("current", headers[2], 0.35, 2)
			headerCell("incoming", headers[3], 0.35, 3)
		end)
	end)
end

function ChangeList.renderChange(row: Row, onActivate: (() -> ())?)
	local change = row.change :: types.ChangeRow
	local warning = change.warning == true
	local valueColor = if warning then theme.warning else theme.textMuted
	Element("TextButton", {
		id = "change",
		Text = "",
		Size = UDim2.fromScale(1, 1),
		AutoButtonColor = false,
		tags = tags("row", row.index % 2 == 0 and "stripe"),
		events = {
			Activated = function()
				if onActivate then
					onActivate()
				end
			end,
		},
	}, function()
		columns(row, function()
			Element("TextLabel", {
				id = "property",
				RichText = true,
				Text = (if warning then "⚠ " else "") .. format.escapeRich(change.property),
				Size = UDim2.fromScale(0.3, 1),
				LayoutOrder = 1,
				tags = { "text", "text-small" },
				TextColor3 = if warning then theme.warning else theme.text,
				TextTruncate = Enum.TextTruncate.AtEnd,
			})
			cell("current", 0.35, 2, function()
				DisplayValue("value", change.current, change.currentText, valueColor)
			end)
			cell("incoming", 0.35, 3, function()
				DisplayValue("value", change.incoming, change.incomingText, valueColor)
			end)
		end)
	end)
end

return ChangeList
