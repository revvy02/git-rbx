--[[
	Presentational tree row, after game-prototype's miu-tree/TreeRow: same
	props contract (depth, label, hasChildren, expanded, selected, onPath,
	onActivated, onToggle) adapted to the resolver's stack — plain miu events
	instead of the miu-flow Activated helper, theme constants instead of wind
	state classes — plus resolver extras: a class icon, a phantom dim, and a
	rest-state background tint for conflict rows.

	State waterfall for the background: selected > on-path > tint.
]]

local Element = require("../miu/Instance")

local classIcon = require("./classIcon")
local theme = require("../theme")

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
	tint: Color3?, -- rest background tint (conflict state)
	stripe: boolean?, -- zebra shading for alternating rows
	onActivated: (() -> ())?,
	onToggle: (() -> ())?,
}

local INDENT = 12

local function TreeRow(props: TreeRowProps)
	local background = theme.selection
	local backgroundTransparency = 1
	if props.selected then
		backgroundTransparency = 0
	elseif props.onPath then
		backgroundTransparency = 0.85
	elseif props.tint then
		background = props.tint :: Color3
		backgroundTransparency = 0.85
	elseif props.stripe then
		background = theme.bg
		backgroundTransparency = 0.5
	end

	Element("Frame", {
		id = props.id,
		LayoutOrder = props.layoutOrder,
		Size = UDim2.new(1, 0, 0, 22),
		BackgroundColor3 = background,
		BackgroundTransparency = backgroundTransparency,
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
			BackgroundTransparency = 1,
			TextColor3 = theme.textMuted,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
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
			BackgroundTransparency = 1,
			TextColor3 = props.color or theme.text,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTruncate = Enum.TextTruncate.AtEnd,
			TextSize = theme.textSize,
			Font = theme.font,
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
