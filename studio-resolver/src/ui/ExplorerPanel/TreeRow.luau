--[[
	Presentational tree row, after game-prototype's miu-tree/TreeRow: same
	props contract (depth, label, hasChildren, expanded, selected, onPath,
	onActivated, onToggle) adapted to the resolver's stack — plain miu events
	instead of the miu-flow Activated helper — plus resolver extras: a class
	icon and a phantom dim.

	Background states are wind tags: selected > on-path > stripe, precedence
	via rule priority in theme.rules().
]]

local packages = require("../../packages")
local Element = packages.miu.Element
local tags = packages.wind.tags

local classIcon = require("../classIcon")
local theme = require("../../library/theme")

export type TreeRowProps = {
	id: string,
	layoutOrder: number?,
	depth: number,
	label: string, -- RichText
	className: string?, -- class icon; omit for placeholder rows
	phantom: boolean?,
	hasChildren: boolean?,
	expanded: boolean?,
	selected: boolean?,
	onPath: boolean?, -- ancestor of the selection
	color: Color3?, -- label color (conflict status / side)
	stripe: boolean?, -- zebra shading for alternating rows
	onActivated: (() -> ())?,
	onToggle: (() -> ())?,
}

local INDENT = 12

local function TreeRow(props: TreeRowProps)
	Element("Frame", {
		id = props.id,
		LayoutOrder = props.layoutOrder,
		Size = UDim2.new(1, 0, 0, 22),
		tags = tags("row", props.stripe and "stripe", props.onPath and "on-path", props.selected and "selected"),
	}, function()
		Element("UIPadding", {
			id = "pad",
			PaddingLeft = UDim.new(0, 4 + props.depth * INDENT),
		})
		Element("UIListLayout", {
			id = "layout",
			FillDirection = Enum.FillDirection.Horizontal,
			VerticalAlignment = Enum.VerticalAlignment.Center,
		})

		Element("TextButton", {
			id = "expand",
			Text = if not props.hasChildren then "" elseif props.expanded then "v " else "> ",
			Size = UDim2.new(0, 16, 1, 0),
			tags = { "bg-none", "button", "muted" },
			events = {
				Activated = function()
					if props.onToggle then
						props.onToggle()
					end
				end,
			},
		})

		local iconWidth = 0
		if props.className then
			iconWidth = 16
			local icon = classIcon(props.className :: string)
			Element("ImageLabel", {
				id = "icon",
				Image = icon.Image,
				ImageRectOffset = icon.ImageRectOffset,
				ImageRectSize = icon.ImageRectSize,
				ImageTransparency = if props.phantom then 0.5 else 0,
				Size = UDim2.new(0, 16, 0, 16),
				BackgroundTransparency = 1,
			})
		end

		Element("TextButton", {
			id = "label",
			RichText = true,
			Text = ` {props.label}`,
			Size = UDim2.new(1, -16 - iconWidth, 1, 0),
			tags = { "text" },
			TextColor3 = props.color or theme.text,
			TextTruncate = Enum.TextTruncate.AtEnd,
			events = {
				Activated = function()
					if props.onActivated then
						props.onActivated()
					end
				end,
			},
		})
	end)
end

return TreeRow
