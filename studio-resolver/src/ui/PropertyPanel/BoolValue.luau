local Element = require("../../packages").miu.Element

local theme = require("../../library/theme")

-- boolean: Studio-style checkbox + dim true/false text.
local function BoolValue(id: string, value: boolean)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("TextLabel", {
			id = "box",
			Text = if value then "✓" else "",
			Size = UDim2.new(0, 12, 0, 12),
			tags = { "corner-sm", "vs-border" },
			BackgroundColor3 = if value then theme.custom else theme.surfaceHigh,
			TextColor3 = theme.bg,
			TextSize = 10,
			Font = theme.font,
		})
		Element("TextLabel", {
			id = "text",
			Text = tostring(value),
			Size = UDim2.new(1, -18, 0, 14),
			tags = { "text", "text-small", "muted" },
		})
	end)
end

return BoolValue
