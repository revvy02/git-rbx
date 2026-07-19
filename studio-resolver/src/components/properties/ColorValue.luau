local Element = require("../../packages").miu.Instance

local theme = require("../../theme")

-- Color3: swatch + friendly RGB ints (the precise float triple stays in the
-- comparison card via format.formatValue).
local function ColorValue(id: string, color: Color3)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("Frame", {
			id = "swatch",
			Size = UDim2.new(0, 12, 0, 12),
			tags = { "corner-sm", "vs-border" },
			BackgroundColor3 = color,
		})
		Element("TextLabel", {
			id = "text",
			Text = `{math.round(color.R * 255)}, {math.round(color.G * 255)}, {math.round(color.B * 255)}`,
			Size = UDim2.new(1, -18, 0, 14),
			BackgroundTransparency = 1,
			TextColor3 = theme.text,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
		})
	end)
end

return ColorValue
