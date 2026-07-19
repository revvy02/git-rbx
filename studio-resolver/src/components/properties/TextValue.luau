local Element = require("../../miu/Instance")

local theme = require("../../theme")

-- Fallback value renderer: mono text, wrapped (CFrames get long).
local function TextValue(id: string, text: string, color: Color3?)
	Element("TextLabel", {
		id = id,
		RichText = true,
		Text = text,
		tags = { "text-base", "auto-size-y" },
		TextColor3 = color or theme.text,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextSize = theme.textSizeSmall,
		Font = theme.font,
		TextWrapped = true,
	})
end

return TextValue
