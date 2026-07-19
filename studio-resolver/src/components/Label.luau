local Element = require("../packages").miu.Instance

local theme = require("../theme")

local function Label(id: string, text: string, color: Color3?)
	Element("TextLabel", {
		id = id,
		Text = text,
		tags = { "text-base", "auto-size-y" },
		TextColor3 = color or theme.text,
		TextXAlignment = Enum.TextXAlignment.Left,
		TextSize = theme.textSize,
		Font = theme.font,
		TextWrapped = true,
	})
end

return Label
