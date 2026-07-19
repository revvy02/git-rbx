local Element = require("../../packages").miu.Instance

local classIcon = require("../classIcon")
local format = require("../../format")
local theme = require("../../theme")

-- Instance reference: class icon + name (e.g. PrimaryPart).
local function InstanceValue(id: string, inst: Instance)
	local icon = classIcon(inst.ClassName)
	Element("Frame", {
		id = id,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "bg-none", "flex-row", "gap-sm" },
	}, function()
		Element("ImageLabel", {
			id = "icon",
			Image = icon.Image,
			ImageRectOffset = icon.ImageRectOffset,
			ImageRectSize = icon.ImageRectSize,
			Size = UDim2.new(0, 14, 0, 14),
			BackgroundTransparency = 1,
		})
		Element("TextLabel", {
			id = "text",
			RichText = true,
			Text = format.escapeRich(inst.Name),
			Size = UDim2.new(1, -20, 0, 14),
			BackgroundTransparency = 1,
			TextColor3 = theme.text,
			TextXAlignment = Enum.TextXAlignment.Left,
			TextTruncate = Enum.TextTruncate.AtEnd,
			TextSize = theme.textSizeSmall,
			Font = theme.font,
		})
	end)
end

return InstanceValue
