local Element = require("@shared/packages").miu.Element

local theme = require("@shared/library/theme")

-- Fallback value renderer: mono text, single line. Truncates rather than
-- wraps — rows live in fixed-height virtual list slots (full values return
-- with the comparison card).
local function TextValue(id: string, text: string, color: Color3?)
	Element("TextLabel", {
		id = id,
		RichText = true,
		Text = text,
		Size = UDim2.new(1, 0, 0, 14),
		tags = { "text", "text-small" },
		TextColor3 = color or theme.text,
		TextTruncate = Enum.TextTruncate.AtEnd,
	})
end

return TextValue
