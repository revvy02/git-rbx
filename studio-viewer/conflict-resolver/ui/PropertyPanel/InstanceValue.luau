local Element = require("@shared/packages").miu.Element

local classIcon = require("@shared/ui/classIcon")
local format = require("@shared/library/format")

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
			tags = { "text", "text-small" },
			TextTruncate = Enum.TextTruncate.AtEnd,
		})
	end)
end

return InstanceValue
